import {
  moveSingleFileIntoSharedFolder,
  moveIntoSharedFolder,
  moveOutOfSharedFolder,
  trustedFingerprintsStore,
  UploadIntoSharedFolderAborted
} from '!/shares'
import type {
  MoveCascadePreview,
  MoveIntoSharedFolderOptions,
  UnknownMemberPrompt
} from '!/shares'

import type { AppFile, KeyPair } from 'types'

/**
 * A directory is a multi-key (shared) move target when the caller doesn't own
 * it, or owns it but has shared it (its member list has been signed). The same
 * predicate the upload funnel uses in `LayoutFileBrowserInner.vue`. The account
 * root is never a target — there is no `AppFile` for it.
 */
export function isSharedFolder(file: AppFile | null | undefined): boolean {
  return (
    !!file &&
    file.mime === 'dir' &&
    (file.is_owner === false || file.members_signed_at != null)
  )
}

export type MoveDecision =
  | { kind: 'plain'; sources: AppFile[]; destinationId: string | undefined }
  | { kind: 'blocked'; message: string }
  | { kind: 'into-shared'; sources: AppFile[]; destinationFolderId: string }
  | { kind: 'move-out'; sources: AppFile[]; destinationId: string | undefined }

/**
 * Classify a requested move from the sources, their current parent, and the
 * chosen destination — the single place the funnel's decision tree lives,
 * mirroring the mobile `MoveRouter`. Reads only share-state; it never wraps a
 * key or hits a mutation endpoint. The server still enforces every hard rule
 * (ownership, dest-is-shared, folder-requires-cascade); this is the client-side
 * routing plus the early "block the whole move on any ineligible item" guard.
 *
 * When `sharingEnabled` is false (kill-switch or an older server), every
 * destination resolves to a plain move so a server that doesn't speak sharing
 * never engages these paths.
 */
export function classifyMove(args: {
  sources: AppFile[]
  destination: AppFile | undefined
  sourceParent: AppFile | null
  sharingEnabled: boolean
}): MoveDecision {
  const { sources, destination, sourceParent, sharingEnabled } = args
  if (sources.length === 0) {
    return { kind: 'plain', sources, destinationId: destination?.id }
  }

  const destShared = sharingEnabled && isSharedFolder(destination)
  if (destShared && destination) {
    if (sources.some((f) => !f.is_owner)) {
      return {
        kind: 'blocked',
        message: 'You can only move files you own into a shared folder.'
      }
    }
    return { kind: 'into-shared', sources, destinationFolderId: destination.id }
  }

  const sourceShared = sharingEnabled && isSharedFolder(sourceParent)
  if (!sourceShared) {
    return { kind: 'plain', sources, destinationId: destination?.id }
  }

  if (sources.some((f) => !f.is_owner)) {
    return {
      kind: 'blocked',
      message: 'Only the owner can move a file out of a shared folder.'
    }
  }
  return { kind: 'move-out', sources, destinationId: destination?.id }
}

export interface MoveResult {
  moved: number
  blocked?: string
}

export interface ExecuteMoveDeps {
  callerUserId: string
  keypair: KeyPair
  /** Runs the unchanged batch relocation (`Storage.moveAll`). */
  plainMove: (sources: AppFile[], destinationId: string | undefined) => Promise<void>
  /** Fires before any folder cascade wraps a key; false aborts the move. */
  confirmCascade?: (folder: AppFile, preview: MoveCascadePreview) => Promise<boolean>
  onProgress?: MoveIntoSharedFolderOptions['onProgress']
  onSubtreeProgress?: MoveIntoSharedFolderOptions['onSubtreeProgress']
  onUnknownMember?: (prompt: UnknownMemberPrompt) => Promise<boolean>
}

/**
 * Run a classified move, mirroring the mobile `MoveExecutor`. Owned items
 * bound for a shared folder are routed individually — a file through the
 * single-file `member_keys` path, a directory through the cascade `entries`
 * path (which fires the confirm gate before wrapping). The batch was
 * guaranteed all-owned by `classifyMove`, so there is no partial-skip: the
 * first failure aborts and surfaces.
 */
export async function executeMove(
  decision: MoveDecision,
  deps: ExecuteMoveDeps
): Promise<MoveResult> {
  if (!deps.keypair.input || !deps.keypair.publicKey) {
    throw new Error('Cannot move without an active keypair')
  }
  const privateKey = deps.keypair.input
  const wrappingPrivateKey = deps.keypair.wrappingPrivate || deps.keypair.input
  const trusted = trustedFingerprintsStore()
  const onUnknownMember = deps.onUnknownMember ?? (async () => true)

  switch (decision.kind) {
    case 'blocked':
      return { moved: 0, blocked: decision.message }

    case 'plain':
      await deps.plainMove(decision.sources, decision.destinationId)
      return { moved: decision.sources.length }

    case 'into-shared': {
      try {
        for (const item of decision.sources) {
          if (item.mime === 'dir') {
            await moveIntoSharedFolder(
              {
                callerUserId: deps.callerUserId,
                callerPrivateKey: privateKey,
                callerWrappingPrivateKey: wrappingPrivateKey,
                root: item,
                destinationFolderId: decision.destinationFolderId,
                trustedFingerprints: trusted,
                onUnknownMember
              },
              {
                onProgress: deps.onProgress,
                onSubtreeProgress: deps.onSubtreeProgress,
                confirm: deps.confirmCascade
                  ? (preview) => deps.confirmCascade!(item, preview)
                  : undefined
              }
            )
          } else {
            await moveSingleFileIntoSharedFolder({
              callerUserId: deps.callerUserId,
              callerPrivateKey: privateKey,
              callerWrappingPrivateKey: wrappingPrivateKey,
              file: item,
              destinationFolderId: decision.destinationFolderId,
              trustedFingerprints: trusted,
              onUnknownMember
            })
          }
        }
      } catch (err) {
        // A declined cascade-confirm (or TOFU prompt) aborts the move with
        // nothing sent — surface it as a no-op, not an error.
        if (err instanceof UploadIntoSharedFolderAborted) {
          return { moved: 0 }
        }
        throw err
      }
      return { moved: decision.sources.length }
    }

    case 'move-out': {
      for (const item of decision.sources) {
        await moveOutOfSharedFolder({
          callerUserId: deps.callerUserId,
          callerPrivateKey: privateKey,
          fileId: item.id,
          destinationFolderId: decision.destinationId ?? null
        })
      }
      return { moved: decision.sources.length }
    }
  }
}
