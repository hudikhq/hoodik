import * as cryptfns from '!/cryptfns'

import * as api from './api'
import * as shareCrypto from './crypto'
import {
  FolderMemberFingerprintChanged,
  FolderMemberListInvalid,
  UploadIntoSharedFolderAborted,
  type UnknownMemberPrompt,
  type UploadIntoSharedFolderProgress
} from './editable-types'

import type {
  FolderMember,
  FolderMembersResponse,
  TrustedFingerprintsStore,
  UploadMultiKeyMember
} from 'types'

const MEMBER_SIG_V1_PREFIX = new TextEncoder().encode('hoodik-folder-mem-v1\0')

const ROLE_TO_WIRE: Record<FolderMember['share_role'], number> = {
  reader: 0,
  editor: 1,
  'co-owner': 2
}

export function ensureNotAborted(signal?: AbortSignal): void {
  if (signal?.aborted) {
    throw new UploadIntoSharedFolderAborted()
  }
}

function uuidStringToBytes(uuid: string): Uint8Array {
  const hex = uuid.replace(/-/g, '')
  if (hex.length !== 32) {
    throw new Error(`Invalid UUID: ${uuid}`)
  }
  return cryptfns.uint8.fromHex(hex)
}

function concatPrefixed(prefix: Uint8Array, payload: Uint8Array): Uint8Array {
  const out = new Uint8Array(prefix.length + payload.length)
  out.set(prefix, 0)
  out.set(payload, prefix.length)
  return out
}

/**
 * Server response carries `pubkey` as a PEM string; the v1 member-sig
 * encoder wants the raw DER bytes. We decode the PEM body here so the
 * client recomputes the exact bytes the original signer covered.
 */
function pemToDerBytes(pem: string): Uint8Array {
  const trimmed = pem
    .replace(/-----BEGIN [A-Z ]+-----/g, '')
    .replace(/-----END [A-Z ]+-----/g, '')
    .replace(/\s+/g, '')
  return cryptfns.uint8.fromBase64(trimmed)
}

/**
 * Verify the per-member σ for one row using the supplied signer pubkey.
 * Returns `false` (rather than throwing) so the caller can decide whether
 * a missing/invalid signature is fatal in the current context.
 */
async function verifySingleMember(
  member: FolderMember,
  signerPubkey: string
): Promise<boolean> {
  if (!member.member_signature || member.added_at === null) {
    return false
  }
  const { member_sig_encode_v1, rsa_verify_bytes } = await import('!/cryptfns/wasm')
  const userIdBytes = uuidStringToBytes(member.user_id)
  const pubkeyDer = pemToDerBytes(member.pubkey)
  const fingerprintBytes = cryptfns.uint8.fromHex(member.pubkey_fingerprint)
  const roleByte = ROLE_TO_WIRE[member.share_role]
  const der = member_sig_encode_v1(
    userIdBytes,
    pubkeyDer,
    fingerprintBytes,
    roleByte,
    BigInt(member.added_at)
  )
  if (!der) return false
  const signingInput = concatPrefixed(MEMBER_SIG_V1_PREFIX, der)
  return rsa_verify_bytes(signingInput, member.member_signature, signerPubkey)
}

/**
 * Walk the server response and verify the list signature plus per-
 * member signatures against the authorized-signer set (folder owner
 * plus every current Co-owner whose own member record verifies against
 * the folder owner). Every failure path
 * here is a HARD STOP on the upload; there are no soft warnings.
 *
 * Legacy rows with `member_signature = null` have
 * no per-member σ on the recipient row, so the per-member check skips
 * them — the TOFU prompt in `reconcileFingerprints` then bears the
 * trust decision. The list signature itself is mandatory regardless.
 */
