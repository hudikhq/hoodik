import * as cryptfns from '!/cryptfns'
import {
  audit_event_encode_v1,
  audit_event_sig_input_encode_v1,
  entries_encode_v1,
  folder_member_list_encode_v1,
  member_sig_encode_v1,
  rsa_fingerprint_public,
  rsa_sign_bytes,
  rsa_verify_bytes,
  share_payload_encode_v1,
  spki_fingerprint
} from '!/cryptfns/wasm'

import type {
  AuditEventActionWire,
  AuditEventSigInputV1,
  FolderMemberListV1,
  FolderMemberListMemberInput,
  KeyTransitionRef,
  ShareEntryInput,
  ShareEvent,
  ShareRequestPayloadV1,
  ShareRole
} from 'types'

const SHARE_REQUEST_V1_PREFIX = new TextEncoder().encode('hoodik-share-v1\0')
const AUDIT_EVENT_V1_PREFIX = new TextEncoder().encode('hoodik-audit-v1\0')
const AUDIT_EVENT_SIG_V1_PREFIX = new TextEncoder().encode('hoodik-audit-sig-v1\0')
const FOLDER_LIST_V1_PREFIX = new TextEncoder().encode('hoodik-folder-list-v1\0')
const MEMBER_SIG_V1_PREFIX = new TextEncoder().encode('hoodik-folder-mem-v1\0')

const SHARE_ROLE_WIRE: Record<ShareRole, number> = {
  reader: 0,
  editor: 1,
  'co-owner': 2
}

const AUDIT_ACTION_WIRE: Record<AuditEventActionWire, number> = {
  grant: 0,
  revoke: 1,
  role_change: 2,
  shared_folder_upload: 3,
  fork: 4,
  shared_by_co_owner: 5,
  shared_folder_edit: 6,
  shared_folder_restore: 7,
  shared_folder_evict: 8,
  shared_folder_move_out: 9
}

/**
 * `255` is the documented sentinel for "absent" role bytes in the Rust
 * encoder — `audit_event_sig_input_encode_v1` collapses it to `None` before
 * building the DER struct. Mirroring the constant here keeps the contract
 * explicit and grep-able when readers compare the JS and Rust sides.
 */
const WIRE_ROLE_ABSENT = 0xff

function roleBeforeAfterWire(role: ShareRole | null): number {
  return role === null ? WIRE_ROLE_ABSENT : SHARE_ROLE_WIRE[role]
}

function concatPrefixed(prefix: Uint8Array, payload: Uint8Array): Uint8Array {
  const out = new Uint8Array(prefix.length + payload.length)
  out.set(prefix, 0)
  out.set(payload, prefix.length)
  return out
}

function uuidStringToBytes(uuid: string): Uint8Array {
  const hex = uuid.replace(/-/g, '')
  if (hex.length !== 32) {
    throw new Error(`Invalid UUID: ${uuid}`)
  }
  return cryptfns.uint8.fromHex(hex)
}

/**
 * Strip PEM armor and decode the base64-encoded DER body. The WASM
 * encoders take raw DER bytes, and the server's member-signature
 * canonical defines `pubkey_der` as exactly the body of the stored PEM —
 * the PKCS#1 body for RSA accounts, the SPKI body for curve25519
 * accounts — so decoding the armor generically produces the right
 * canonical bytes for both key types.
 */
function pemToDerBytes(pem: string): Uint8Array {
  const trimmed = pem
    .replace(/-----BEGIN [A-Z ]+-----/g, '')
    .replace(/-----END [A-Z ]+-----/g, '')
    .replace(/\s+/g, '')
  return cryptfns.uint8.fromBase64(trimmed)
}

/**
 * The key material the SPA holds about another user, as the server
 * serialises it on member, audit, and discovery records. `key_type`
 * absent means an RSA account; `"curve25519"` accounts sign with their
 * Ed25519 identity `pubkey` and receive key wraps under their hybrid
 * `wrapping_pubkey`.
 */
