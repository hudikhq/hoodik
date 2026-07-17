import { computed, onMounted, ref, watch } from 'vue'

import {
  api as sharesApi,
  crypto as shareCrypto,
  groups as shareGroups,
  subtree as shareSubtree,
  DiscoverUserError,
  store as sharesStoreFactory,
  capabilitiesStore,
  trustedFingerprintsStore,
  SubtreeCapExceeded,
  SubtreeAborted,
  SUBTREE_HARD_CAP,
  SUBTREE_DETERMINATE_THRESHOLD
} from '!/shares'
import { errorNotification, notification } from '!/index'

import type {
  AppFile,
  AppShare,
  AuditEventActionWire,
  DiscoveredUser,
  KeyPair,
  ShareEntryInput,
  ShareRole
} from 'types'

/**
 * A group the caller may share into from this dialog: their own groups
 * plus any where they're an editor or co-owner. `memberCount` is known for
 * owned groups (the roster ships with the list) and `null` for member-of
 * groups, where peers aren't the caller's data to enumerate.
 */
export interface ShareableGroup {
  id: string
  name: string
  memberCount: number | null
}

export interface SharingPeopleAddProps {
  file: AppFile | null
  authenticatedUserId: string
  keypair: KeyPair
  prefillEmail?: string
  prefillRole?: ShareRole
  prefillAddFiles?: boolean
  disableCoOwner?: boolean
  ownerId?: string
  readOnly?: boolean
}

export interface SharingPeopleAddEmit {
  (e: 'ok', shares: AppShare[]): void
  (e: 'cancel'): void
}

const RECENT_RECIPIENTS_KEY = 'hoodik:shares:recentRecipients'

type WalkPhase = 'idle' | 'discovering' | 'walking' | 'preparing' | 'submitting' | 'done'

interface ProgressState {
  current: number
  total: number
  phase: WalkPhase
  walked: number
  capExceeded: boolean
}

