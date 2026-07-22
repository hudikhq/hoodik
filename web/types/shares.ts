export type ShareRole = 'reader' | 'editor' | 'co-owner'

/**
 * A member's role *in a group* — what they may do to the group itself
 * (view it, share files into it, manage its roster). This is a different
 * axis from {@link ShareRole}, which is what a recipient may do to a
 * shared *file*. They reuse the same three words but never the same field:
 * group roles travel as `group_role`, file roles as `share_role`. A group
 * co-owner manages the group; a file co-owner reshares the file.
 */
export type GroupRole = 'reader' | 'editor' | 'co-owner'

export type AuditEventAction =
  | AuditEventActionWire
  // Account-level event with no file and no recipient, signed under the
  // key-rotation scheme (not `AuditEventSigInputV1`). Chained into the
  // owner's per-sender audit chain on RSA→curve25519 migration.
  | 'key_rotation'

export interface DiscoveredUser {
  user_id: string
  email: string
  pubkey: string
  fingerprint: string
  /** `"rsa"` (assumed when absent) or `"curve25519"`. Curve25519 accounts
   *  carry an Ed25519 identity `pubkey` and receive key wraps under
   *  `wrapping_pubkey` instead of `pubkey`. */
  key_type?: string
  /** X25519 SPKI PEM for curve25519 accounts; `null` for RSA accounts. */
  wrapping_pubkey?: string | null
}

export interface ShareEntryInput {
  file_id: string
  encrypted_key: string
}

export interface FolderMemberListSig {
  signature: string
  signed_at: number
  signed_by_user_id: string
}

export interface CreateShareEnvelope {
  payload_der: string
  signature: string
  entries: ShareEntryInput[]
  event_signature: string
  /** Required when the share targets a folder root. Carries the fresh
   * post-share `FolderMemberListV1` signature, the timestamp embedded
   * in that payload, and the signer's user id. Server rejects with
   * 400 `missing_members_list_signature` when absent on a folder. */
  members_list_signature?: FolderMemberListSig
  /** Per-recipient signature over `MemberSigPayloadV1`.
   * The granting actor commits to the recipient's pubkey,
   * fingerprint, and resulting role at issue time so a later viewer
   * of the folder member list can chain trust from owner →
   * Co-owner → reshares. Persisted verbatim into every produced
   * `user_files.member_signature`. Optional for backward compat:
   * legacy clients omit it and the server treats the row as legacy
   * per `verifyFolderMemberList`. */
  member_signature?: string
  /** Unix-seconds timestamp embedded in `MemberSigPayloadV1`. Server
   * re-encodes and verifies against this exact value. */
  member_signed_at?: number
}

export interface RevokeShareBody {
  event_signature: string
  timestamp: number
  /** Required when the revoke target is a folder root. Same shape +
   * meaning as `CreateShareEnvelope::members_list_signature`. */
  members_list_signature?: FolderMemberListSig
}

export interface AppShare {
  file_id: string
  recipient_id: string
  recipient_email: string
  recipient_pubkey_fingerprint: string
  share_role: ShareRole
  created_at: number
  shared_at: number | null
  shared_by_user_id: string | null
  shared_by_email: string | null
}

export interface CreateShareResponse {
  shares: AppShare[]
}

export interface IncomingShare {
  file_id: string
  /**
   * Mime type of the shared item. `"dir"` for folders, the file's
   * mime for leaves. Drives the Shared-with-me row click handler:
   * folders navigate into the file browser, files open the detail
   * panel (or the notes editor for editable markdown).
   */
  mime: string
  /** Encrypted filename — the recipient unwraps `encrypted_key` and
   *  decrypts this client-side to surface the plaintext name. */
  encrypted_name: string
  /** Whether the file has a thumbnail (generated at upload time). The
   *  listing never ships the blob — the recipient's row fetches it from
   *  the storage thumbnail route (their `user_files` row grants access)
   *  and decrypts with the same `encrypted_key`. */
  has_thumbnail?: boolean
  /** Cipher used for `encrypted_name` (and the file's chunks). */
  cipher: string
  /** File's `editable` flag; true for markdown notes that can be saved
   *  in-place. Combined with `share_role` to decide whether to open
   *  the editor or the read-only preview. */
  editable: boolean
  /** Total bytes, chunk count, and finalize timestamp lifted from the
   *  owner's `files` row so recipient-side rows render the same size
   *  and upload-progress chips an owned row would. */
  size?: number | null
  chunks?: number | null
  chunks_stored?: number | null
  finished_upload_at?: number | null
  md5?: string | null
  sha1?: string | null
  sha256?: string | null
  blake2b?: string | null
  share_role: ShareRole
  encrypted_key: string
  created_at: number
  shared_at: number | null
  owner_id: string
  owner_email: string
  owner_pubkey: string
  owner_pubkey_fingerprint: string
  shared_by_user_id: string | null
  shared_by_email: string | null
}