export interface SignerKey {
  pubkey: string
  key_type?: string
  /**
   * The signer's single key rotation, when the response carries it. A
   * signature made before the signer rotated verifies under the old key, not
   * the current one — the verifiers here fall back through it. Mirrors the
   * server's read-path chain resolution.
   */
  key_transition?: KeyTransitionRef
}

export interface RecipientKey extends SignerKey {
  wrapping_pubkey?: string | null
}

async function verifyAgainstKey(
  signingInput: Uint8Array,
  signature: string,
  pubkey: string,
  keyType: string | undefined
): Promise<boolean> {
  if (keyType === 'curve25519') {
    return cryptfns.ed25519.verifyBytes(signingInput, signature, pubkey)
  }
  return rsa_verify_bytes(signingInput, signature, pubkey)
}

/**
 * Verify against the signer's current key, falling back to their pre-rotation
 * key when the response carries a `key_transition`. The fallback runs the
 * caller-supplied `oldKeyInput` builder so a canonical that embeds the signer's
 * own fingerprint (the folder roster) can re-encode it under the old key; audit
 * canonicals embed no fingerprint and pass `signingInput` unchanged.
 *
 * A supplied-but-failing transition returns `false` — it never falls through to
 * accept. Absent transition is current-key-only.
 */
async function verifyBytesForSigner(
  signingInput: Uint8Array,
  signature: string,
  signer: SignerKey,
  oldKeyInput?: (transition: KeyTransitionRef) => Uint8Array
): Promise<boolean> {
  if (await verifyAgainstKey(signingInput, signature, signer.pubkey, signer.key_type)) {
    return true
  }
  const transition = signer.key_transition
  if (!transition) {
    return false
  }
  const oldInput = oldKeyInput ? oldKeyInput(transition) : signingInput
  return verifyAgainstKey(oldInput, signature, transition.old_key_pem, transition.old_key_type)
}

function isCurvePrivate(pem: string): boolean {
  // Only RSA keys carry "RSA" in their PEM armor; the Ed25519 identity key and
  // the hybrid wrapping key do not. Testing for the absence of "RSA" — the same
  // discriminator `isCurveKey` uses — avoids depending on the exact label, which
  // is what broke this when the wrapping key gained its own HOODIK WRAPPING armor.
  return !!pem && !pem.toUpperCase().includes('RSA')
}

/**
 * Random 16-byte nonce suitable for `ShareRequestPayloadV1`. Uses
 * `crypto.getRandomValues` directly so the result is unbiased.
 */
export function randomNonce(): Uint8Array {
  const nonce = new Uint8Array(16)
  crypto.getRandomValues(nonce)
  return nonce
}

/**
 * Base64-encode a nonce for the group-write bodies. The single-share path
 * embeds its nonce in the signed DER and never serialises it on its own;
 * the group add-member and share-to-group bodies carry the nonce as a
 * standalone base64 field the server decodes to 16 bytes.
 */
export function toBase64Nonce(nonce: Uint8Array): string {
  return cryptfns.uint8.toBase64(nonce)
}

/**
 * Decrypt the caller's own wrap of a file key.
 * For legacy RSA accounts this is an RSA private-key operation (hex inside).
 * For curve25519 accounts the stored value is a hybrid wrap blob; we unwrap
 * with the wrapping private and return hex so callers are unchanged.
 */
export async function decryptOwnFileKey(
  encryptedKey: string,
  privateKey: string
): Promise<string> {
  if (isCurvePrivate(privateKey)) {
    const keyBytes = await cryptfns.wrapping.unwrap(encryptedKey, privateKey)
    return cryptfns.uint8.toHex(keyBytes)
  }
  return cryptfns.rsa.decryptMessage(privateKey, encryptedKey)
}

/**
 * Wrap a file key for a recipient. RSA accounts (the default when
 * `key_type` is absent) encrypt the key's HEX STRING — the format every
 * stored wrap has used since v1; curve25519 accounts seal the RAW key
 * BYTES in a hybrid wrap blob under `wrapping_pubkey`. Both come back
 * base64 — the encoding the server stores in `user_files.encrypted_key`.
 */
