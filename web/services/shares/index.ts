import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

import type {
  AppShare,
  AuditUserRef,
  Capabilities,
  CreateShareEnvelope,
  ForkBody,
  ForkResponse,
  IncomingShare,
  RevokeShareBody,
  ShareEvent,
  ShareRole,
  TrustedFingerprintEntry
} from 'types'

import { store as storageStore } from '!/storage'

import * as api from './api'
import * as crypto from './crypto'
import * as editable from './editable'
import * as fork from './fork'
import * as groups from './groups'
import * as subtree from './subtree'

export { api, crypto, editable, fork, groups, subtree }
export { GroupMemberFingerprintMismatch } from './groups'
export { grantsStore } from './grants'
export type { Grant, UserGrant, LinkGrant, GrantsStore } from './grants'
export { ForkAbortedError } from './fork'
export type { ForkArgs, ForkOptions, ForkProgress } from './fork'
export { DiscoverUserError, ShareMembershipChangedError, MoveOutRejectedError } from './api'
export {
  FolderMemberListInvalid,
  FolderMemberFingerprintChanged,
  UploadIntoSharedFolderAborted,
  uploadIntoSharedFolder,
  moveSingleFileIntoSharedFolder,
  moveIntoSharedFolder,
  moveOutOfSharedFolder,
  verifyFolderMemberList,
  buildSharedFolderPayloadFromFile
} from './editable'
export type {
  SharedFolderFilePayload,
  UnknownMemberPrompt,
  UploadIntoSharedFolderArgs,
  UploadIntoSharedFolderOptions,
  UploadIntoSharedFolderProgress,
  MoveCascadePreview,
  MoveIntoSharedFolderArgs,
  MoveIntoSharedFolderOptions
} from './editable'
export {
  SubtreeCapExceeded,
  SubtreeAborted,
  SUBTREE_HARD_CAP,
  SUBTREE_DETERMINATE_THRESHOLD
} from './subtree'

const FAIL_CLOSED_CAPABILITIES: Capabilities = {
  sharing: { enabled: false, roles: [] },
  editable_folders: false,
  share_groups: false,
  audit_log: false,
  fork: false
}

/**
 * Primary share state. `incoming` carries the recipient-side list rendered
 * by the "Shared with me" surface; `outgoingByFile` is filled on demand
 * when the owner opens the recipient list for a specific file.
 */