export async function verifyFolderMemberList(
  response: FolderMembersResponse
): Promise<void> {
  const ownerEntry = response.members.find((m) => m.user_id === response.folder_owner_id)
  if (!ownerEntry) {
    throw new FolderMemberListInvalid(
      'owner_missing',
      'Folder owner is not present in the member list — refusing to upload.',
      response.folder_owner_id
    )
  }
  if (!ownerEntry.pubkey) {
    throw new FolderMemberListInvalid(
      'owner_missing',
      'Folder owner pubkey is empty.',
      response.folder_owner_id
    )
  }

  // The owner's fingerprint is self-signed by construction; verify the
  // server-returned fingerprint matches the pubkey before trusting it as
  // the signer-set root.
  const ownerLocalFp = shareCrypto.computeFingerprint(ownerEntry.pubkey)
  if (ownerLocalFp !== ownerEntry.pubkey_fingerprint) {
    throw new FolderMemberListInvalid(
      'fingerprint_mismatch',
      'Folder owner fingerprint does not match the returned pubkey.',
      ownerEntry.user_id
    )
  }

  const signers = new Map<string, string>()
  signers.set(response.folder_owner_id, ownerEntry.pubkey)

  // Pass 1: promote Co-owners to signers if their σ_member verifies
  // against the owner. We do this in a separate pass so subsequent
  // checks of Reader/Editor members can resolve to a Co-owner signer
  // regardless of iteration order in `response.members`.
  for (const m of response.members) {
    if (m.share_role !== 'co-owner') continue
    if (m.signed_by_user_id !== response.folder_owner_id) continue
    if (!m.pubkey) continue
    const localFp = shareCrypto.computeFingerprint(m.pubkey)
    if (localFp !== m.pubkey_fingerprint) {
      throw new FolderMemberListInvalid(
        'fingerprint_mismatch',
        `Co-owner ${m.user_id} fingerprint does not match the returned pubkey.`,
        m.user_id
      )
    }
    const ok = await verifySingleMember(m, ownerEntry.pubkey)
    if (ok) {
      signers.set(m.user_id, m.pubkey)
    }
    // If a co-owner's σ is missing, they're not promoted to signer; we
    // still consider them a member, just not a delegated signer.
  }

  // List signature is mandatory — the earlier soft-warning posture is
  // gone. Missing, unauthorised, or invalid list
  // signature aborts the upload.
  if (
    !response.members_list_signature ||
    !response.members_list_signed_by_user_id ||
    response.members_signed_at === null
  ) {
    throw new FolderMemberListInvalid(
      'list_signature_missing',
      'Folder member list has no signature — refusing to upload.'
    )
  }

  const listSignerPubkey = signers.get(response.members_list_signed_by_user_id)
  if (!listSignerPubkey) {
    throw new FolderMemberListInvalid(
      'list_signature_unauthorized_signer',
      `Member list signed by ${response.members_list_signed_by_user_id}, who is not the owner or a verified Co-owner.`,
      response.members_list_signed_by_user_id
    )
  }

  const listInput = buildListInputFromResponse(response)
  const listOk = await shareCrypto.verifyFolderMemberListSignature(
    listInput,
    response.members_list_signature,
    listSignerPubkey
  )
  if (!listOk) {
    throw new FolderMemberListInvalid(
      'list_signature_invalid',
      'Folder member list signature failed verification.'
    )
  }

  // Pass 2: every member's fingerprint must match its pubkey; every
  // member with a signature must verify against a known signer. Members
  // with `member_signature = null` are legacy rows — the upload still
  // proceeds, but the fingerprint flow in `reconcileFingerprints` will
  // require explicit TOFU consent on those entries.
  for (const m of response.members) {
    if (!m.pubkey) {
      throw new FolderMemberListInvalid(
        'unknown_signer',
        `Member ${m.user_id} has no pubkey — refusing to upload.`,
        m.user_id
      )
    }
    const localFp = shareCrypto.computeFingerprint(m.pubkey)
    if (localFp !== m.pubkey_fingerprint) {
      throw new FolderMemberListInvalid(
        'fingerprint_mismatch',
        `Member ${m.user_id} fingerprint does not match the returned pubkey.`,
        m.user_id
      )
    }
    if (!m.member_signature) continue
    if (!m.signed_by_user_id) {
      throw new FolderMemberListInvalid(
        'unknown_signer',
        `Member ${m.user_id} carries a signature but no signer id.`,
        m.user_id
      )
    }
    const signerPubkey = signers.get(m.signed_by_user_id)
    if (!signerPubkey) {
      throw new FolderMemberListInvalid(
        'unknown_signer',
        `Member ${m.user_id} signed by an unknown actor.`,
        m.user_id
      )
    }
    const ok = await verifySingleMember(m, signerPubkey)
    if (!ok) {
      throw new FolderMemberListInvalid(
        'member_signature_invalid',
        `Member ${m.user_id} signature did not verify.`,
        m.user_id
      )
    }
  }
}

/**
 * Build the typed `FolderMemberListV1` from a server response. Used by
 * both the verifier (recomputes the canonical bytes) and the signing
 * path (when the caller already has the response in hand — e.g. right
 * after a mutating endpoint returned the refreshed list).
 */