export async function wrapForRecipient(
  fileKeyHex: string,
  recipient: RecipientKey
): Promise<string> {
  if (recipient.key_type === 'curve25519') {
    if (!recipient.wrapping_pubkey) {
      throw new Error('curve25519 recipient has no wrapping pubkey')
    }
    return cryptfns.wrapping.wrap(cryptfns.uint8.fromHex(fileKeyHex), recipient.wrapping_pubkey)
  }
  return cryptfns.rsa.encryptMessage(fileKeyHex, recipient.pubkey)
}

/**
 * `sha256(asn1_der(SortedEntries))` — the bytes covered by `entries_hash`
 * inside `ShareRequestPayloadV1`. Sorting and DER encoding happen inside
 * the WASM helper so JS and Rust produce identical bytes given identical
 * inputs.
 */
export async function computeEntriesHash(entries: ShareEntryInput[]): Promise<Uint8Array> {
  if (entries.length === 0) {
    throw new Error('Cannot hash an empty entries list')
  }

  const fileIds = new Uint8Array(entries.length * 16)
  const encryptedKeys: Uint8Array[] = []
  const lengths = new Uint32Array(entries.length)

  for (let i = 0; i < entries.length; i++) {
    const fileIdBytes = uuidStringToBytes(entries[i].file_id)
    fileIds.set(fileIdBytes, i * 16)

    const encryptedKeyBytes = cryptfns.uint8.fromBase64(entries[i].encrypted_key)
    encryptedKeys.push(encryptedKeyBytes)
    lengths[i] = encryptedKeyBytes.length
  }

  const totalKeyBytes = encryptedKeys.reduce((sum, key) => sum + key.length, 0)
  const flat = new Uint8Array(totalKeyBytes)
  let cursor = 0
  for (const key of encryptedKeys) {
    flat.set(key, cursor)
    cursor += key.length
  }

  const derBytes = entries_encode_v1(fileIds, flat, lengths)
  if (!derBytes) {
    throw new Error('Failed to DER-encode entries — invalid input')
  }

  const digestHex = cryptfns.sha256.digest(derBytes)
  return cryptfns.uint8.fromHex(digestHex)
}

/**
 * DER-encode `ShareRequestPayloadV1` and sign
 * `b"hoodik-share-v1\0" || payload_der`. Returns base64 strings ready to
 * drop into the JSON envelope.
 */
export async function signSharePayload(
  payload: ShareRequestPayloadV1,
  privateKey: string
): Promise<{ payloadDer: string; signature: string }> {
  const payloadDer = share_payload_encode_v1(
    payload.senderId,
    payload.recipientId,
    payload.recipientPubkeyFingerprint,
    SHARE_ROLE_WIRE[payload.shareRole],
    payload.rootFileId,
    payload.entriesHash,
    payload.timestamp,
    payload.nonce
  )
  if (!payloadDer) {
    throw new Error('Failed to DER-encode ShareRequestPayloadV1')
  }

  const signingInput = concatPrefixed(SHARE_REQUEST_V1_PREFIX, payloadDer)
  const signature = isCurvePrivate(privateKey)
    ? await cryptfns.ed25519.signBytes(signingInput, privateKey)
    : rsa_sign_bytes(signingInput, privateKey)
  if (!signature) {
    throw new Error('Failed to sign ShareRequestPayloadV1')
  }

  return {
    payloadDer: cryptfns.uint8.toBase64(payloadDer),
    signature
  }
}

/**
 * DER-encode an `AuditEventSigInputV1` and sign with the audit-event
 * domain-prefix. Returns the base64-encoded RSA-PSS signature.
 */
export async function signAuditEvent(
  input: AuditEventSigInputV1,
  privateKey: string
): Promise<string> {
  const der = audit_event_sig_input_encode_v1(
    input.senderId,
    input.recipientId ?? new Uint8Array(0),
    input.fileId,
    AUDIT_ACTION_WIRE[input.action],
    roleBeforeAfterWire(input.shareRoleBefore),
    roleBeforeAfterWire(input.shareRoleAfter),
    input.timestamp
  )
  if (!der) {
    throw new Error('Failed to DER-encode AuditEventSigInputV1')
  }

  const signingInput = concatPrefixed(AUDIT_EVENT_SIG_V1_PREFIX, der)
  const signature = isCurvePrivate(privateKey)
    ? await cryptfns.ed25519.signBytes(signingInput, privateKey)
    : rsa_sign_bytes(signingInput, privateKey)
  if (!signature) {
    throw new Error('Failed to sign AuditEventSigInputV1')
  }
  return signature
}

