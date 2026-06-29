import { defineStore, storeToRefs } from 'pinia'
import { computed, ref } from 'vue'
import type { ComputedRef } from 'vue'

import type {
  AppFile,
  AppLink,
  AppShare,
  CreateShareEnvelope,
  KeyPair,
  RevokeShareBody
} from 'types'

import { store as sharesStoreFactory } from './index'
import { store as linksStoreFactory } from '!/links'

/**
 * Discriminated grant — either a recipient-side share row or a public-link
 * row, both keyed under the same file_id. The split exists at the consent
 * layer (named-pubkey vs. URL-holder); the modal renders the two kinds on
 * separate tabs but the data layer treats them as one ledger so the UI can
 * later collapse if usage data warrants it.
 */
export type UserGrant = {
  kind: 'user'
  file_id: string
  recipient_id: string
  user: AppShare
}

export type LinkGrant = {
  kind: 'link'
  file_id: string
  link_id: string
  link: AppLink
}

export type Grant = UserGrant | LinkGrant

type LoadState = 'idle' | 'loading' | 'loaded' | 'error'

/**
 * Per-file load tracking so the modal can render skeletons without racing
 * the user's first interaction. Failures stick to the file id so a retry
 * is a deliberate user action, not an auto-loop.
 */
interface PerFileState {
  state: LoadState
  error: string | null
}

/**
 * Read-side aggregator over the existing shares + links Pinia stores.
 * Owns no domain state of its own — when the underlying stores update
 * (createShare, revoke, Links.upsertItem, …) the computed grants list
 * reflects that on the next tick. Writers are pass-throughs so callers
 * don't have to know about the split.
 */
export const grantsStore = defineStore('grants', () => {
  const shares = sharesStoreFactory()
  const links = linksStoreFactory()
  const { outgoingByFile } = storeToRefs(shares)

  const perFile = ref<Record<string, PerFileState>>({})

  function setState(fileId: string, state: LoadState, error: string | null = null): void {
    perFile.value = { ...perFile.value, [fileId]: { state, error } }
  }

  function stateOf(fileId: string): PerFileState {
    return perFile.value[fileId] ?? { state: 'idle', error: null }
  }

  function isLoading(fileId: string): boolean {
    return stateOf(fileId).state === 'loading'
  }

  function errorOf(fileId: string): string | null {
    return stateOf(fileId).error
  }

  /**
   * Pull both ledgers for `fileId` in parallel. The link side is loaded
   * via `Links.find(kp)` — that decrypts every link the caller owns into
   * the links store; the file-scoped grants then read from that store.
   * `find` is idempotent in practice (upserts by id), so calling it again
   * for a follow-up open is cheap.
   */
  async function loadGrants(fileId: string, kp: KeyPair): Promise<void> {
    setState(fileId, 'loading')
    try {
      await Promise.all([
        shares.loadOutgoingFor(fileId).catch((err) => {
          throw err instanceof Error ? err : new Error(String(err))
        }),
        links.find(kp).catch((err) => {
          throw err instanceof Error ? err : new Error(String(err))
        })
      ])
      setState(fileId, 'loaded')
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load grants'
      setState(fileId, 'error', message)
      throw err
    }
  }

  /**
   * Merged list for the modal. Reading an unloaded file id is safe — the
   * computed returns an empty array so the empty-state UI can render
   * before `loadGrants` resolves.
   */
  function grants(fileId: string): ComputedRef<Grant[]> {
    return computed(() => {
      const users: Grant[] = (outgoingByFile.value[fileId] ?? []).map((user) => ({
        kind: 'user' as const,
        file_id: fileId,
        recipient_id: user.recipient_id,
        user
      }))
      const linkRows: Grant[] = links.items
        .filter((link) => link.file_id === fileId)
        .map((link) => ({
          kind: 'link' as const,
          file_id: fileId,
          link_id: link.id,
          link
        }))
      return [...users, ...linkRows]
    })
  }

  function userGrants(fileId: string): ComputedRef<UserGrant[]> {
    return computed(
      () => grants(fileId).value.filter((g): g is UserGrant => g.kind === 'user')
    )
  }

  function linkGrants(fileId: string): ComputedRef<LinkGrant[]> {
    return computed(
      () => grants(fileId).value.filter((g): g is LinkGrant => g.kind === 'link')
    )
  }

  /**
   * Pass-throughs. Each one delegates to the relevant store so the wider
   * codebase (audit log, Links view, FolderMembersView) sees the same
   * mutation. createShare returns the rows the server confirmed; revoke
   * is fire-and-forget because the server has no useful response payload.
   */
  async function addUserGrant(envelope: CreateShareEnvelope): Promise<AppShare[]> {
    return shares.createShare(envelope)
  }

  async function revokeGrant(
    fileId: string,
    userId: string,
    body: RevokeShareBody
  ): Promise<void> {
    await shares.revoke(fileId, userId, body)
  }

  async function createLink(file: AppFile, kp: KeyPair): Promise<AppLink> {
    const link = await links.create(file, kp)
    links.upsertItem(link)
    return link
  }

  async function revokeLink(linkId: string): Promise<void> {
    await links.remove(linkId)
  }

  return {
    perFile,
    grants,
    userGrants,
    linkGrants,
    isLoading,
    errorOf,
    stateOf,
    loadGrants,
    addUserGrant,
    revokeGrant,
    createLink,
    revokeLink
  }
})

export type GrantsStore = ReturnType<typeof grantsStore>