export const store = defineStore('shares', () => {
  const incoming = ref<IncomingShare[]>([])
  const outgoingByFile = ref<Record<string, AppShare[]>>({})
  const events = ref<ShareEvent[]>([])
  const eventUsers = ref<Record<string, AuditUserRef>>({})
  const eventsTotal = ref<number>(0)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const lastSeenAt = ref<number>(0)

  const incomingFromSender = computed(() => (senderId: string): IncomingShare[] => {
    return incoming.value.filter((s) => s.shared_by_user_id === senderId || s.owner_id === senderId)
  })

  const incomingCount = computed(() => incoming.value.length)

  const unreadCount = computed(() => {
    return incoming.value.filter((s) => {
      const stamp = s.shared_at ?? s.created_at
      return stamp > lastSeenAt.value
    }).length
  })

  async function loadIncoming(limit = 50, offset = 0): Promise<void> {
    loading.value = true
    error.value = null
    try {
      const page = await api.getSharesMine(limit, offset)
      if (offset === 0) {
        incoming.value = page.items
      } else {
        incoming.value = [...incoming.value, ...page.items]
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to load incoming shares'
      throw e
    } finally {
      loading.value = false
    }
  }

  async function loadOutgoingFor(fileId: string): Promise<AppShare[]> {
    loading.value = true
    try {
      const recipients = await api.getShareRecipients(fileId)
      outgoingByFile.value = { ...outgoingByFile.value, [fileId]: recipients }
      return recipients
    } finally {
      loading.value = false
    }
  }

  async function createShare(envelope: CreateShareEnvelope): Promise<AppShare[]> {
    const result = await api.createShare(envelope)
    const rootFileId = result.shares[0]?.file_id
    if (rootFileId) {
      const existing = outgoingByFile.value[rootFileId] ?? []
      const existingRecipients = new Set(existing.map((row) => row.recipient_id))
      const incomingIds = new Set(result.shares.map((s) => s.recipient_id))
      const merged = [
        ...existing.filter((row) => !incomingIds.has(row.recipient_id)),
        ...result.shares
      ]
      outgoingByFile.value = { ...outgoingByFile.value, [rootFileId]: merged }

      // Bump the file-browser `shared_with_count` badge for net-new
      // recipients. Role-changes on existing recipients don't add to the
      // count — server-side `enrich_shared_with_counts` counts distinct
      // recipients, not events.
      const newRecipients = new Set(
        result.shares
          .map((s) => s.recipient_id)
          .filter((id) => !existingRecipients.has(id))
      )
      if (newRecipients.size > 0) {
        storageStore().bumpSharedWithCount(rootFileId, newRecipients.size)
      }
    }
    return result.shares
  }

  async function revoke(
    fileId: string,
    userId: string,
    body: RevokeShareBody
  ): Promise<void> {
    await api.revokeShare(fileId, userId, body)
    // Drop the directly-revoked row and any rows the revoked recipient
    // had granted under this scope. The server cascades Co-owner
    // grants on revoke; without mirroring that here
    // the list stays stale until a refresh. Non-Co-owner revokes have
    // no downstream grants, so the cascade filter is a no-op for them.
    const existing = outgoingByFile.value[fileId]
    if (existing) {
      const droppedRecipients = new Set<string>()
      const remaining = existing.filter((row) => {
        const drop = row.recipient_id === userId || row.shared_by_user_id === userId
        if (drop) droppedRecipients.add(row.recipient_id)
        return !drop
      })
      outgoingByFile.value = { ...outgoingByFile.value, [fileId]: remaining }
      if (droppedRecipients.size > 0) {
        storageStore().bumpSharedWithCount(fileId, -droppedRecipients.size)
      }
    }
    // The recipient list only carries incoming rows for the current
    // session, so a self-remove (caller == userId) drops the row by
    // file_id. Owner-side revokes leave `incoming` untouched because
    // the owner's incoming list never held the file in the first place.
    incoming.value = incoming.value.filter((row) => row.file_id !== fileId)
  }

  async function fork(fileId: string, body: ForkBody): Promise<ForkResponse> {
    const result = await api.forkFile(fileId, body)
    return result
  }

  async function loadEvents(query: {
    file_id?: string
    action?: string
    limit?: number
    offset?: number
  } = {}): Promise<void> {
    loading.value = true
    try {
      const page = await api.getShareEvents(query)
      events.value = page.events
      eventUsers.value = page.users ?? {}
      eventsTotal.value = page.total
    } finally {
      loading.value = false
    }
  }

  function markSeenNow(): void {
    lastSeenAt.value = Math.floor(Date.now() / 1000)
  }

  function reset(): void {
    incoming.value = []
    outgoingByFile.value = {}
    events.value = []
    eventUsers.value = {}
    eventsTotal.value = 0
    error.value = null
    lastSeenAt.value = 0
  }

  return {
    incoming,
    outgoingByFile,
    events,
    eventUsers,
    eventsTotal,
    loading,
    error,
    lastSeenAt,
    incomingFromSender,
    incomingCount,
    unreadCount,
    loadIncoming,
    loadOutgoingFor,
    createShare,
    revoke,
    fork,
    loadEvents,
    markSeenNow,
    reset
  }
})

/**
 * Trust state for recipient fingerprints, scoped to the currently
 * authenticated user. Persisted in `localStorage` so a session restore
 * carries the same trust decisions forward.
 */
const TRUSTED_FP_KEY_PREFIX = 'hoodik:trustedFingerprints:'
const DEFAULT_STALE_DAYS = 90

interface TrustedFingerprintsState {
  ownerUserId: string | null
  map: Record<string, TrustedFingerprintEntry>
}

function loadTrustedFromStorage(ownerUserId: string): Record<string, TrustedFingerprintEntry> {
  if (typeof localStorage === 'undefined') return {}
  const raw = localStorage.getItem(TRUSTED_FP_KEY_PREFIX + ownerUserId)
  if (!raw) return {}
  try {
    const parsed = JSON.parse(raw)
    if (parsed && typeof parsed === 'object') {
      return parsed as Record<string, TrustedFingerprintEntry>
    }
  } catch {
    // Corrupted storage entries fall back to an empty map; the user just
    // has to re-verify next time they share with that recipient.
  }
  return {}
}

function persistTrustedToStorage(
  ownerUserId: string,
  map: Record<string, TrustedFingerprintEntry>
): void {
  if (typeof localStorage === 'undefined') return
  localStorage.setItem(TRUSTED_FP_KEY_PREFIX + ownerUserId, JSON.stringify(map))
}

export const trustedFingerprintsStore = defineStore('trustedFingerprints', () => {
  const state = ref<TrustedFingerprintsState>({ ownerUserId: null, map: {} })

  function bind(ownerUserId: string | null): void {
    if (ownerUserId === state.value.ownerUserId) return
    state.value = {
      ownerUserId,
      map: ownerUserId ? loadTrustedFromStorage(ownerUserId) : {}
    }
  }

  function lookup(userId: string): TrustedFingerprintEntry | null {
    return state.value.map[userId] ?? null
  }

  function trustFingerprint(
    userId: string,
    fingerprint: string,
    method: TrustedFingerprintEntry['verificationMethod']
  ): void {
    const entry: TrustedFingerprintEntry = {
      pubkeyFingerprint: fingerprint,
      lastVerifiedAt: Math.floor(Date.now() / 1000),
      verificationMethod: method
    }
    state.value = {
      ...state.value,
      map: { ...state.value.map, [userId]: entry }
    }
    if (state.value.ownerUserId) {
      persistTrustedToStorage(state.value.ownerUserId, state.value.map)
    }
  }

  function forgetFingerprint(userId: string): void {
    if (!state.value.map[userId]) return
    const next = { ...state.value.map }
    delete next[userId]
    state.value = { ...state.value, map: next }
    if (state.value.ownerUserId) {
      persistTrustedToStorage(state.value.ownerUserId, state.value.map)
    }
  }

  function isStale(userId: string, ninetyDays: number = DEFAULT_STALE_DAYS): boolean {
    const entry = state.value.map[userId]
    if (!entry) return true
    const ageSeconds = Math.floor(Date.now() / 1000) - entry.lastVerifiedAt
    return ageSeconds > ninetyDays * 24 * 60 * 60
  }

  function reset(): void {
    state.value = { ownerUserId: null, map: {} }
  }

  return {
    state,
    bind,
    lookup,
    trustFingerprint,
    forgetFingerprint,
    isStale,
    reset
  }
})

/**
 * Capability advertisement, fetched from the public `GET /api/capabilities`
 * endpoint at app boot and on every successful login. Every getter
 * defaults to `false` so a missing or failed fetch fails closed.
 */
export const capabilitiesStore = defineStore('capabilities', () => {
  const caps = ref<Capabilities | null>(null)
  const loading = ref(false)
  const lastFetchedAt = ref<number | null>(null)
  const fetchError = ref<string | null>(null)

  const sharingEnabled = computed<boolean>(() => caps.value?.sharing.enabled === true)

  const roles = computed<ShareRole[]>(() => caps.value?.sharing.roles ?? [])

  const editableFolders = computed<boolean>(
    () => sharingEnabled.value && caps.value?.editable_folders === true
  )

  const shareGroups = computed<boolean>(
    () => sharingEnabled.value && caps.value?.share_groups === true
  )

  const auditLog = computed<boolean>(
    () => sharingEnabled.value && caps.value?.audit_log === true
  )

  const forkEnabled = computed<boolean>(
    () => sharingEnabled.value && caps.value?.fork === true
  )

  async function fetch(): Promise<void> {
    loading.value = true
    fetchError.value = null
    try {
      caps.value = await api.getCapabilities()
      lastFetchedAt.value = Math.floor(Date.now() / 1000)
    } catch (e) {
      caps.value = FAIL_CLOSED_CAPABILITIES
      fetchError.value = e instanceof Error ? e.message : 'Capability fetch failed'
    } finally {
      loading.value = false
    }
  }

  function reset(): void {
    caps.value = null
    lastFetchedAt.value = null
    fetchError.value = null
  }

  return {
    caps,
    loading,
    lastFetchedAt,
    fetchError,
    sharingEnabled,
    roles,
    editableFolders,
    shareGroups,
    auditLog,
    forkEnabled,
    fetch,
    reset
  }
})

export type SharesStore = ReturnType<typeof store>
export type TrustedFingerprintsStore = ReturnType<typeof trustedFingerprintsStore>
export type CapabilitiesStore = ReturnType<typeof capabilitiesStore>