export interface IncomingSharePage {
  items: IncomingShare[]
  total: number
  limit: number
  offset: number
}

export interface ShareEvent {
  id: string
  sender_id: string | null
  recipient_id: string | null
  /** Null on account-level events (e.g. `key_rotation`) that aren't tied to a
   *  file. Mirrors the server's nullable `share_events.file_id`. */
  file_id: string | null
  action: AuditEventAction
  share_role_before: ShareRole | null
  share_role_after: ShareRole | null
  created_at: number
  prev_event_hash: string | null
  this_event_hash: string
  sender_signature: string | null
  /**
   * File's encrypted name, joined from `files`. Null when the file row
   * is gone (deleted). Combined with `encrypted_key` + `cipher`, lets the
   * client render the plaintext name on the audit row.
   */
  encrypted_name: string | null
  /**
   * Cipher the file was encrypted with — needed so `decrypt(...)` picks
   * the algorithm that matches the file's stored ciphertext. Null
   * alongside `encrypted_name` when the file row is gone.
   */
  cipher: string | null
  /**
   * Caller's RSA-wrapped file key, joined from `user_files` and scoped
   * to the caller. Null when the caller has no wrap for this file
   * (revoked recipient, never had access) — the audit row falls back to
   * the truncated file id.
   */
  encrypted_key: string | null
}

/**
 * A signer's single key rotation, attached to any response carrying a
 * signature they may have produced before rotating. Absent means the signer
 * never rotated — verify against the current key only. Carries every field
 * the transition canonical covers, so the client can verify the certificate
 * before trusting the old key. Public certificate material only. Mirrors the
 * server's `KeyTransitionRef`.
 */
export interface KeyTransitionRef {
  old_key_pem: string
  old_key_type: string
  old_fingerprint: string
  new_identity_key_pem: string
  new_wrapping_key_pem: string
  new_fingerprint: string
  old_signature: string
  new_signature: string
  issued_at: number
}

/**
 * A stored `key_transitions` row as `GET /api/auth/key-transitions` serves it.
 * Byte columns arrive as JSON number arrays; the signatures are raw bytes
 * (unlike {@link KeyTransitionRef}, which pre-encodes them to base64).
 */
export interface KeyTransitionRow {
  user_id: string
  old_fingerprint: string
  old_key_spki: number[]
  old_key_type: string
  new_fingerprint: string
  new_identity_key_pem: string
  new_wrapping_key_pem: string
  old_signature: number[]
  new_signature: number[]
  issued_at: number
}

export interface AuditUserRef {
  id: string
  email: string
  pubkey: string
  fingerprint: string
  /** `"rsa"` (assumed when absent) or `"curve25519"` — see {@link DiscoveredUser}. */
  key_type?: string
  wrapping_pubkey?: string | null
  key_transition?: KeyTransitionRef
}

export interface ShareEventPage {
  events: ShareEvent[]
  users: Record<string, AuditUserRef>
  total: number
  limit: number
  offset: number
}

export interface SharingCapabilities {
  enabled: boolean
  roles: ShareRole[]
}

export interface Capabilities {
  sharing: SharingCapabilities
  editable_folders: boolean
  share_groups: boolean
  audit_log: boolean
  fork: boolean
  default_cipher?: string
  server_version?: string
}

export interface FolderMember {
  user_id: string
  email: string | null
  pubkey: string
  pubkey_fingerprint: string
  /** `"rsa"` (assumed when absent) or `"curve25519"` — see {@link DiscoveredUser}. */
  key_type?: string
  wrapping_pubkey?: string | null
  share_role: ShareRole
  is_owner: boolean
  added_at: number | null
  signed_by_user_id: string | null
  member_signature: string | null
  key_transition?: KeyTransitionRef
}

export interface FolderMembersResponse {
  folder_id: string
  folder_owner_id: string
  folder_owner_pubkey_fingerprint: string
  signature_algorithm: string
  members: FolderMember[]
  members_signed_at: number | null
  members_list_signature: string | null
  members_list_signed_by_user_id: string | null
}