/**
 * Verify an `AuditEventSigInputV1` signature against a known sender key.
 * Used by the audit-log chain verifier to confirm rows weren't forged.
 */
export async function verifyAuditEvent(
  input: AuditEventSigInputV1,
  signature: string,
  sender: SignerKey
): Promise<boolean> {
  const der = audit_event_sig_input_encode_v1(
    input.senderId,
    input.recipientId ?? new Uint8Array(0),
    input.fileId,
    AUDIT_ACTION_WIRE[input.action],
    roleBeforeAfterWire(input.shareRoleBefore),
    roleBeforeAfterWire(input.shareRoleAfter),
    input.timestamp
  )
  if (!der) {
    return false
  }
  const signingInput = concatPrefixed(AUDIT_EVENT_SIG_V1_PREFIX, der)
  return verifyBytesForSigner(signingInput, signature, sender)
}

/**
 * Sign a single folder-member record. The granting actor (folder owner
 * for fresh grants, Co-owner for reshares) commits to the recipient's
 * pubkey, fingerprint, and share role at the moment the grant is
 * issued, producing a `MemberSigPayloadV1`.
 *
 * The result populates `user_files.member_signature` so any later
 * member-list verifier (the SPA's `verifyFolderMemberList`) can chain
 * trust from the folder owner through the named signer to this entry.
 * Returns the base64-encoded RSA-PSS signature.
 */
export async function signMember(
  args: {
    userId: string
    pubkeyPem: string
    pubkeyFingerprintHex: string
    shareRole: ShareRole
    signedAt: bigint
  },
  signerPrivateKey: string
): Promise<string> {
  const der = member_sig_encode_v1(
    uuidStringToBytes(args.userId),
    pemToDerBytes(args.pubkeyPem),
    cryptfns.uint8.fromHex(args.pubkeyFingerprintHex),
    SHARE_ROLE_WIRE[args.shareRole],
    args.signedAt
  )
  if (!der) {
    throw new Error('Failed to DER-encode MemberSigPayloadV1')
  }
  const signingInput = concatPrefixed(MEMBER_SIG_V1_PREFIX, der)
  const signature = isCurvePrivate(signerPrivateKey)
    ? await cryptfns.ed25519.signBytes(signingInput, signerPrivateKey)
    : rsa_sign_bytes(signingInput, signerPrivateKey)
  if (!signature) {
    throw new Error('Failed to sign MemberSigPayloadV1')
  }
  return signature
}

/**
 * Verify a single member's σ against the named signer's pubkey. Pure
 * mirror of `signMember`; used by both `verifyFolderMemberList` (the
 * SPA's hard-fail verifier on the upload path) and by tests that
 * round-trip the producer + verifier without going through the server.
 */
export async function verifyMemberSignature(
  args: {
    userId: string
    pubkeyPem: string
    pubkeyFingerprintHex: string
    shareRole: ShareRole
    signedAt: bigint
  },
  signature: string,
  signer: SignerKey
): Promise<boolean> {
  const der = member_sig_encode_v1(
    uuidStringToBytes(args.userId),
    pemToDerBytes(args.pubkeyPem),
    cryptfns.uint8.fromHex(args.pubkeyFingerprintHex),
    SHARE_ROLE_WIRE[args.shareRole],
    args.signedAt
  )
  if (!der) return false
  const signingInput = concatPrefixed(MEMBER_SIG_V1_PREFIX, der)
  return verifyBytesForSigner(signingInput, signature, signer)
}

/**
 * `sha256(hex(modulus))` for an RSA public key. Same algorithm the server
 * uses for `users.fingerprint`; we re-derive client-side so the UX never
 * has to trust an unverified server claim about the recipient's identity.
 */
