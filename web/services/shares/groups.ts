import { ErrorResponse } from '!/api'

import * as api from './api'
import * as shareCrypto from './crypto'
import * as subtree from './subtree'

import type {
  AddGroupMemberBody,
  AppFile,
  AppShareGroup,
  DiscoveredUser,
  GroupMemberWithKey,
  GroupRole,
  KeyTransitionRow,
  ShareRole,
  TrustedFingerprintsStore
} from 'types'

/**
 * Raised when a group member's server-returned pubkey does not hash to the
 * fingerprint the server claims for it, or disagrees with a fingerprint the
 * caller already trusted (TOFU). Either case is a HARD STOP — the file key is
 * never wrapped under the unverified pubkey, so a malicious server can't
 * substitute its own key to read a share fanned out to the group. Mirrors the
 * single-share path's mismatch handling in `SharingPeopleAdd.vue`.
 */
export class GroupMemberFingerprintMismatch extends Error {
  readonly reason: 'pubkey_mismatch' | 'trust_changed'
  readonly userId: string
  readonly email: string

  constructor(
    reason: GroupMemberFingerprintMismatch['reason'],
    userId: string,
    email: string
  ) {
    super(
      reason === 'pubkey_mismatch'
        ? `Group member ${email} key does not match its fingerprint — refusing to share.`
        : `Group member ${email} key changed since you last verified it — refusing to share.`
    )
    this.name = 'GroupMemberFingerprintMismatch'
    this.reason = reason
    this.userId = userId
    this.email = email
  }
}

/**
 * Recompute the member's fingerprint from the returned pubkey and reconcile it
 * against the TOFU trust store. The local fingerprint — never the
 * server-supplied `member.fingerprint` — is what the wrap and signatures bind
 * to, so a server that lies about a member's key is caught here before any key
 * is wrapped. First sight records the recomputed fingerprint; a disagreement
 * with a trusted entry hard-stops unless a verified key-transition chain
 * connects them. Returns the verified fingerprint.
 */
async function verifyMemberFingerprint(
  member: GroupMemberWithKey,
  trusted: TrustedFingerprintsStore
): Promise<string> {
  const localFingerprint = shareCrypto.fingerprintForUser(member)
  if (localFingerprint !== member.fingerprint) {
    throw new GroupMemberFingerprintMismatch('pubkey_mismatch', member.user_id, member.email)
  }
  const cached = trusted.lookup(member.user_id)
  if (cached && cached.pubkeyFingerprint !== localFingerprint) {
    // A key transition chain linking the previously trusted fingerprint to
    // the current one explains the change (post-migration continuity) — but
    // only after every hop's certificate verifies. The rows come from the
    // server, so an unverified chain is exactly how a hostile server would
    // substitute its own key for a contact's.
    let transitioned = false
    try {
      const chain = (await api.getKeyTransitions(member.user_id)) as KeyTransitionRow[]
      transitioned = await shareCrypto.verifyTransitionChain(
        member.user_id,
        chain,
        cached.pubkeyFingerprint,
        localFingerprint
      )
    } catch {
      /* best effort; if chain fetch fails we fall through to hard mismatch */
    }
    if (transitioned) {
      trusted.trustFingerprint(member.user_id, localFingerprint, 'key-transition')
      return localFingerprint
    }
    throw new GroupMemberFingerprintMismatch('trust_changed', member.user_id, member.email)
  }
  if (!cached) {
    trusted.trustFingerprint(member.user_id, localFingerprint, 'silent')
  }
  return localFingerprint
}

/**
 * True for the server's "this recipient already holds the file" rejections —
 * the file owner (`cannot_share_owner_row`) or the caller themselves
 * (`cannot_share_with_self`). The roster filter already drops both, but a row
 * opened from inside a shared folder (loaded via the storage listing) carries
 * the caller's id in `user_id` rather than the owner's, so a co-owner fanning
 * out can still reach the owner. The fan-out skips these and keeps going;
 * every other error aborts so a real failure is never swallowed.
 */
function isAlreadyHasFileRejection(err: unknown): boolean {
  return (
    err instanceof ErrorResponse &&
    err.status === 400 &&
    (err.body?.message === 'cannot_share_owner_row' ||
      err.body?.message === 'cannot_share_with_self')
  )
}

export interface AddMemberArgs {
  groupId: string
  recipient: DiscoveredUser
  groupRole: GroupRole
}

/**
 * Add a member to a group. A group is a saved recipient selection, so this is
 * a plain roster insert — no file keys move. Replay protection rides on the
 * timestamp + nonce the body carries.
 */
export async function addMember(args: AddMemberArgs): Promise<void> {
  const body: AddGroupMemberBody = {
    user_id: args.recipient.user_id,
    pubkey_fingerprint: args.recipient.fingerprint,
    group_role: args.groupRole,
    timestamp: Math.floor(Date.now() / 1000),
    nonce: shareCrypto.toBase64Nonce(shareCrypto.randomNonce())
  }
  await api.addGroupMember(args.groupId, body)
}

/**
 * Create a new group owned by the caller. Thin wrapper over the API endpoint;
 * the dialog calls this then optionally chains `addMember` for each initial
 * member.
 */
