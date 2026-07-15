import { computed, ref, watch } from 'vue'
import type { Ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import {
  store as sharesStoreFactory,
  crypto as shareCrypto,
  api as sharesApi,
  DiscoverUserError
} from '!/shares'
import { meta as storageMeta } from '!/storage'
import { errorNotification } from '!/index'

import type { AuditEventAction, KeyPair, ShareEvent, ShareRole } from 'types'

export const ACTION_LABELS: Record<AuditEventAction, string> = {
  grant: 'Shared',
  revoke: 'Revoked',
  role_change: 'Changed role',
  shared_folder_upload: 'Uploaded into shared folder',
  fork: 'Forked',
  shared_by_co_owner: 'Re-shared as co-owner',
  shared_folder_edit: 'Edited shared file',
  shared_folder_restore: 'Restored shared version',
  shared_folder_evict: 'Cascade revoked',
  shared_folder_move_out: 'Moved out of shared folder',
  key_rotation: 'Key rotation'
}

/**
 * Tri-state per-row classification.
 *
 *   - **verified** — silent. The row IS the signal. No decoration.
 *   - **system** — neutral pill ("System"). The row legitimately has no
 *     sender signature by construction (cascade-revoke fan-outs, server-
 *     only events), OR is an account-level event (`key_rotation`) signed
 *     under a different scheme this view doesn't re-verify. The row is
 *     still cryptographically chained.
 *   - **tampered** — full-width banner with a forensic export CTA. Fires
 *     when ANY of the three local checks fails on a non-system row:
 *     (a) the row's signature fails to verify against the named sender,
 *     (b) the row's self-hash recompute disagrees with the stored hash,
 *     (c) the chain link between two visible adjacent rows in the same
 *         bucket is broken.
 *     A page-boundary gap (predecessor outside the loaded slice) is NOT
 *     tampering — the slice-aware verifier classifies that as
 *     `page-boundary`, which passes.
 */
export type RowVerificationState = 'verified' | 'system' | 'tampered'

export interface RowDisclosure {
  rowIdShort: string
  thisHashTail: string
  prevHashTail: string
  senderSigStatus: string | null
  chainStatus: string | null
}

function asString(value: unknown): string {
  if (Array.isArray(value)) return value[0] ?? ''
  return typeof value === 'string' ? value : ''
}

function parseActionQuery(value: string): 'all' | AuditEventAction {
  if (value && value in ACTION_LABELS) return value as AuditEventAction
  return 'all'
}

export function useShareHubAudit(keypair: Ref<KeyPair | undefined>) {
  const route = useRoute()
  const router = useRouter()
  const shares = sharesStoreFactory()

  const fileIdFilter = ref<string>(asString(route.query.file_id))
  const actionFilter = ref<'all' | AuditEventAction>(parseActionQuery(asString(route.query.action)))
  // Sender filter is captured by email (user-typed) and resolved into a
  // user_id through the existing discover endpoint. The query string still
  // uses ?sender=<user_id> so deep links remain stable across the rename.
  const senderEmailInput = ref<string>('')
  const senderFilter = ref<string>(asString(route.query.sender))
  const senderError = ref<string | null>(null)
  const senderResolving = ref(false)
  const recipientFilter = ref<string>(asString(route.query.recipient))
  const startDate = ref<string>(asString(route.query.from))
  const endDate = ref<string>(asString(route.query.to))

  const sigChecks = ref<Record<string, boolean>>({})
  const loading = ref(false)
  const expandedRows = ref<Set<string>>(new Set())

  async function refresh(): Promise<void> {
    loading.value = true
    try {
      await shares.loadEvents({
        file_id: fileIdFilter.value || undefined,
        action: actionFilter.value === 'all' ? undefined : actionFilter.value,
        limit: 100
      })
      await verifySignatures()
    } catch (err) {
      errorNotification(err)
    } finally {
      loading.value = false
    }
  }

  async function verifySignatures(): Promise<void> {
    const checks: Record<string, boolean> = {}
    for (const row of shares.events) {
      // `key_rotation` is signed under the key-rotation scheme, not the
      // share-event canonical — it's classified `system`, so skip the
      // share-signature check rather than verify against the wrong input.
      if (row.action === 'key_rotation') continue
      if (!row.sender_signature || !row.sender_id) {
        checks[row.id] = false
        continue
      }
      const senderRecord = shares.eventUsers[row.sender_id]
      if (!senderRecord) {
        checks[row.id] = false
        continue
      }
      try {
        checks[row.id] = await shareCrypto.verifyEventSignature(row, senderRecord)
      } catch {
        checks[row.id] = false
      }
    }
    sigChecks.value = checks
  }

  watch(
    () => [
      fileIdFilter.value,
      actionFilter.value,
      senderFilter.value,
      recipientFilter.value,
      startDate.value,
      endDate.value
    ],
    () => {
      const query: Record<string, string> = {}
      if (fileIdFilter.value) query.file_id = fileIdFilter.value
      if (actionFilter.value !== 'all') query.action = actionFilter.value
      if (senderFilter.value) query.sender = senderFilter.value
      if (recipientFilter.value) query.recipient = recipientFilter.value
      if (startDate.value) query.from = startDate.value
      if (endDate.value) query.to = endDate.value
      router.replace({ query }).catch(() => {})
      void refresh()
    }
  )

  const chainVerification = computed(() => shareCrypto.verifyChain(shares.events))

  function rowChainStatus(rowIndex: number): shareCrypto.ChainRowStatus | undefined {
    return chainVerification.value.rowStatus[rowIndex]
  }

  const filteredRows = computed<ShareEvent[]>(() => {
    return shares.events.filter((row) => {
      if (senderFilter.value && row.sender_id !== senderFilter.value) return false
      if (recipientFilter.value && row.recipient_id !== recipientFilter.value) return false
      if (startDate.value) {
        const start = Date.parse(startDate.value) / 1000
        if (Number.isFinite(start) && row.created_at < start) return false
      }
      if (endDate.value) {
        const end = Date.parse(endDate.value) / 1000 + 24 * 60 * 60
        if (Number.isFinite(end) && row.created_at > end) return false
      }
      return true
    })
  })

  function senderEmail(row: ShareEvent): string {
    if (!row.sender_id) return 'system'
    return shares.eventUsers[row.sender_id]?.email ?? row.sender_id.slice(0, 8)
  }

  function recipientEmail(row: ShareEvent): string {
    if (!row.recipient_id) return ''
    return shares.eventUsers[row.recipient_id]?.email ?? row.recipient_id.slice(0, 8)
  }

  function formatRole(role: ShareRole): string {
    return role === 'co-owner' ? 'Co-owner' : role.charAt(0).toUpperCase() + role.slice(1)
  }

  /**
   * In-component memo of decrypted file names, keyed by `file_id`. The
   * server's `events_for_user` query carries `encrypted_name` + `cipher` +
   * `encrypted_key` per row via two LEFT JOINs; we unwrap once per file id
   * (a single grant row and its revoke share the same id) and reuse across
   * re-renders. Cleared when `shares.events` swaps, same lifecycle as the
   * row-state cache above — both live for this component instance only, no
   * global cross-page cache.
   */
  const decryptedNames = ref<Map<string, string>>(new Map())
  const decryptedNamesVersion = ref(0)

  async function hydrateDecryptedNames(events: ShareEvent[]): Promise<void> {
    const privateKey = keypair.value?.wrappingPrivate || keypair.value?.input
    if (!privateKey) return
    // A file_id we already decrypted in a previous batch stays — the
    // ciphertext is immutable for the lifetime of the file row.
    const seen = new Set(decryptedNames.value.keys())
    let touched = false
    for (const row of events) {
      if (!row.file_id) continue
      if (seen.has(row.file_id)) continue
      if (!row.encrypted_name || !row.encrypted_key || !row.cipher) continue
      seen.add(row.file_id)
      try {
        const decrypted = await storageMeta.decrypt(
          {
            encrypted_key: row.encrypted_key,
            encrypted_name: row.encrypted_name,
            cipher: row.cipher
          },
          privateKey as string
        )
        decryptedNames.value.set(row.file_id, decrypted.name)
        touched = true
      } catch {
        // A decrypt failure leaves the map entry unset; rowSentence falls
        // back to the truncated id. Wrong-key rows happen legitimately
        // (key rotation across a long audit history) and shouldn't show
        // as an error — the id is correct, just less readable.
      }
    }
    if (touched) {
      decryptedNamesVersion.value += 1
    }
  }

  watch(
    () => shares.events,
    (events) => {
      decryptedNames.value = new Map()
      decryptedNamesVersion.value += 1
      void hydrateDecryptedNames(events)
    }
  )

  /**
   * One-line sentence describing the event in everyday English. Keeps the
   * structure consistent across actions so a glance down the list reads
   * naturally: "<sender> <verb> <object> [with <recipient>] [as <role>]."
   * Falls back to the raw action token if a new action ships without a
   * sentence template — better that than a blank.
   */
  function rowSentence(row: ShareEvent): string {
    // Reactive read so the sentence re-renders once decryption resolves.
    void decryptedNamesVersion.value
    const sender = senderEmail(row)
    const recipient = recipientEmail(row)
    const decryptedName = row.file_id ? decryptedNames.value.get(row.file_id) : undefined
    const fileLabel =
      decryptedName ?? (row.file_id ? `file ${row.file_id.slice(0, 8)}…` : 'a file')
    const roleAfter = row.share_role_after ? formatRole(row.share_role_after) : null
    const roleBefore = row.share_role_before ? formatRole(row.share_role_before) : null

    switch (row.action) {
      case 'grant':
        return `${sender} shared ${fileLabel} with ${recipient}${roleAfter ? ` as ${roleAfter}` : ''}`
      case 'shared_by_co_owner':
        return `${sender} re-shared ${fileLabel} with ${recipient}${roleAfter ? ` as ${roleAfter}` : ''}`
      case 'revoke':
        return `${sender} revoked ${recipient || 'access'} from ${fileLabel}`
      case 'role_change':
        if (roleBefore && roleAfter) {
          return `${sender} changed ${recipient || 'recipient'}'s role on ${fileLabel} from ${roleBefore} to ${roleAfter}`
        }
        return `${sender} changed ${recipient || 'recipient'}'s role on ${fileLabel}`
      case 'fork':
        return `${sender} forked ${fileLabel} into their drive`
      case 'shared_folder_upload':
        return `${sender} uploaded into shared folder ${fileLabel}`
      case 'shared_folder_edit':
        return `${sender} edited shared file ${fileLabel}`
      case 'shared_folder_restore':
        return `${sender} restored a previous version of shared file ${fileLabel}`
      case 'shared_folder_evict':
        return `${recipient || 'A recipient'} lost access to ${fileLabel} (cascade)`
      case 'shared_folder_move_out':
        return `${sender} moved ${fileLabel} out of a shared folder`
      case 'key_rotation':
        return `${sender} rotated their account encryption keys`
      default:
        return `${ACTION_LABELS[row.action] ?? row.action} on ${fileLabel}`
    }
  }

  // Per-row verification memo, scoped to this component instance. Reset on
  // every mount (each `useShareHubAudit` call builds its own Map) and
  // invalidated by the watchers below whenever `shares.events` or
  // `sigChecks` change, so one account's row state can never leak into
  // another's view.
  const rowStateCache = new Map<string, RowVerificationState>()

  function rowState(row: ShareEvent): RowVerificationState {
    const cached = rowStateCache.get(row.id)
    if (cached) return cached
    const state: RowVerificationState = computeRowState(row)
    rowStateCache.set(row.id, state)
    return state
  }

  function computeRowState(row: ShareEvent): RowVerificationState {
    // Cascade-revoke fan-outs and other server-attributed events ship
    // without a sender signature by construction. `key_rotation` is signed,
    // but under the key-rotation scheme rather than the share-event canonical.
    // Treat both as system — the "System" pill carries the meaning. Chain math
    // still runs for these rows; they're verifiable, just not via this view's
    // share-signature check.
    if (!row.sender_signature || row.action === 'key_rotation') return 'system'

    const rowIndex = shares.events.indexOf(row)
    const chainStatus = rowIndex >= 0 ? rowChainStatus(rowIndex) : undefined
    const sigOk = computeSigOk(row)
    const chainTampered = chainStatus === 'self-hash-mismatch' || chainStatus === 'link-broken'
    if (!sigOk || chainTampered) return 'tampered'
    return 'verified'
  }

  function computeSigOk(row: ShareEvent): boolean {
    if (!row.sender_signature || !row.sender_id) return false
    const check = sigChecks.value[row.id]
    // `verifySignatures` populates `sigChecks` after each refresh. While
    // it's resolving, default to verified so we don't flash a red
    // banner on every page load.
    return check === undefined ? true : check === true
  }

  /**
   * Headline copy for the tampered banner. Names the most-severe failing
   * check so the reader has a concrete starting point — chain breaks
   * point at deletion / forge attempts, signature failures point at
   * key-substitution or row-content tampering. When multiple checks fail
   * at once we lead with the signature failure (the primary defense).
   */
  function tamperedHeadline(row: ShareEvent): string {
    const rowIndex = shares.events.indexOf(row)
    const chainStatus = rowIndex >= 0 ? rowChainStatus(rowIndex) : undefined
    if (!computeSigOk(row)) {
      return 'Signature failed verification on this event.'
    }
    if (chainStatus === 'self-hash-mismatch') {
      return 'Row content does not match its stored hash.'
    }
    if (chainStatus === 'link-broken') {
      return 'Chain link to the previous visible event is broken.'
    }
    return 'Tamper indicator on this event.'
  }

  /**
   * Disclosure payload — per-row verification breakdown the SPA can
   * locally compute. Hashes always render; signature and chain lines are
   * present only when they have useful information. System rows omit the
   * signature line (cascade-revoke fan-outs legitimately ship without a
   * sender signature — the "System" pill carries that meaning). Verified
   * chain rows omit the chain line so the disclosure stays short for the
   * happy path; only page-boundary and tampered states surface chain copy.
   */
  function rowDisclosure(row: ShareEvent): RowDisclosure {
    const rowIndex = shares.events.indexOf(row)
    const chainStatus = rowIndex >= 0 ? rowChainStatus(rowIndex) : undefined
    const state = rowState(row)
    const hashTail = (h: string | null | undefined) => (h ? h.slice(-16) : '—')

    let senderSigStatus: string | null = null
    if (state !== 'system' && row.sender_id) {
      if (!row.sender_signature) {
        senderSigStatus = 'Sender signature missing — this row should not exist'
      } else if (sigChecks.value[row.id] === true) {
        senderSigStatus = 'Verified against sender pubkey'
      } else if (sigChecks.value[row.id] === false) {
        senderSigStatus = 'Failed verification against sender pubkey'
      } else {
        senderSigStatus = 'Pending verification'
      }
    }

    let chainCopy: string | null = null
    if (chainStatus === 'page-boundary') {
      chainCopy = 'Earlier event in this chain is on another page'
    } else if (chainStatus === 'self-hash-mismatch') {
      chainCopy = 'Row content does not match its stored hash'
    } else if (chainStatus === 'link-broken') {
      chainCopy = 'Chain link to the previous visible event is broken'
    }

    return {
      rowIdShort: row.id.slice(-12),
      thisHashTail: hashTail(row.this_event_hash),
      prevHashTail: hashTail(row.prev_event_hash),
      senderSigStatus,
      chainStatus: chainCopy
    }
  }

  function toggleDisclosure(rowId: string): void {
    if (expandedRows.value.has(rowId)) {
      expandedRows.value.delete(rowId)
    } else {
      expandedRows.value.add(rowId)
    }
    // Force Vue's reactivity to pick the Set mutation up.
    expandedRows.value = new Set(expandedRows.value)
  }

  function isExpanded(rowId: string): boolean {
    return expandedRows.value.has(rowId)
  }

  const hasActiveFilter = computed(() => {
    return Boolean(
      fileIdFilter.value ||
        actionFilter.value !== 'all' ||
        senderFilter.value ||
        recipientFilter.value ||
        startDate.value ||
        endDate.value
    )
  })

  /**
   * Forensic export hand-off — copies the row's raw JSON + computed
   * hash status to the clipboard so a power user can paste it into an
   * issue tracker or a back-and-forth with the project maintainer.
   * Client-only by design (no new server endpoint for this).
   */
  async function exportRow(row: ShareEvent): Promise<void> {
    const disclosure = rowDisclosure(row)
    const payload = {
      row: {
        id: row.id,
        sender_id: row.sender_id,
        recipient_id: row.recipient_id,
        file_id: row.file_id,
        action: row.action,
        share_role_before: row.share_role_before,
        share_role_after: row.share_role_after,
        created_at: row.created_at,
        prev_event_hash: row.prev_event_hash,
        this_event_hash: row.this_event_hash,
        sender_signature: row.sender_signature
      },
      verification: disclosure
    }
    try {
      await navigator.clipboard.writeText(JSON.stringify(payload, null, 2))
    } catch {
      // Clipboard access can fail in narrow embed contexts; the
      // disclosure remains visible so the user can copy by hand.
    }
  }

  watch(
    () => shares.events,
    () => {
      rowStateCache.clear()
    }
  )

  watch(
    () => sigChecks.value,
    () => {
      rowStateCache.clear()
    },
    { deep: true }
  )

  function clearFilters(): void {
    fileIdFilter.value = ''
    actionFilter.value = 'all'
    senderEmailInput.value = ''
    senderFilter.value = ''
    senderError.value = null
    recipientFilter.value = ''
    startDate.value = ''
    endDate.value = ''
  }

  /**
   * Resolve the typed email into a user_id via the existing discover
   * endpoint, then drive the existing `senderFilter` ref. The events list
   * already filters by `sender_id` client-side; once a real user_id lands
   * in `senderFilter`, the existing watcher repaints the table.
   *
   * Empty / whitespace input clears the filter outright. Discovery errors
   * surface as inline copy beside the input — the events list stays as-is
   * so a failed lookup never blanks the user's current view.
   */
  async function resolveSenderEmail(): Promise<void> {
    senderError.value = null
    const trimmed = senderEmailInput.value.trim()
    if (!trimmed) {
      senderFilter.value = ''
      return
    }
    senderResolving.value = true
    try {
      const user = await sharesApi.discoverUser(trimmed)
      senderFilter.value = user.user_id
    } catch (err) {
      if (err instanceof DiscoverUserError) {
        switch (err.kind) {
          case 'not_found':
            senderError.value = "We couldn't find a Hoodik account for that email."
            break
          case 'self':
            senderError.value = "That's your own account — pick another email."
            break
          case 'rate_limited':
            senderError.value = err.retryAfterSeconds
              ? `Too many lookups — try again in ${err.retryAfterSeconds}s.`
              : 'Too many lookups — slow down and retry.'
            break
          case 'feature_disabled':
            senderError.value = 'Account discovery is disabled on this server.'
            break
          default:
            senderError.value = 'Could not resolve that email right now.'
        }
      } else {
        senderError.value = 'Could not resolve that email right now.'
      }
    } finally {
      senderResolving.value = false
    }
  }

  return {
    fileIdFilter,
    actionFilter,
    senderEmailInput,
    senderError,
    senderResolving,
    recipientFilter,
    startDate,
    endDate,
    loading,
    filteredRows,
    hasActiveFilter,
    refresh,
    rowSentence,
    senderEmail,
    recipientEmail,
    rowState,
    tamperedHeadline,
    rowDisclosure,
    toggleDisclosure,
    isExpanded,
    exportRow,
    clearFilters,
    resolveSenderEmail
  }
}