export function computeFingerprint(pubkey: string): string {
  const fingerprint = rsa_fingerprint_public(pubkey)
  if (!fingerprint) {
    throw new Error('Failed to compute pubkey fingerprint')
  }
  return fingerprint
}

/**
 * Fingerprint dispatch across account key types — mirrors the server's
 * `identity::KeyType::fingerprint`. RSA accounts hash the key modulus
 * (`computeFingerprint`); curve25519 accounts hash the Ed25519 SPKI body,
 * exactly what registration stored in `users.fingerprint`.
 */
export function fingerprintForUser(user: SignerKey): string {
  if (user.key_type === 'curve25519') {
    const fingerprint = spki_fingerprint(user.pubkey)
    if (!fingerprint) {
      throw new Error('Failed to compute pubkey fingerprint')
    }
    return fingerprint
  }
  return computeFingerprint(user.pubkey)
}

/**
 * Quad-group rendering for fingerprint display:
 *   `abcd1234ef...` → `ABCD-1234-EFAB-...`
 *
 * Insertion is byte-aligned so the output is stable for any fingerprint
 * length — the client never relies on a particular total length.
 */
export function formatFingerprint(hexFp: string): string {
  const upper = hexFp.toUpperCase()
  const chunks: string[] = []
  for (let i = 0; i < upper.length; i += 4) {
    chunks.push(upper.slice(i, i + 4))
  }
  return chunks.join('-')
}

/**
 * Build a `ShareRequestPayloadV1` from typed string inputs. The server's
 * fingerprint column stores SHA-256 hex (64 chars) — we accept that exact
 * format and convert to bytes here so call sites don't have to.
 */
export function buildSharePayload(args: {
  senderId: string
  recipientId: string
  recipientPubkeyFingerprintHex: string
  shareRole: ShareRole
  rootFileId: string
  entriesHash: Uint8Array
  timestamp: bigint
  nonce: Uint8Array
}): ShareRequestPayloadV1 {
  return {
    senderId: uuidStringToBytes(args.senderId),
    recipientId: uuidStringToBytes(args.recipientId),
    recipientPubkeyFingerprint: cryptfns.uint8.fromHex(args.recipientPubkeyFingerprintHex),
    shareRole: args.shareRole,
    rootFileId: uuidStringToBytes(args.rootFileId),
    entriesHash: args.entriesHash,
    timestamp: args.timestamp,
    nonce: args.nonce
  }
}

/**
 * Build an `AuditEventSigInputV1` from typed string inputs. Used by both
 * the share-create and revoke flows on the client.
 */
export function buildAuditEventSigInput(args: {
  senderId: string
  recipientId: string | null
  fileId: string
  action: AuditEventActionWire
  shareRoleBefore: ShareRole | null
  shareRoleAfter: ShareRole | null
  timestamp: bigint
}): AuditEventSigInputV1 {
  return {
    senderId: uuidStringToBytes(args.senderId),
    recipientId: args.recipientId ? uuidStringToBytes(args.recipientId) : null,
    fileId: uuidStringToBytes(args.fileId),
    action: args.action,
    shareRoleBefore: args.shareRoleBefore,
    shareRoleAfter: args.shareRoleAfter,
    timestamp: args.timestamp
  }
}

/**
 * Recompute one row's chain hash. Mirrors the server-side rule
 * `sha256(b"hoodik-audit-v1\0" || prev_hash || encode_audit_event_v1(row))`.
 * System-cascade rows (NULL sender) chain among
 * themselves; the previous hash for the first row on a chain is 32
 * zero bytes.
 */