export async function createGroup(name: string): Promise<AppShareGroup> {
  return api.createGroup({ name })
}

export interface ShareToGroupArgs {
  /** Group being shared into. */
  groupId: string
  /** Root file/folder of the subtree to share. */
  root: AppFile
  /** Full subtree (root + descendants), already walked + cap-checked by the
   *  caller so the fan-out doesn't re-walk per member. */
  subtree: AppFile[]
  /** File role every recipient receives on the shared subtree. */
  shareRole: ShareRole
  /** Caller's id (the sender of every grant). */
  senderId: string
  /** Caller's signing (identity) private key — signs each recipient's share
   *  envelope. On a curve25519 account this is the Ed25519 key. */
  privateKey: string
  /** Caller's wrapping private key — unwraps each node's file key before
   *  re-wrapping it for each recipient. Equal to `privateKey` on legacy RSA
   *  accounts; the hybrid wrapping key on curve25519 accounts. */
  wrappingPrivateKey: string
  /** TOFU trust store. Each recipient's recomputed fingerprint is reconciled
   *  against it before any key is wrapped; first sight records, a known
   *  mismatch hard-stops that recipient. */
  trusted: TrustedFingerprintsStore
  /** Reports completed recipients out of the total the fan-out will reach, for
   *  per-recipient progress in the dialog. */
  onProgress?: (done: number, total: number) => void
}

/**
 * Share a file/folder subtree to every member of a group by fanning out the
 * existing single-share path once per recipient. Fetches the group roster
 * (owner + members, each with their pubkey) in one call, drops the caller and
 * the file owner (they already hold the file), then for each remaining
 * recipient: recomputes the fingerprint and reconciles it against TOFU
 * (hard-stop on mismatch, before any key is wrapped), wraps the subtree's file
 * keys under the verified pubkey, builds the signed share envelope, and POSTs
 * `/api/shares`. Single-share is an idempotent upsert, so a mid-fan-out
 * failure is retry-safe. If a recipient turns out to already hold the file
 * (the owner reached through a row that carries the caller's id, or the
 * caller themselves), the server rejects just that grant and the fan-out
 * skips it and continues — see [`isAlreadyHasFileRejection`].
 */
export async function shareToGroup(args: ShareToGroupArgs): Promise<void> {
  const roster = await api.groupMembers(args.groupId)
  const recipients = roster.filter(
    (m) => m.user_id !== args.senderId && m.user_id !== args.root.user_id
  )
  if (recipients.length === 0) {
    throw new Error('This group has no one else to share with yet.')
  }

  // A recipient already holding the root file at a different role makes the
  // server resolve the audit action to `role_change` (with the existing role
  // as the before-value) and verify the event signature against THAT — so the
  // envelope has to be signed for the right transition or the whole fan-out
  // 400s on `event_signature_invalid`. One owner-side recipient fetch keyed on
  // the root file (the same key the server's audit canonical uses) covers every
  // member without an N+1, mirroring the single-share People-tab path.
  const existingRoles = new Map<string, ShareRole>(
    (await api.getShareRecipients(args.root.id)).map((r) => [r.recipient_id, r.share_role])
  )

  const total = recipients.length
  args.onProgress?.(0, total)
  for (let i = 0; i < recipients.length; i++) {
    const member = recipients[i]
    const existingRole = existingRoles.get(member.user_id) ?? null
    // Mirror the server's resolution: it treats the action as `role_change`
    // only when the recipient's existing root role actually moves. A same-role
    // re-share reconstructs to a `grant`, so signing `role_change` for it would
    // 400. (The server applies the same `share_role != requested` filter.)
    const isRoleChange = existingRole !== null && existingRole !== args.shareRole
    if (existingRole === args.shareRole && args.subtree.length === 1) {
      // A single-file re-share at the same role is a pure no-op server-side, so
      // skip it to keep the audit log clean. A folder is never skipped on the
      // root role alone — descendants the recipient doesn't hold yet still need
      // their wraps, and the server inserts those per file under a `grant`.
      args.onProgress?.(i + 1, total)
      continue
    }
    const verifiedFingerprint = await verifyMemberFingerprint(member, args.trusted)
    const target: DiscoveredUser = {
      user_id: member.user_id,
      email: member.email,
      pubkey: member.pubkey,
      fingerprint: verifiedFingerprint,
      key_type: member.key_type,
      wrapping_pubkey: member.wrapping_pubkey
    }
    const entries = await subtree.buildEntriesForSubtree(
      args.subtree,
      target,
      args.wrappingPrivateKey
    )
    const envelope = await subtree.buildShareEnvelopeForRecipient({
      root: args.root,
      entries,
      target,
      shareRole: args.shareRole,
      senderId: args.senderId,
      privateKey: args.privateKey,
      action: isRoleChange
        ? 'role_change'
        : args.root.is_owner
          ? 'grant'
          : 'shared_by_co_owner',
      shareRoleBefore: isRoleChange ? existingRole : null
    })
    try {
      await api.createShare(envelope)
    } catch (err) {
      if (!isAlreadyHasFileRejection(err)) throw err
    }
    args.onProgress?.(i + 1, total)
  }
}