/**
 * `ShareRequestPayloadV1` mirrors the Rust struct of the same name. The
 * client builds this from typed fields, encodes through the WASM exporter,
 * then signs `b"hoodik-share-v1\0" || payloadDer` before sending.
 */
export interface ShareRequestPayloadV1 {
  senderId: Uint8Array
  recipientId: Uint8Array
  recipientPubkeyFingerprint: Uint8Array
  shareRole: ShareRole
  rootFileId: Uint8Array
  entriesHash: Uint8Array
  timestamp: bigint
  nonce: Uint8Array
}

/**
 * The actions the `AuditEventSigInputV1` share-event canonical can encode.
 * {@link AuditEventAction} adds `key_rotation`, which is signed under a
 * different scheme and must never reach the share-event encoder.
 */
export type AuditEventActionWire =
  | 'grant'
  | 'revoke'
  | 'role_change'
  | 'shared_folder_upload'
  | 'fork'
  | 'shared_by_co_owner'
  | 'shared_folder_edit'
  | 'shared_folder_restore'
  | 'shared_folder_evict'
  | 'shared_folder_move_out'

/**
 * `FolderMemberListV1` mirrors the Rust struct of the same name. The
 * client builds this from typed fields, encodes through the WASM
 * exporter, then signs `b"hoodik-folder-list-v1\0" || payloadDer`
 * before submitting on any folder membership mutation.
 */
export interface FolderMemberListV1 {
  folderId: Uint8Array
  folderOwnerId: Uint8Array
  members: FolderMemberListMember[]
  membersSignedAt: bigint
}

export interface FolderMemberListMember {
  userId: Uint8Array
  pubkeyFingerprint: Uint8Array
  shareRole: ShareRole
  isOwner: boolean
  signedByUserId: Uint8Array
}

/**
 * Builder input for `buildFolderMemberListInput` — typed string form
 * of `FolderMemberListMember`, so call sites don't have to pre-convert
 * UUIDs and hex fingerprints to byte arrays.
 */
export interface FolderMemberListMemberInput {
  userId: string
  pubkeyFingerprintHex: string
  shareRole: ShareRole
  isOwner: boolean
  signedByUserId: string
}

/**
 * `AuditEventSigInputV1` mirrors the Rust struct. `recipientId` is
 * `null` for self-referential events (fork, evict). The two role fields
 * are `null` when the audit row carries no before/after role for that
 * action.
 */
export interface AuditEventSigInputV1 {
  senderId: Uint8Array
  recipientId: Uint8Array | null
  fileId: Uint8Array
  action: AuditEventActionWire
  shareRoleBefore: ShareRole | null
  shareRoleAfter: ShareRole | null
  timestamp: bigint
}

export interface TrustedFingerprintEntry {
  pubkeyFingerprint: string
  lastVerifiedAt: number
  /** `silent` means the entry was recorded after a successful share
   *  without the user having to manually acknowledge the fingerprint.
   *  `key-transition` is used after a successful automatic post-migration
   *  fingerprint continuity via the append-only key_transitions chain.
   *  Kept as a separate value so an audit of past trust decisions can
   *  distinguish ceremony-backed entries from passive ones. */
  verificationMethod: 'qr' | 'voice' | 'in-person' | 'silent' | 'other' | 'key-transition'
}

export interface AppShareGroup {
  id: string
  owner_id: string
  name: string
  created_at: number
}

export interface AppShareGroupMember {
  user_id: string
  email: string
  fingerprint: string
  added_at: number
  /** The member's role *in the group* (reader/editor/co-owner). Distinct
   *  from any file-level share role those words also name. */
  group_role: GroupRole
}

export interface AppShareGroupWithMembers extends AppShareGroup {
  members: AppShareGroupMember[]
}

export interface AppShareGroupAsMember extends AppShareGroup {
  owner_email: string
  added_at: number
  /** The caller's own role in this group — drives which actions the UI
   *  offers (share-to-group for editors, manage for co-owners). */
  group_role: GroupRole
}

/**
 * The full recipient set returned by
 * `GET /api/shares/groups/{id}/members`. The roster is the full recipient
 * set a share-to-group fan-out wraps for: the group owner (carried with
 * `group_role: "owner"`) plus every member. Reuses the `DiscoveredUser`
 * field shape so the fan-out wraps the whole set from one response instead
 * of an N+1 of discover calls.
 */