export function recomputeChainHash(row: ShareEvent, prevHashB64: string | null): string {
  const senderBytes = row.sender_id ? uuidStringToBytes(row.sender_id) : new Uint8Array(16)
  const recipientBytes = row.recipient_id
    ? uuidStringToBytes(row.recipient_id)
    : new Uint8Array(16)
  const fileBytes = uuidStringToBytes(row.file_id)
  const wireRole = row.share_role_after === null ? WIRE_ROLE_ABSENT : SHARE_ROLE_WIRE[row.share_role_after]
  const der = audit_event_encode_v1(
    senderBytes,
    recipientBytes,
    fileBytes,
    row.action,
    wireRole,
    BigInt(row.created_at)
  )
  if (!der) {
    throw new Error('Failed to DER-encode AuditEventRowV1')
  }
  const prev = prevHashB64 ? cryptfns.uint8.fromBase64(prevHashB64) : new Uint8Array(32)
  const input = new Uint8Array(AUDIT_EVENT_V1_PREFIX.length + prev.length + der.length)
  input.set(AUDIT_EVENT_V1_PREFIX, 0)
  input.set(prev, AUDIT_EVENT_V1_PREFIX.length)
  input.set(der, AUDIT_EVENT_V1_PREFIX.length + prev.length)
  const digestHex = cryptfns.sha256.digest(input)
  return cryptfns.uint8.toBase64(cryptfns.uint8.fromHex(digestHex))
}

/**
 * Per-row classification of a chain walk. Surfaces both whether the
 * row's chain status is acceptable (`linked` or `page-boundary` both are)
 * and the precise failure mode when it isn't (`self-hash` for content
 * tampering, `link-broken` for a forged or rewritten link between two
 * visible rows). The disclosure / banner copy renders different language
 * per state, so collapsing this to a boolean drops information the user
 * needs.
 */
export type ChainRowStatus =
  | 'linked'
  | 'page-boundary'
  | 'self-hash-mismatch'
  | 'link-broken'

/**
 * Result of walking the per-sender chain over a page of events. Indexes
 * map 1:1 to the input array. Verification is order-aware: events are
 * grouped by sender (and the NULL-sender bucket) and walked oldest-first,
 * because the server-side chain is appended in time order.
 */
export interface ChainVerification {
  /** True when the row's chain status is `linked` or `page-boundary` —
   * the row passes self-hash AND either links cleanly to an in-page
   * predecessor or has a predecessor outside the loaded slice. */
  chainOk: boolean[]
  /** Per-row chain classification — see `ChainRowStatus`. */
  rowStatus: ChainRowStatus[]
  /** Index of the first row that fails chain verification, or -1 when
   * the whole loaded slice is consistent. */
  firstBreakIndex: number
}

/**
 * Walk the per-sender chain over a page of events. Events are expected
 * in newest-first order (which is how `GET /api/shares/events` returns
 * them); the walker re-orders by `created_at, id` ascending inside each
 * sender bucket so the chain compares against the right `prev_hash`.
 *
 * **Slice-aware contract.** The audit chain is per-sender across the
 * WHOLE table, but the events endpoint serves a viewer-scoped slice (rows
 * the caller authored, targets, or owns the file for). Mid-bucket
 * predecessors can legitimately fall outside the slice — the most common
 * case is one Co-owner reshare interleaved with another Co-owner's
 * revoke, where the recipient sees only their own grants and not the
 * sibling's revokes. The verifier classifies every row as one of:
 *
 *   - **page-boundary**: `prev_event_hash` does not match the
 *     `this_event_hash` of any earlier in-page row in the same bucket.
 *     Treat the row as a fresh chain head — self-verify the hash against
 *     the row's own stored `prev_event_hash`, skip the link-check.
 *   - **linked**: `prev_event_hash` matches an earlier in-page row's
 *     hash. Strict link-check: row's recomputed hash must equal the
 *     stored hash AND the row's `prev_event_hash` must equal the
 *     predecessor's `this_event_hash`.
 *
 * Tightened security claim: the badge fires only when (a) a row's self
 * hash fails, OR (b) two visible adjacent rows in the same bucket
 * disagree on prev_hash. This catches deletion + naive-forge attacks
 * against any pair of contiguously visible rows; the prior strict-
 * after-loaded behaviour produced false positives whenever a bucket's
 * mid-chain predecessor was filtered out by visibility.
 */