export function useSharingPeopleAdd(props: SharingPeopleAddProps, emit: SharingPeopleAddEmit) {
  const trusted = trustedFingerprintsStore()
  const shares = sharesStoreFactory()
  const capabilities = capabilitiesStore()

  const isDir = computed(() => props.file?.mime === 'dir')

  /**
   * Folder-editable affordance. The flag is a UI hint:
   * "share_role >= editor on a folder" is already what makes the folder
   * editable; the toggle simply communicates that promotion to the user.
   * Reader on a folder = view-only; Editor or Co-owner on a folder = can
   * upload new files via the multi-key path. There is no new persisted
   * column — the role already encodes it. Starts off matching the default
   * role (Reader = unchecked) so a Reader recipient never lands on a
   * disabled-but-checked toggle.
   */
  const folderEditable = ref(false)

  const email = ref('')
  const recipient = ref<DiscoveredUser | null>(null)
  const recipientFingerprintHex = ref('')
  const formattedFingerprint = ref('')
  const trustStatus = ref<'unknown' | 'trusted-fresh' | 'trusted-stale' | 'mismatch'>('unknown')
  const role = ref<ShareRole>('reader')
  const discoverError = ref<string | null>(null)
  const submitting = ref(false)

  const groupSuggestions = ref<ShareableGroup[]>([])
  const selectedGroup = ref<ShareableGroup | null>(null)
  const progress = ref<ProgressState>({
    current: 0,
    total: 0,
    phase: 'idle',
    walked: 0,
    capExceeded: false
  })
  const recentRecipients = ref<string[]>([])

  // Captured when the discover lookup turns up a fingerprint that
  // disagrees with the cached one — the user has to resolve the
  // mismatch through a dedicated modal before the share can proceed,
  // which is the only place we still demand an explicit acknowledgement
  // from the user.
  const mismatchPayload = ref<{
    recipientEmail: string
    cachedFingerprint: string
    newFingerprint: string
    lastVerifiedAt: number
  } | null>(null)

  let walkController: AbortController | null = null

  function loadRecentRecipients(): string[] {
    if (typeof localStorage === 'undefined') return []
    const raw = localStorage.getItem(RECENT_RECIPIENTS_KEY)
    if (!raw) return []
    try {
      const parsed = JSON.parse(raw)
      return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === 'string') : []
    } catch {
      return []
    }
  }

  function recordRecentRecipient(value: string): void {
    if (typeof localStorage === 'undefined') return
    const trimmed = value.trim().toLowerCase()
    if (!trimmed) return
    const next = [trimmed, ...recentRecipients.value.filter((e) => e !== trimmed)].slice(0, 10)
    recentRecipients.value = next
    localStorage.setItem(RECENT_RECIPIENTS_KEY, JSON.stringify(next))
  }

  function resetForm(): void {
    email.value = props.prefillEmail ?? ''
    recipient.value = null
    recipientFingerprintHex.value = ''
    formattedFingerprint.value = ''
    trustStatus.value = 'unknown'
    role.value = props.prefillRole ?? 'reader'
    folderEditable.value =
      role.value === 'reader' ? false : props.prefillAddFiles ?? true
    discoverError.value = null
    submitting.value = false
    selectedGroup.value = null
    mismatchPayload.value = null
    progress.value = { current: 0, total: 0, phase: 'idle', walked: 0, capExceeded: false }
    if (walkController) {
      walkController.abort()
      walkController = null
    }
  }

  /**
   * Groups the caller may share into: their own groups, plus any group where
   * they're an editor or co-owner (an editor+ may initiate a share into the
   * group; the per-file reshare check still applies server-side). Reader
   * member-of groups are excluded since a reader can't initiate a share. Gated
   * on the `share_groups` capability — when it's off, no group is offered.
   */
  async function loadGroupSuggestions(): Promise<void> {
    if (!capabilities.shareGroups) {
      groupSuggestions.value = []
      return
    }
    try {
      const response = await sharesApi.listGroups()
      const owned: ShareableGroup[] = response.owned.map((g) => ({
        id: g.id,
        name: g.name,
        memberCount: g.members.length
      }))
      const editable: ShareableGroup[] = response.member_of
        .filter((g) => g.group_role === 'editor' || g.group_role === 'co-owner')
        .map((g) => ({ id: g.id, name: g.name, memberCount: null }))
      groupSuggestions.value = [...owned, ...editable]
    } catch {
      groupSuggestions.value = []
    }
  }

  onMounted(() => {
    recentRecipients.value = loadRecentRecipients()
    void loadGroupSuggestions()
  })

  /**
   * When the parent prefills an email from a "Change" affordance, we want
   * the role picker to mount without forcing the user to hit Find user
   * again. The watcher kicks discover() once the prefill lands, but only
   * if the panel is genuinely idle — an in-flight subtree walk or a
   * submit must not be interrupted by a re-entrant lookup.
   *
   * `prefillRole` and `prefillAddFiles` ride alongside: when the parent
   * knows the recipient's current grant, the role picker and folder
   * add-files checkbox start at the existing values so a Change click
   * lands on the recipient's current state, not on the form default.
   */
  watch(
    () => props.prefillEmail,
    (value, previous) => {
      if (value === previous) return
      if (value === undefined) return
      email.value = value
      if (props.prefillRole) {
        role.value = props.prefillRole
      }
      folderEditable.value =
        (props.prefillRole ?? role.value) === 'reader'
          ? false
          : props.prefillAddFiles ?? true
      const trimmed = value.trim()
      if (!trimmed) return
      if (progress.value.phase !== 'idle') return
      void discover()
    }
  )

  /**
   * Reader cannot upload — surface that as an unchecked box, not a
   * disabled-but-checked one. The disabled state alone reads as "they can
   * still upload, just not on this form", which is the opposite of what
   * the role grants. Reverting back to Editor or Co-owner leaves the box
   * unchecked so the user explicitly opts back in to add-files rights.
   */
  watch(role, (value) => {
    if (value === 'reader') {
      folderEditable.value = false
    }
  })

  async function discover(): Promise<void> {
    const trimmed = email.value.trim()
    if (!trimmed) {
      discoverError.value = 'Enter the recipient email first.'
      return
    }
    // If the input matches a shareable group name exactly, switch to group
    // mode — the dialog shows the group panel and the share routes through
    // the share-to-group endpoint instead of a single-user grant.
    const groupMatch = groupSuggestions.value.find(
      (group) => group.name.toLowerCase() === trimmed.toLowerCase()
    )
    if (groupMatch) {
      selectedGroup.value = groupMatch
      recipient.value = null
      discoverError.value = null
      progress.value = { ...progress.value, phase: 'idle' }
      return
    }
    selectedGroup.value = null
    discoverError.value = null
    recipient.value = null
    recipientFingerprintHex.value = ''
    formattedFingerprint.value = ''
    trustStatus.value = 'unknown'
    progress.value = { ...progress.value, phase: 'discovering' }

    try {
      const user = await sharesApi.discoverUser(trimmed)
      if (props.ownerId && user.user_id === props.ownerId) {
        discoverError.value = 'That account already owns this file.'
        progress.value = { ...progress.value, phase: 'idle' }
        return
      }
      recipient.value = user
      const localFingerprint = shareCrypto.fingerprintForUser(user)
      recipientFingerprintHex.value = localFingerprint
      formattedFingerprint.value = shareCrypto.formatFingerprint(localFingerprint)

      // Background trust comparison. The Share button is gated only on
      // mismatch — the dedicated modal is where the loud "your contact's
      // key changed" surface lives. The other states (unknown / trusted-
      // fresh / trusted-stale) render a passive pill and let the share
      // proceed; the cached entry refreshes silently after a successful
      // submit so the next visit stays on the trusted-fresh path.
      const cached = trusted.lookup(user.user_id)
      if (cached) {
        if (cached.pubkeyFingerprint !== localFingerprint) {
          trustStatus.value = 'mismatch'
          mismatchPayload.value = {
            recipientEmail: user.email,
            cachedFingerprint: shareCrypto.formatFingerprint(cached.pubkeyFingerprint),
            newFingerprint: formattedFingerprint.value,
            lastVerifiedAt: cached.lastVerifiedAt
          }
        } else if (trusted.isStale(user.user_id)) {
          trustStatus.value = 'trusted-stale'
        } else {
          trustStatus.value = 'trusted-fresh'
        }
      } else {
        trustStatus.value = 'unknown'
      }

      recordRecentRecipient(trimmed)
    } catch (err) {
      if (err instanceof DiscoverUserError) {
        switch (err.kind) {
          case 'not_found':
            discoverError.value = "We couldn't find a Hoodik account for that email."
            break
          case 'self':
            discoverError.value = "That's your email."
            break
          case 'rate_limited':
            discoverError.value = 'Slow down — too many lookups in the last minute.'
            break
          case 'feature_disabled':
            discoverError.value = 'Sharing is currently disabled on this server.'
            break
          default:
            discoverError.value = err.message
        }
      } else {
        discoverError.value = (err as Error).message || 'Failed to discover recipient'
      }
    } finally {
      if (progress.value.phase === 'discovering') {
        progress.value = { ...progress.value, phase: 'idle' }
      }
    }
  }

  const showTrustedPill = computed(
    () => trustStatus.value === 'trusted-fresh' && recipient.value !== null
  )

  const showUnknownPill = computed(
    () =>
      recipient.value !== null &&
      (trustStatus.value === 'unknown' || trustStatus.value === 'trusted-stale')
  )

  function onMismatchAccept(): void {
    // The user has confirmed the new fingerprint out of band — record
    // it as a fresh trust entry and let the share proceed. The cached
    // entry is intentionally replaced rather than merged; a rotation
    // resets the staleness clock.
    if (!recipient.value || !recipientFingerprintHex.value) return
    trusted.trustFingerprint(
      recipient.value.user_id,
      recipientFingerprintHex.value,
      'in-person'
    )
    trustStatus.value = 'trusted-fresh'
    mismatchPayload.value = null
  }

  function onMismatchCancel(): void {
    // Bail back to the email input; the cached entry stays untouched
    // so a refusal-to-consent doesn't double as a confirmation of
    // substitution.
    recipient.value = null
    recipientFingerprintHex.value = ''
    formattedFingerprint.value = ''
    trustStatus.value = 'unknown'
    mismatchPayload.value = null
  }

  const confirmDisabled = computed(() => {
    if (props.readOnly) return true
    if (submitting.value) return true
    if (progress.value.capExceeded) return true
    if (selectedGroup.value) {
      // A group whose roster we can see (owned) and that's empty has no one
      // to share with. For member-of groups we can't see the roster, so the
      // button stays enabled and the server returns a clear empty-group error.
      return selectedGroup.value.memberCount === 0
    }
    if (!recipient.value) return true
    // Mismatch is the one state that still blocks submit; it resolves
    // through the dedicated modal, not an inline checkbox.
    if (trustStatus.value === 'mismatch') return true
    return false
  })

  const lastVerifiedLabel = computed(() => {
    if (!recipient.value) return ''
    const entry = trusted.lookup(recipient.value.user_id)
    if (!entry) return ''
    const ageSeconds = Math.floor(Date.now() / 1000) - entry.lastVerifiedAt
    const days = Math.floor(ageSeconds / (24 * 60 * 60))
    if (days <= 0) return 'Last verified today.'
    if (days === 1) return 'Last verified yesterday.'
    return `Last verified ${days} days ago.`
  })

  async function collectSubtree(): Promise<AppFile[]> {
    if (!props.file) return []
    walkController = new AbortController()
    const collected = await shareSubtree.collectSubtree(props.file, {
      signal: walkController.signal,
      onProgress: ({ walked }) => {
        progress.value = { ...progress.value, walked, phase: 'walking' }
      }
    })
    if (collected.length > SUBTREE_HARD_CAP) {
      throw new SubtreeCapExceeded(collected.length)
    }
    return collected
  }

  async function buildEntriesForAll(
    subtree: AppFile[],
    recipient: DiscoveredUser
  ): Promise<ShareEntryInput[]> {
    if (!walkController) {
      walkController = new AbortController()
    }
    progress.value = {
      ...progress.value,
      phase: 'preparing',
      current: 0,
      total: subtree.length
    }
    return shareSubtree.buildEntriesForSubtree(
      subtree,
      recipient,
      props.keypair.wrappingPrivate || (props.keypair.input as string),
      {
        signal: walkController.signal,
        onProgress: ({ current, total }) => {
          progress.value = { ...progress.value, current, total }
        }
      }
    )
  }

  async function submit(): Promise<void> {
    if (!props.file || !props.keypair.input) {
      return
    }
    if (!recipient.value && !selectedGroup.value) {
      return
    }

    if (props.disableCoOwner && role.value === 'co-owner') {
      role.value = 'editor'
    }

    submitting.value = true
    try {
      const subtree = await collectSubtree()
      const root = props.file

      if (selectedGroup.value) {
        await submitGroupShare(root, subtree, selectedGroup.value)
        return
      }

      if (!recipient.value) return
      const target = recipient.value
      const entries = await buildEntriesForAll(subtree, target)
      const created = await submitShare(root, entries, target)
      trusted.trustFingerprint(target.user_id, recipientFingerprintHex.value, 'silent')

      progress.value = { ...progress.value, phase: 'done' }
      const itemLabel = entries.length > 1 ? `${entries.length} files` : root.name
      notification(`Shared ${itemLabel} with ${target.email}`, `Role: ${role.value}`, 'success')
      emit('ok', created)
      resetForm()
    } catch (err) {
      if (err instanceof SubtreeAborted) {
        progress.value = { ...progress.value, phase: 'idle' }
      } else if (err instanceof SubtreeCapExceeded) {
        progress.value = { ...progress.value, phase: 'idle', capExceeded: true }
        notification(
          'Folder too large to share',
          `This folder has more than ${SUBTREE_HARD_CAP.toLocaleString('en-US')} files. ` +
            'Please share a sub-folder, or split the share.',
          'error'
        )
      } else {
        errorNotification(err)
      }
    } finally {
      submitting.value = false
      walkController = null
      if (progress.value.phase === 'submitting' || progress.value.phase === 'preparing') {
        progress.value = { ...progress.value, phase: 'idle' }
      }
    }
  }

  /**
   * Share the walked subtree to a group by fanning out the single-share path
   * once per group member. The service fetches the roster (owner + members with
   * pubkeys), drops the caller and the file owner, then for each remaining
   * recipient recomputes the fingerprint, reconciles it against TOFU (hard-stop
   * on mismatch, before any key is wrapped), wraps the subtree's keys, and POSTs
   * `/api/shares` — the same path the People-tab single-share uses. The group
   * endpoint returns no recipient list, so the dialog emits `ok` with an empty
   * array; the parent refetches as needed.
   */
  async function submitGroupShare(
    root: AppFile,
    subtree: AppFile[],
    group: ShareableGroup
  ): Promise<void> {
    if (!props.keypair.input) return
    progress.value = { ...progress.value, phase: 'submitting' }
    await shareGroups.shareToGroup({
      groupId: group.id,
      root,
      subtree,
      shareRole: role.value,
      senderId: props.authenticatedUserId,
      privateKey: props.keypair.input,
      wrappingPrivateKey: props.keypair.wrappingPrivate || props.keypair.input,
      trusted,
      onProgress: (done, total) => {
        progress.value = { ...progress.value, current: done, total }
      }
    })
    progress.value = { ...progress.value, phase: 'done' }
    const itemLabel = subtree.length > 1 ? `${subtree.length} files` : root.name
    notification(`Shared ${itemLabel} with ${group.name}`, `Role: ${role.value}`, 'success')
    emit('ok', [])
    resetForm()
  }

  async function submitShare(
    root: AppFile,
    entries: ShareEntryInput[],
    target: DiscoveredUser
  ): Promise<AppShare[]> {
    // The audit event the server persists depends on the recipient's
    // existing state on this root: `role_change` when they already hold a
    // different role, `shared_by_co_owner` on a Co-owner reshare, `grant`
    // on a fresh share. The server reconstructs the same canonical from its
    // own read of `user_files`, so the SPA mirrors that decision here — a
    // mismatched action or before-role surfaces as `event_signature_invalid`
    // rather than a silently NULL signature on the persisted row.
    const isRoleChange =
      props.prefillRole !== undefined &&
      target.email === props.prefillEmail &&
      role.value !== props.prefillRole
    const action: AuditEventActionWire = isRoleChange
      ? 'role_change'
      : props.disableCoOwner
        ? 'shared_by_co_owner'
        : 'grant'
    const shareRoleBefore: ShareRole | null = isRoleChange
      ? (props.prefillRole as ShareRole)
      : null

    const envelope = await shareSubtree.buildShareEnvelopeForRecipient({
      root,
      entries,
      target,
      shareRole: role.value,
      senderId: props.authenticatedUserId,
      privateKey: props.keypair.input as string,
      action,
      shareRoleBefore
    })

    progress.value = { ...progress.value, phase: 'submitting' }
    return shares.createShare(envelope)
  }

  function cancel(): void {
    if (walkController) {
      walkController.abort()
      walkController = null
    }
    resetForm()
    emit('cancel')
  }

  function abortWalk(): void {
    if (walkController) {
      walkController.abort()
    }
  }

  function fillRecent(value: string): void {
    email.value = value
  }

  const folderHint = computed(() => {
    if (!isDir.value) return ''
    const walked = progress.value.walked
    if (walked === 0) return ''
    if (walked < SUBTREE_DETERMINATE_THRESHOLD) {
      return `Discovered ${walked} files so far…`
    }
    return `Discovered ${walked.toLocaleString('en-US')} files. This may take a few seconds.`
  })

  const showFolderHint = computed(() => {
    if (progress.value.phase === 'preparing' || progress.value.phase === 'submitting') {
      return false
    }
    return folderHint.value !== ''
  })

  /**
   * Render `XXXX-XXXX-…-XXXX` so the recipient block's fingerprint stays
   * single-line at its column width. The full value lands in the `title=`
   * tooltip on the element so a curious user can hover.
   */
  const abbreviatedFormattedFingerprint = computed(() => {
    if (!formattedFingerprint.value) return ''
    if (formattedFingerprint.value.length <= 19) return formattedFingerprint.value
    return `${formattedFingerprint.value.slice(0, 10)}…-${formattedFingerprint.value.slice(-4)}`
  })

  const roleDescription = computed(() => {
    switch (role.value) {
      case 'reader':
        return 'Reader — can view only.'
      case 'editor':
        return 'Editor — can view and edit. No re-share.'
      case 'co-owner':
        return 'Co-owner — can view, edit, re-share, and save copies.'
      default:
        return ''
    }
  })

  const progressLabel = computed(() => {
    switch (progress.value.phase) {
      case 'discovering':
        return 'Looking up recipient…'
      case 'walking':
        return `Walking subtree (${progress.value.walked.toLocaleString('en-US')} files found)`
      case 'preparing':
        return `Encrypting access (${progress.value.current.toLocaleString('en-US')} / ${progress.value.total.toLocaleString('en-US')})`
      case 'submitting':
        return 'Submitting to server…'
      default:
        return ''
    }
  })

  /**
   * Short, calm status the submit overlay shows under the spinner. The
   * inline progress strip is verbose by design (file counts, cancel
   * link); the overlay collapses that to a single line so the eye lands
   * on the spinner rather than tracking shifting numbers.
   */
  const overlayStatus = computed(() => {
    if (selectedGroup.value) {
      return `Sharing with ${selectedGroup.value.name}…`
    }
    if (recipient.value) {
      return `Sharing with ${recipient.value.email}…`
    }
    return 'Sharing…'
  })

  const determinate = computed(() => {
    if (progress.value.phase !== 'preparing') return false
    return progress.value.total > 0
  })

  const indeterminateActive = computed(() => {
    return progress.value.phase === 'walking' || progress.value.phase === 'discovering'
  })

  const capMessage = computed(() => {
    if (!progress.value.capExceeded) return ''
    return (
      `This folder has more than ${SUBTREE_HARD_CAP.toLocaleString('en-US')} files. ` +
      'Please share a sub-folder, or split the share.'
    )
  })

  return {
    isDir,
    folderEditable,
    email,
    recipient,
    formattedFingerprint,
    role,
    discoverError,
    submitting,
    groupSuggestions,
    selectedGroup,
    progress,
    recentRecipients,
    mismatchPayload,
    discover,
    showTrustedPill,
    showUnknownPill,
    onMismatchAccept,
    onMismatchCancel,
    confirmDisabled,
    lastVerifiedLabel,
    submit,
    cancel,
    abortWalk,
    fillRecent,
    folderHint,
    showFolderHint,
    abbreviatedFormattedFingerprint,
    roleDescription,
    progressLabel,
    overlayStatus,
    determinate,
    indeterminateActive,
    capMessage
  }
}