export interface GroupMemberWithKey {
  user_id: string
  email: string
  pubkey: string
  fingerprint: string
  /** `"rsa"` (assumed when absent) or `"curve25519"` — see {@link DiscoveredUser}. */
  key_type?: string
  wrapping_pubkey?: string | null
  group_role: GroupRole | 'owner'
}

export interface GroupsResponse {
  owned: AppShareGroupWithMembers[]
  member_of: AppShareGroupAsMember[]
}

export interface CreateGroupBody {
  name: string
}

/**
 * Body for `POST /api/shares/groups/{id}/members`. A group is a saved
 * recipient selection, so adding a member is a plain roster insert: no file
 * keys move and there is no crypto payload. The timestamp + nonce guard the
 * write against replay.
 */
export interface AddGroupMemberBody {
  user_id: string
  pubkey_fingerprint: string
  /** The new member's role in the group (reader/editor/co-owner). */
  group_role: GroupRole
  timestamp: number
  /** 16-byte replay nonce, base64-encoded. Bound per `(caller, nonce)` on
   *  the server alongside the timestamp window. */
  nonce: string
}

/**
 * One member's wrap of the file key for the multi-key upload endpoint.
 * Mirrors the Rust `MemberKey` struct in `shares/src/data/multikey_upload.rs`.
 */
export interface UploadMultiKeyMember {
  user_id: string
  encrypted_key: string
  is_owner_of_file?: boolean
}

/**
 * Snapshot of the destination folder's member list at the moment the
 * uploader verified signatures. Echoed to the server so it can reject
 * stale uploads with `409 share_membership_changed`.
 */
export interface UploadMultiKeySnapshot {
  members_signed_at: number | null
  members_list_signature: string | null
}

export interface UploadMultiKeyBody {
  new_file_id: string
  parent_file_id: string
  name_hash: string
  encrypted_name: string
  encrypted_thumbnail?: string
  mime: string
  size?: number
  chunks: number
  sha256?: string
  md5?: string
  sha1?: string
  blake2b?: string
  cipher?: string
  editable?: boolean
  file_modified_at?: string
  search_tokens_hashed?: string[]
  member_keys: UploadMultiKeyMember[]
  members_list_snapshot: UploadMultiKeySnapshot
  event_signature: string
  timestamp: number
}

export interface UploadMultiKeyResponse {
  file_id: string
}

export interface EvictFromFolderBody {
  event_signature: string
  timestamp: number
}

export interface MoveIntoSharedBody {
  file_id: string
  destination_folder_id: string
  member_keys: UploadMultiKeyMember[]
  members_list_snapshot: UploadMultiKeySnapshot
  event_signature: string
  timestamp: number
}

/**
 * One node of a folder cascade for `POST /api/storage/move-into-shared`: a
 * file id (the moved root or any descendant) and that node's file key wrapped
 * once per destination member. The server recomputes the subtree from its own
 * state and rejects with `entries_do_not_match_subtree` unless `entries`
 * covers exactly the root plus every descendant.
 */
export interface CascadeEntry {
  file_id: string
  member_keys: UploadMultiKeyMember[]
}

/**
 * Folder variant of `MoveIntoSharedBody`. Carries one `CascadeEntry` per node
 * instead of a flat `member_keys`; the single-file shape stays for back-compat.
 */
export interface MoveIntoSharedCascadeBody {
  file_id: string
  destination_folder_id: string
  entries: CascadeEntry[]
  members_list_snapshot: UploadMultiKeySnapshot
  event_signature: string
  timestamp: number
}

/**
 * `POST /api/storage/move-out-of-shared` — the file's owner detaches an owned
 * node (and its subtree) from the shared folder it lives in. No key wraps
 * travel: the nodes revert to private files the owner already holds keys for,
 * and the server drops every other member's rows. `destination_folder_id`
 * omitted/null lands the node at the owner's drive root.
 */
export interface MoveOutOfSharedBody {
  file_id: string
  destination_folder_id?: string | null
  event_signature: string
  timestamp: number
}

export interface ForkBody {
  new_file_id: string
  encrypted_metadata: string
  encrypted_thumbnail?: string
  name_hash: string
  mime: string
  size?: number
  chunks?: number
  md5?: string
  sha1?: string
  sha256?: string
  blake2b?: string
  cipher?: string
  encrypted_key: string
  search_tokens_hashed?: string[]
  event_signature: string
  timestamp: number
}

export interface ForkResponse {
  file_id: string
  created_at: number
}
