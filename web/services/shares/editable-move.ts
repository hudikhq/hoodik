import * as api from './api'
import * as shareCrypto from './crypto'
import * as subtree from './subtree'
import {
  ensureNotAborted,
  verifyAndReconcile,
  withMembershipRetry,
  wrapForEveryMember
} from './editable-members'
import {
  UploadIntoSharedFolderAborted,
  type MoveIntoSharedFolderArgs,
  type MoveIntoSharedFolderOptions,
  type UnknownMemberPrompt
} from './editable-types'

import type {
  AppFile,
  CascadeEntry,
  FolderMembersResponse,
  MoveIntoSharedBody,
  MoveOutOfSharedBody,
  TrustedFingerprintsStore
} from 'types'

/**
 * Move an owned single *file* into a shared folder. Unwraps the file's own key,
 * re-wraps it once per destination member (flat `member_keys` — the existing
 * single-file server shape, distinct from the folder cascade), signs one
 * `shared_folder_upload` event bound to the file, and POSTs. Verifies the
 * destination roster and reconciles fingerprints first (HARD-STOP on failure);
 * a `409 share_membership_changed` re-verifies and retries once.
 */
export async function moveSingleFileIntoSharedFolder(
  args: {
    callerUserId: string
    callerPrivateKey: string
    callerWrappingPrivateKey: string
    file: AppFile
    destinationFolderId: string
    trustedFingerprints: TrustedFingerprintsStore
    onUnknownMember?: (prompt: UnknownMemberPrompt) => Promise<boolean>
  },
  options: {
    signal?: AbortSignal
    fetchMembers?: (folderId: string) => Promise<FolderMembersResponse>
    postMove?: (body: MoveIntoSharedBody) => Promise<void>
  } = {}
): Promise<void> {
  ensureNotAborted(options.signal)
  const fetchMembers = options.fetchMembers ?? api.getFolderMembers
  const poster = options.postMove ?? api.moveIntoShared

  const fileKeyHex = await shareCrypto.decryptOwnFileKey(
    args.file.encrypted_key,
    args.callerWrappingPrivateKey
  )

  const response = await fetchMembers(args.destinationFolderId)
  await verifyAndReconcile(
    response,
    args.callerUserId,
    args.trustedFingerprints,
    args.onUnknownMember
  )

  const submit = async (snapshot: FolderMembersResponse): Promise<void> => {
    ensureNotAborted(options.signal)
    const memberKeys = await wrapForEveryMember(
      fileKeyHex,
      snapshot.members,
      args.callerUserId,
      options.signal,
      undefined
    )
    const timestamp = Math.floor(Date.now() / 1000)
    const auditInput = shareCrypto.buildAuditEventSigInput({
      senderId: args.callerUserId,
      recipientId: null,
      fileId: args.file.id,
      action: 'shared_folder_upload',
      shareRoleBefore: null,
      shareRoleAfter: null,
      timestamp: BigInt(timestamp)
    })
    const eventSignature = await shareCrypto.signAuditEvent(auditInput, args.callerPrivateKey)
    await poster({
      file_id: args.file.id,
      destination_folder_id: args.destinationFolderId,
      member_keys: memberKeys,
      members_list_snapshot: {
        members_signed_at: snapshot.members_signed_at,
        members_list_signature: snapshot.members_list_signature
      },
      event_signature: eventSignature,
      timestamp
    })
  }

  await withMembershipRetry(
    response,
    (snapshot) =>
      verifyAndReconcile(
        snapshot,
        args.callerUserId,
        args.trustedFingerprints,
        args.onUnknownMember
      ),
    submit
  )
}

/**
 * Cascade analog of `uploadIntoSharedFolder` for relocating an owned *folder*
 * (and everything under it) into a shared folder. Enumerates the subtree,
 * verifies the destination roster (signatures + authorized signers) and
 * reconciles fingerprints — both HARD-STOP on failure, nothing is sent — then
 * re-wraps every node's key once per member, signs one `shared_folder_upload`
 * audit event bound to the moved root, and POSTs the cascade body. A
 * `409 share_membership_changed` re-verifies the fresh roster and re-wraps
 * once; a second conflict propagates.
 *
 * Throws the same set as `uploadIntoSharedFolder`, plus
 * `SubtreeCapExceeded`/`SubtreeAborted` from the subtree walk.
 */