export function verifyChain(events: ShareEvent[]): ChainVerification {
  const chainOk = new Array<boolean>(events.length).fill(false)
  const rowStatus = new Array<ChainRowStatus>(events.length).fill('self-hash-mismatch')
  if (events.length === 0) {
    return { chainOk, rowStatus, firstBreakIndex: -1 }
  }

  const byBucket = new Map<string, number[]>()
  for (let i = 0; i < events.length; i++) {
    const bucket = events[i].sender_id ?? '__system__'
    const list = byBucket.get(bucket) ?? []
    list.push(i)
    byBucket.set(bucket, list)
  }

  for (const indices of byBucket.values()) {
    indices.sort((a, b) => {
      const aRow = events[a]
      const bRow = events[b]
      if (aRow.created_at !== bRow.created_at) return aRow.created_at - bRow.created_at
      return aRow.id.localeCompare(bRow.id)
    })

    // Cache the `this_event_hash` of every row we've successfully
    // verified in this bucket so far. A row is a page-boundary iff its
    // `prev_event_hash` is missing from this set — meaning its real
    // predecessor sits outside the loaded slice.
    const seenHashes = new Set<string>()
    let lastVerifiedHash: string | null = null

    for (const idx of indices) {
      const row = events[idx]
      const prevHash = row.prev_event_hash ?? null

      const recomputed = recomputeChainHash(row, prevHash)
      if (recomputed !== row.this_event_hash) {
        chainOk[idx] = false
        rowStatus[idx] = 'self-hash-mismatch'
        continue
      }

      const predecessorInPage = prevHash !== null && seenHashes.has(prevHash)
      if (predecessorInPage && lastVerifiedHash !== null && prevHash !== lastVerifiedHash) {
        chainOk[idx] = false
        rowStatus[idx] = 'link-broken'
        continue
      }

      chainOk[idx] = true
      rowStatus[idx] = predecessorInPage ? 'linked' : 'page-boundary'
      seenHashes.add(row.this_event_hash)
      lastVerifiedHash = row.this_event_hash
    }
  }

  const firstBreakIndex = chainOk.findIndex((ok) => !ok)
  return { chainOk, rowStatus, firstBreakIndex }
}

/**
 * Encode `FolderMemberListV1` through the WASM bridge. The encoder
 * sorts by `user_id` before emitting bytes, so the JS side can hand
 * over members in any order and still produce the canonical form the
 * server verifies against.
 */
function encodeFolderMemberList(input: FolderMemberListV1): Uint8Array {
  if (input.members.length === 0) {
    throw new Error('Cannot encode an empty folder member list')
  }
  const count = input.members.length
  const userIds = new Uint8Array(count * 16)
  const signedBy = new Uint8Array(count * 16)
  const fingerprints = new Uint8Array(count * 32)
  const shareRoles = new Uint8Array(count)
  const isOwnerFlags = new Uint8Array(count)
  for (let i = 0; i < count; i++) {
    const m = input.members[i]
    userIds.set(m.userId, i * 16)
    signedBy.set(m.signedByUserId, i * 16)
    fingerprints.set(m.pubkeyFingerprint, i * 32)
    shareRoles[i] = SHARE_ROLE_WIRE[m.shareRole]
    isOwnerFlags[i] = m.isOwner ? 1 : 0
  }
  const der = folder_member_list_encode_v1(
    input.folderId,
    input.folderOwnerId,
    userIds,
    fingerprints,
    shareRoles,
    isOwnerFlags,
    signedBy,
    input.membersSignedAt
  )
  if (!der) {
    throw new Error('Failed to DER-encode FolderMemberListV1')
  }
  return der
}

/**
 * Build the canonical `FolderMemberListV1` payload from typed string
 * inputs. Used by share / revoke / role-change flows that need to sign
 * the post-mutation member set.
 */
export function buildFolderMemberListInput(args: {
  folderId: string
  folderOwnerId: string
  members: FolderMemberListMemberInput[]
  membersSignedAt: bigint
}): FolderMemberListV1 {
  return {
    folderId: uuidStringToBytes(args.folderId),
    folderOwnerId: uuidStringToBytes(args.folderOwnerId),
    members: args.members.map((m) => ({
      userId: uuidStringToBytes(m.userId),
      pubkeyFingerprint: cryptfns.uint8.fromHex(m.pubkeyFingerprintHex),
      shareRole: m.shareRole,
      isOwner: m.isOwner,
      signedByUserId: uuidStringToBytes(m.signedByUserId)
    })),
    membersSignedAt: args.membersSignedAt
  }
}