function buildListInputFromResponse(response: FolderMembersResponse) {
  return shareCrypto.buildFolderMemberListInput({
    folderId: response.folder_id,
    folderOwnerId: response.folder_owner_id,
    members: response.members.map((m) => ({
      userId: m.user_id,
      pubkeyFingerprintHex: m.pubkey_fingerprint,
      shareRole: m.share_role,
      isOwner: m.is_owner,
      signedByUserId: m.signed_by_user_id ?? response.folder_owner_id
    })),
    membersSignedAt: BigInt(response.members_signed_at ?? 0)
  })
}

/**
 * Walk the member list, requesting TOFU confirmation for any member whose
 * fingerprint is new. Existing-but-different cached fingerprints throw
 * `FolderMemberFingerprintChanged`; accepted new fingerprints are written
 * back to the trusted-fingerprint cache so subsequent uploads skip the
 * prompt.
 */
export async function reconcileFingerprints(
  members: FolderMember[],
  callerUserId: string,
  trusted: TrustedFingerprintsStore,
  onUnknownMember?: (prompt: UnknownMemberPrompt) => Promise<boolean>
): Promise<void> {
  for (const m of members) {
    if (m.user_id === callerUserId) continue
    const cached = trusted.lookup(m.user_id)
    if (cached) {
      if (cached.pubkeyFingerprint !== m.pubkey_fingerprint) {
        throw new FolderMemberFingerprintChanged(
          m.user_id,
          cached.pubkeyFingerprint,
          m.pubkey_fingerprint
        )
      }
      continue
    }
    if (!onUnknownMember) {
      throw new FolderMemberListInvalid(
        'fingerprint_mismatch',
        `New member ${m.user_id} requires fingerprint confirmation before upload.`,
        m.user_id
      )
    }
    const accepted = await onUnknownMember({
      member: m,
      signedByUserId: m.signed_by_user_id
    })
    if (!accepted) {
      throw new UploadIntoSharedFolderAborted()
    }
    trusted.trustFingerprint(m.user_id, m.pubkey_fingerprint, 'other')
  }
}

export async function wrapForEveryMember(
  fileKeyHex: string,
  members: FolderMember[],
  callerUserId: string,
  signal: AbortSignal | undefined,
  onProgress: ((p: UploadIntoSharedFolderProgress) => void) | undefined
): Promise<UploadMultiKeyMember[]> {
  const total = members.length
  let wrapped = 0
  onProgress?.({ wrappedKeys: 0, totalKeys: total, phase: 'wrapping-keys' })
  const out: UploadMultiKeyMember[] = []
  for (const m of members) {
    ensureNotAborted(signal)
    const encryptedKey = await shareCrypto.wrapForRecipient(fileKeyHex, m.pubkey)
    out.push({
      user_id: m.user_id,
      encrypted_key: encryptedKey,
      is_owner_of_file: m.user_id === callerUserId
    })
    wrapped += 1
    onProgress?.({ wrappedKeys: wrapped, totalKeys: total, phase: 'wrapping-keys' })
  }
  return out
}

/**
 * Verify the roster signature chain and reconcile every member fingerprint
 * against the caller's trust cache. Both checks HARD-STOP on failure — this
 * pair runs before the initial submit and again on the membership-changed
 * retry, so it lives in one place.
 */
export async function verifyAndReconcile(
  response: FolderMembersResponse,
  callerUserId: string,
  trusted: TrustedFingerprintsStore,
  onUnknownMember?: (prompt: UnknownMemberPrompt) => Promise<boolean>
): Promise<void> {
  await verifyFolderMemberList(response)
  await reconcileFingerprints(response.members, callerUserId, trusted, onUnknownMember)
}

/**
 * Run a roster-bound submit, retrying once if the server reports the
 * membership changed under us (`409 share_membership_changed`). On conflict
 * the fresh roster is re-verified and re-reconciled before the single retry;
 * any other error — and a second conflict — propagates unchanged.
 */
export async function withMembershipRetry<T>(
  initial: FolderMembersResponse,
  reverify: (snapshot: FolderMembersResponse) => Promise<void>,
  submit: (snapshot: FolderMembersResponse) => Promise<T>
): Promise<T> {
  try {
    return await submit(initial)
  } catch (err) {
    if (err instanceof api.ShareMembershipChangedError) {
      const refreshed = err.currentMembers
      await reverify(refreshed)
      return submit(refreshed)
    }
    throw err
  }
}