export async function moveIntoSharedFolder(
  args: MoveIntoSharedFolderArgs,
  options: MoveIntoSharedFolderOptions = {}
): Promise<void> {
  ensureNotAborted(options.signal)
  const fetchMembers = options.fetchMembers ?? api.getFolderMembers
  const walk = options.collectSubtree ?? ((root: AppFile) =>
    subtree.collectSubtree(root, {
      signal: options.signal,
      onProgress: options.onSubtreeProgress
    }))
  const poster = options.postMove ?? api.moveIntoSharedCascade

  const nodes = await walk(args.root)

  const response = await fetchMembers(args.destinationFolderId)
  options.onProgress?.({
    wrappedKeys: 0,
    totalKeys: nodes.length * response.members.length,
    phase: 'verifying-members'
  })
  await verifyAndReconcile(
    response,
    args.callerUserId,
    args.trustedFingerprints,
    args.onUnknownMember
  )

  if (options.confirm) {
    const proceed = await options.confirm({
      itemCount: nodes.length - 1,
      members: response.members
    })
    if (!proceed) {
      throw new UploadIntoSharedFolderAborted()
    }
  }

  const submit = async (snapshot: FolderMembersResponse): Promise<void> => {
    ensureNotAborted(options.signal)
    const total = nodes.length * snapshot.members.length
    let wrapped = 0
    options.onProgress?.({ wrappedKeys: 0, totalKeys: total, phase: 'wrapping-keys' })
    const entries: CascadeEntry[] = await subtree.buildCascadeEntries(
      nodes,
      snapshot.members,
      args.callerUserId,
      args.callerWrappingPrivateKey,
      {
        signal: options.signal,
        onProgress: () => {
          wrapped += snapshot.members.length
          options.onProgress?.({ wrappedKeys: wrapped, totalKeys: total, phase: 'wrapping-keys' })
        }
      }
    )
    ensureNotAborted(options.signal)
    options.onProgress?.({ wrappedKeys: total, totalKeys: total, phase: 'signing' })
    const timestamp = Math.floor(Date.now() / 1000)
    const auditInput = shareCrypto.buildAuditEventSigInput({
      senderId: args.callerUserId,
      recipientId: null,
      fileId: args.root.id,
      action: 'shared_folder_upload',
      shareRoleBefore: null,
      shareRoleAfter: null,
      timestamp: BigInt(timestamp)
    })
    const eventSignature = await shareCrypto.signAuditEvent(
      auditInput,
      args.callerPrivateKey
    )
    ensureNotAborted(options.signal)
    options.onProgress?.({ wrappedKeys: total, totalKeys: total, phase: 'submitting' })
    await poster({
      file_id: args.root.id,
      destination_folder_id: args.destinationFolderId,
      entries,
      members_list_snapshot: {
        members_signed_at: snapshot.members_signed_at,
        members_list_signature: snapshot.members_list_signature
      },
      event_signature: eventSignature,
      timestamp
    })
    options.onProgress?.({ wrappedKeys: total, totalKeys: total, phase: 'done' })
  }

  await withMembershipRetry(
    response,
    (snapshot) =>
      verifyAndReconcile(
        snapshot,
        args.callerUserId,
        args.trustedFingerprints,
        args.onUnknownMember
      ),
    submit
  )
}

/**
 * The file's owner detaches an owned node (and its subtree, if a folder) from
 * the shared folder it lives in: the nodes revert to private files the owner
 * already holds keys for. No keys are wrapped — the server drops every other
 * member's rows across the subtree. Signs one `shared_folder_move_out` event
 * bound to the moved root and POSTs the body. `destinationFolderId` is the new
 * private parent (null = the owner's drive root).
 */
export async function moveOutOfSharedFolder(
  args: {
    callerUserId: string
    callerPrivateKey: string
    fileId: string
    destinationFolderId?: string | null
  },
  options: { postMove?: (body: MoveOutOfSharedBody) => Promise<void> } = {}
): Promise<void> {
  const poster = options.postMove ?? api.moveOutOfShared
  const timestamp = Math.floor(Date.now() / 1000)
  const auditInput = shareCrypto.buildAuditEventSigInput({
    senderId: args.callerUserId,
    recipientId: null,
    fileId: args.fileId,
    action: 'shared_folder_move_out',
    shareRoleBefore: null,
    shareRoleAfter: null,
    timestamp: BigInt(timestamp)
  })
  const eventSignature = await shareCrypto.signAuditEvent(auditInput, args.callerPrivateKey)
  await poster({
    file_id: args.fileId,
    destination_folder_id: args.destinationFolderId ?? null,
    event_signature: eventSignature,
    timestamp
  })
}