/**
 * DER-encode the folder member list and sign
 * `b"hoodik-folder-list-v1\0" || payload_der` with the signer's RSA
 * private key. Returns the base64 signature plus the canonical DER so
 * call sites can submit both to the server.
 */
export async function signFolderMemberList(
  input: FolderMemberListV1,
  signerPrivateKey: string
): Promise<{ payloadDer: Uint8Array; signature: string }> {
  const payloadDer = encodeFolderMemberList(input)
  const signingInput = concatPrefixed(FOLDER_LIST_V1_PREFIX, payloadDer)
  const signature = isCurvePrivate(signerPrivateKey)
    ? await cryptfns.ed25519.signBytes(signingInput, signerPrivateKey)
    : rsa_sign_bytes(signingInput, signerPrivateKey)
  if (!signature) {
    throw new Error('Failed to sign FolderMemberListV1')
  }
  return { payloadDer, signature }
}

/**
 * Verify a `members_list_signature` against the canonical DER of the
 * supplied `FolderMemberListV1`. Returns `false` on encoding failure
 * so the caller can treat it as a verification failure rather than
 * propagating a low-level encoder error.
 *
 * When the signer carries a `key_transition`, the fallback re-encodes the
 * roster with the signer's *pre-migration* fingerprint in their own member
 * row — the roster commits to the signer's fingerprint, which rotated with the
 * key — then verifies against the old key. Mirrors the server's
 * `members_list_sig` fallback. `signerUserId` identifies which member row to
 * rewrite; without it the fallback can't run and only the current key is tried.
 */
export async function verifyFolderMemberListSignature(
  input: FolderMemberListV1,
  signatureB64: string,
  signer: SignerKey,
  signerUserId?: string
): Promise<boolean> {
  try {
    const payloadDer = encodeFolderMemberList(input)
    const signingInput = concatPrefixed(FOLDER_LIST_V1_PREFIX, payloadDer)
    return verifyBytesForSigner(signingInput, signatureB64, signer, (transition) => {
      if (!signerUserId) {
        // No way to locate the signer's own row to rewrite; re-encoding
        // unchanged would just retry the current-fingerprint bytes. Return
        // them so the fallback verify against the old key fails deterministically
        // rather than accepting a mismatched canonical.
        return signingInput
      }
      const oldFingerprintHex = fingerprintForUser({
        pubkey: transition.old_key_pem,
        key_type: transition.old_key_type
      })
      const signerIdBytes = uuidStringToBytes(signerUserId)
      const oldFingerprint = cryptfns.uint8.fromHex(oldFingerprintHex)
      const rewritten: FolderMemberListV1 = {
        ...input,
        members: input.members.map((m) =>
          bytesEqual(m.userId, signerIdBytes) ? { ...m, pubkeyFingerprint: oldFingerprint } : m
        )
      }
      return concatPrefixed(FOLDER_LIST_V1_PREFIX, encodeFolderMemberList(rewritten))
    })
  } catch {
    return false
  }
}

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, i) => byte === b[i])
}

/**
 * Verify the per-row `sender_signature` over `AuditEventSigInputV1`. The
 * mapping from a `ShareEvent` to its `AuditEventSigInputV1` is fixed.
 * Returns `false` for rows with a NULL signature
 * (system-cascade) — those are surfaced as "system action" in the UI.
 */
export async function verifyEventSignature(
  row: ShareEvent,
  sender: SignerKey
): Promise<boolean> {
  if (!row.sender_signature || !row.sender_id) return false
  const input = buildAuditEventSigInput({
    senderId: row.sender_id,
    recipientId: row.recipient_id,
    fileId: row.file_id,
    action: row.action,
    shareRoleBefore: row.share_role_before,
    shareRoleAfter: row.share_role_after,
    timestamp: BigInt(row.created_at)
  })
  return verifyAuditEvent(input, row.sender_signature, sender)
}
