//! ASN.1 DER encoders for the v1 signed payloads used by account-to-account
//! sharing. The same Rust code is exposed through native, WASM, and
//! Flutter FFI call sites — there is no second canonical encoder anywhere
//! in the codebase.
//!
//! Forward-compatibility rules apply uniformly to every structure in this
//! module:
//!
//! 1. The first field is always `version` (INTEGER).
//! 2. The last field is always `extensions: Option<OctetString>`. v1
//!    encoders set it to `None`; later minor revisions populate it with
//!    a nested DER-encoded blob whose contents v1 decoders ignore.
//!    Because ASN.1 DER's SEQUENCE has no implicit extension marker, the
//!    slot has to be declared in v1 to keep new bytes parseable by older
//!    code.
//! 3. When a hard reshape becomes unavoidable, bump the domain prefix to
//!    `v2` and ship a parallel encoder. The v1 encoder is preserved
//!    unchanged so v1 signatures and v1 audit-chain hashes remain
//!    verifiable forever.

use der::{asn1::OctetString, Decode, Encode, Enumerated, Sequence};

use crate::error::{CryptoResult, Error};

/// Domain-separation prefix for share-request signatures.
pub const SHARE_REQUEST_V1_PREFIX: &[u8] = b"hoodik-share-v1\0";

/// Domain-separation prefix for per-member signatures inside a folder share.
pub const MEMBER_SIG_V1_PREFIX: &[u8] = b"hoodik-folder-mem-v1\0";

/// Domain-separation prefix for the folder-member-list signature.
pub const FOLDER_LIST_V1_PREFIX: &[u8] = b"hoodik-folder-list-v1\0";

/// Domain-separation prefix for the audit-event hash chain.
pub const AUDIT_EVENT_V1_PREFIX: &[u8] = b"hoodik-audit-v1\0";

/// Domain-separation prefix for the per-row audit-event sender signature.
/// Signatures cover `AUDIT_EVENT_SIG_V1_PREFIX ||
/// encode_audit_event_sig_input_v1(payload)`.
pub const AUDIT_EVENT_SIG_V1_PREFIX: &[u8] = b"hoodik-audit-sig-v1\0";

/// Domain-separation prefix for the key-transition certificate. Both the old
/// and new keys sign `KEY_TRANSITION_V1_PREFIX || encode_key_transition_v1`.
pub const KEY_TRANSITION_V1_PREFIX: &[u8] = b"hoodik-key-transition-v1\0";

const UUID_LEN: usize = 16;
const SHA256_LEN: usize = 32;
const NONCE_LEN: usize = 16;

/// Three-tier permission encoded as ASN.1 ENUMERATED in the signed payload.
/// The integer values are part of the wire format and must not be reordered.
#[derive(Enumerated, Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ShareRoleEnum {
    Reader = 0,
    Editor = 1,
    CoOwner = 2,
}

/// Action recorded on a `share_events` row. ENUMERATED wire values are
/// stable forever — adding new actions appends, never reuses or reorders.
#[derive(Enumerated, Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AuditEventActionEnum {
    Grant = 0,
    Revoke = 1,
    RoleChange = 2,
    SharedFolderUpload = 3,
    Fork = 4,
    SharedByCoOwner = 5,
    SharedFolderEdit = 6,
    SharedFolderRestore = 7,
    SharedFolderEvict = 8,
    SharedFolderMoveOut = 9,
}

/// One file entry in a share request: `(file_id, sender's wrap of file_key
/// for recipient)`. The entries SEQUENCE OF is what `entries_hash` commits
/// to inside `ShareRequestPayloadV1`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShareEntry {
    pub file_id: [u8; UUID_LEN],
    pub encrypted_key: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct ShareEntryDer {
    file_id: OctetString,
    encrypted_key: OctetString,
}

impl ShareEntryDer {
    fn from_native(e: &ShareEntry) -> CryptoResult<Self> {
        Ok(Self {
            file_id: OctetString::new(Vec::from(e.file_id))?,
            encrypted_key: OctetString::new(e.encrypted_key.clone())?,
        })
    }

    fn into_native(self) -> CryptoResult<ShareEntry> {
        let file_id = bytes_to_array::<UUID_LEN>(self.file_id.as_bytes(), "file_id")?;
        Ok(ShareEntry {
            file_id,
            encrypted_key: self.encrypted_key.as_bytes().to_owned(),
        })
    }
}

/// Signed payload sent by `POST /api/shares`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShareRequestPayloadV1 {
    pub sender_id: [u8; UUID_LEN],
    pub recipient_id: [u8; UUID_LEN],
    pub recipient_pubkey_fingerprint: [u8; SHA256_LEN],
    pub share_role: ShareRoleEnum,
    pub root_file_id: [u8; UUID_LEN],
    pub entries_hash: [u8; SHA256_LEN],
    pub timestamp: i64,
    pub nonce: [u8; NONCE_LEN],
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct ShareRequestPayloadV1Der {
    version: u8,
    sender_id: OctetString,
    recipient_id: OctetString,
    recipient_pubkey_fingerprint: OctetString,
    share_role: ShareRoleEnum,
    root_file_id: OctetString,
    entries_hash: OctetString,
    timestamp: i64,
    nonce: OctetString,
    extensions: Option<OctetString>,
}

/// Signed payload covering a folder member's public key and role. Signer
/// is either the folder owner or any current Co-owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberSigPayloadV1 {
    pub user_id: [u8; UUID_LEN],
    pub pubkey_der: Vec<u8>,
    pub fingerprint: [u8; SHA256_LEN],
    pub share_role: ShareRoleEnum,
    pub signed_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct MemberSigPayloadV1Der {
    version: u8,
    user_id: OctetString,
    pubkey: OctetString,
    fingerprint: OctetString,
    share_role: ShareRoleEnum,
    signed_at: i64,
    extensions: Option<OctetString>,
}

/// The re-key endorsement. The user's old key signs this to vouch that the new
/// keys are theirs, and the new identity key counter-signs to prove it holds
/// the matching private key. Verifiers walk a chain of these so identity,
/// fingerprint continuity, existing share signatures, and TOFU trust all
/// survive a migration off RSA.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyTransitionV1 {
    pub user_id: [u8; UUID_LEN],
    pub old_key_spki: Vec<u8>,
    pub old_fingerprint: [u8; SHA256_LEN],
    pub new_identity_key_spki: Vec<u8>,
    pub new_wrapping_key_spki: Vec<u8>,
    pub new_fingerprint: [u8; SHA256_LEN],
    pub issued_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct KeyTransitionV1Der {
    version: u8,
    user_id: OctetString,
    old_key_spki: OctetString,
    old_fingerprint: OctetString,
    new_identity_key_spki: OctetString,
    new_wrapping_key_spki: OctetString,
    new_fingerprint: OctetString,
    issued_at: i64,
    extensions: Option<OctetString>,
}

impl KeyTransitionV1Der {
    fn from_native(p: &KeyTransitionV1) -> CryptoResult<Self> {
        Ok(Self {
            version: 1,
            user_id: OctetString::new(Vec::from(p.user_id))?,
            old_key_spki: OctetString::new(p.old_key_spki.clone())?,
            old_fingerprint: OctetString::new(Vec::from(p.old_fingerprint))?,
            new_identity_key_spki: OctetString::new(p.new_identity_key_spki.clone())?,
            new_wrapping_key_spki: OctetString::new(p.new_wrapping_key_spki.clone())?,
            new_fingerprint: OctetString::new(Vec::from(p.new_fingerprint))?,
            issued_at: p.issued_at,
            extensions: None,
        })
    }

    fn into_native(self) -> CryptoResult<KeyTransitionV1> {
        if self.version != 1 {
            return Err(Error::InvalidLength("key_transition_v1: version != 1"));
        }
        Ok(KeyTransitionV1 {
            user_id: bytes_to_array::<UUID_LEN>(self.user_id.as_bytes(), "user_id")?,
            old_key_spki: self.old_key_spki.as_bytes().to_owned(),
            old_fingerprint: bytes_to_array::<SHA256_LEN>(
                self.old_fingerprint.as_bytes(),
                "old_fingerprint",
            )?,
            new_identity_key_spki: self.new_identity_key_spki.as_bytes().to_owned(),
            new_wrapping_key_spki: self.new_wrapping_key_spki.as_bytes().to_owned(),
            new_fingerprint: bytes_to_array::<SHA256_LEN>(
                self.new_fingerprint.as_bytes(),
                "new_fingerprint",
            )?,
            issued_at: self.issued_at,
        })
    }
}

/// Row contents folded into the per-sender audit-chain hash. The chain
/// rule is
/// `this_event_hash = SHA-256(b"hoodik-audit-v1\0" || prev_event_hash || encode_audit_event_v1(row))`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEventRowV1 {
    pub sender_id: [u8; UUID_LEN],
    pub recipient_id: [u8; UUID_LEN],
    pub file_id: [u8; UUID_LEN],
    pub action: String,
    pub share_role: Option<ShareRoleEnum>,
    pub created_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct AuditEventRowV1Der {
    version: u8,
    sender_id: OctetString,
    recipient_id: OctetString,
    file_id: OctetString,
    action: OctetString,
    share_role: Option<ShareRoleEnum>,
    created_at: i64,
    extensions: Option<OctetString>,
}

/// One entry in the canonical folder member list. The signer's
/// per-member σ on each row defends against pubkey substitution; the
/// list signature over the whole sorted sequence defends against
/// selective member omission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderListMember {
    pub user_id: [u8; UUID_LEN],
    pub pubkey_fingerprint: [u8; SHA256_LEN],
    pub share_role: ShareRoleEnum,
    pub is_owner: bool,
    pub signed_by_user_id: [u8; UUID_LEN],
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct FolderListMemberDer {
    user_id: OctetString,
    pubkey_fingerprint: OctetString,
    share_role: ShareRoleEnum,
    is_owner: bool,
    signed_by_user_id: OctetString,
}

impl FolderListMemberDer {
    fn from_native(m: &FolderListMember) -> CryptoResult<Self> {
        Ok(Self {
            user_id: OctetString::new(Vec::from(m.user_id))?,
            pubkey_fingerprint: OctetString::new(Vec::from(m.pubkey_fingerprint))?,
            share_role: m.share_role,
            is_owner: m.is_owner,
            signed_by_user_id: OctetString::new(Vec::from(m.signed_by_user_id))?,
        })
    }

    fn into_native(self) -> CryptoResult<FolderListMember> {
        Ok(FolderListMember {
            user_id: bytes_to_array::<UUID_LEN>(self.user_id.as_bytes(), "user_id")?,
            pubkey_fingerprint: bytes_to_array::<SHA256_LEN>(
                self.pubkey_fingerprint.as_bytes(),
                "pubkey_fingerprint",
            )?,
            share_role: self.share_role,
            is_owner: self.is_owner,
            signed_by_user_id: bytes_to_array::<UUID_LEN>(
                self.signed_by_user_id.as_bytes(),
                "signed_by_user_id",
            )?,
        })
    }
}

/// Signed payload for a folder's canonical member list. The signer is
/// whichever actor (folder owner or any current Co-owner) most recently
/// mutated the membership. Members are
/// emitted in `user_id` ascending order so JS/Rust/Flutter producers
/// all agree on a single byte sequence; the encoder sorts on the
/// caller's behalf so call sites can ignore input order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FolderMemberListV1 {
    pub folder_id: [u8; UUID_LEN],
    pub folder_owner_id: [u8; UUID_LEN],
    pub members: Vec<FolderListMember>,
    pub members_signed_at: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct FolderMemberListV1Der {
    version: u8,
    folder_id: OctetString,
    folder_owner_id: OctetString,
    members: Vec<FolderListMemberDer>,
    members_signed_at: i64,
    extensions: Option<OctetString>,
}

/// Per-row sender signature input for audit events. Sender signs
/// `AUDIT_EVENT_SIG_V1_PREFIX || encode_audit_event_sig_input_v1`
/// with RSA-PSS-SHA256 at the time of the action; server verifies before
/// inserting the `share_events` row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEventSigInputV1 {
    pub sender_id: [u8; UUID_LEN],
    pub recipient_id: Option<[u8; UUID_LEN]>,
    pub file_id: [u8; UUID_LEN],
    pub action: AuditEventActionEnum,
    pub share_role_before: Option<ShareRoleEnum>,
    pub share_role_after: Option<ShareRoleEnum>,
    pub timestamp: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct AuditEventSigInputV1Der {
    version: u8,
    sender_id: OctetString,
    #[asn1(context_specific = "0", optional = "true", tag_mode = "EXPLICIT")]
    recipient_id: Option<OctetString>,
    file_id: OctetString,
    action: AuditEventActionEnum,
    #[asn1(context_specific = "1", optional = "true", tag_mode = "EXPLICIT")]
    share_role_before: Option<ShareRoleEnum>,
    #[asn1(context_specific = "2", optional = "true", tag_mode = "EXPLICIT")]
    share_role_after: Option<ShareRoleEnum>,
    timestamp: i64,
    extensions: Option<OctetString>,
}

/// DER-encode a share-request payload. Output is the body the sender
/// signs as `b"hoodik-share-v1\0" || payload_der`.
pub fn encode_share_request_v1(payload: &ShareRequestPayloadV1) -> CryptoResult<Vec<u8>> {
    Ok(ShareRequestPayloadV1Der::from_native(payload)?.to_vec()?)
}

/// DER-encode the per-row sender signature input (`AuditEventSigInputV1`).
pub fn encode_audit_event_sig_input_v1(
    payload: &AuditEventSigInputV1,
) -> CryptoResult<Vec<u8>> {
    Ok(AuditEventSigInputV1Der::from_native(payload)?.to_vec()?)
}

/// DER-decode the per-row sender signature input.
pub fn decode_audit_event_sig_input_v1(bytes: &[u8]) -> CryptoResult<AuditEventSigInputV1> {
    AuditEventSigInputV1Der::from_der(bytes)?.into_native()
}

/// DER-decode the share-request payload — used by the server to recover
/// the signed fields after verifying the signature against received bytes.
pub fn decode_share_request_v1(bytes: &[u8]) -> CryptoResult<ShareRequestPayloadV1> {
    ShareRequestPayloadV1Der::from_der(bytes)?.into_native()
}

/// DER-encode a member signature payload.
pub fn encode_member_sig_v1(payload: &MemberSigPayloadV1) -> CryptoResult<Vec<u8>> {
    Ok(MemberSigPayloadV1Der::from_native(payload)?.to_vec()?)
}

/// DER-encode a key-transition certificate body — the bytes both the old and
/// new keys sign.
pub fn encode_key_transition_v1(payload: &KeyTransitionV1) -> CryptoResult<Vec<u8>> {
    Ok(KeyTransitionV1Der::from_native(payload)?.to_vec()?)
}

/// DER-decode a key-transition certificate body.
pub fn decode_key_transition_v1(bytes: &[u8]) -> CryptoResult<KeyTransitionV1> {
    KeyTransitionV1Der::from_der(bytes)?.into_native()
}

/// DER-decode a member signature payload.
pub fn decode_member_sig_v1(bytes: &[u8]) -> CryptoResult<MemberSigPayloadV1> {
    MemberSigPayloadV1Der::from_der(bytes)?.into_native()
}

/// DER-encode an audit-event row body. Output is the trailing component of
/// each per-sender chain hash.
pub fn encode_audit_event_v1(row: &AuditEventRowV1) -> CryptoResult<Vec<u8>> {
    Ok(AuditEventRowV1Der::from_native(row)?.to_vec()?)
}

/// DER-decode an audit-event row body.
pub fn decode_audit_event_v1(bytes: &[u8]) -> CryptoResult<AuditEventRowV1> {
    AuditEventRowV1Der::from_der(bytes)?.into_native()
}

/// DER-encode a folder member-list payload. Members are sorted by
/// `user_id` ascending so the bytes are stable regardless of caller-
/// supplied order.
pub fn encode_folder_member_list_v1(payload: &FolderMemberListV1) -> CryptoResult<Vec<u8>> {
    Ok(FolderMemberListV1Der::from_native(payload)?.to_vec()?)
}

/// DER-decode a folder member-list payload.
pub fn decode_folder_member_list_v1(bytes: &[u8]) -> CryptoResult<FolderMemberListV1> {
    FolderMemberListV1Der::from_der(bytes)?.into_native()
}

/// DER-encode the entries list. Server hashes the output to recompute
/// `entries_hash` and reject requests where the recomputed hash differs
/// from the one signed into `ShareRequestPayloadV1`.
pub fn encode_entries_v1(entries: &[ShareEntry]) -> CryptoResult<Vec<u8>> {
    let mut sorted: Vec<&ShareEntry> = entries.iter().collect();
    sorted.sort_by_key(|a| a.file_id);

    let mut wire = Vec::with_capacity(sorted.len());
    for entry in sorted {
        wire.push(ShareEntryDer::from_native(entry)?);
    }
    Ok(wire.to_vec()?)
}

/// DER-decode an entries list. Returns entries in the order they were
/// encoded (which is sorted by `file_id`).
pub fn decode_entries_v1(bytes: &[u8]) -> CryptoResult<Vec<ShareEntry>> {
    let wire = Vec::<ShareEntryDer>::from_der(bytes)?;
    wire.into_iter().map(ShareEntryDer::into_native).collect()
}

impl ShareRequestPayloadV1Der {
    fn from_native(p: &ShareRequestPayloadV1) -> CryptoResult<Self> {
        Ok(Self {
            version: 1,
            sender_id: OctetString::new(Vec::from(p.sender_id))?,
            recipient_id: OctetString::new(Vec::from(p.recipient_id))?,
            recipient_pubkey_fingerprint: OctetString::new(Vec::from(
                p.recipient_pubkey_fingerprint,
            ))?,
            share_role: p.share_role,
            root_file_id: OctetString::new(Vec::from(p.root_file_id))?,
            entries_hash: OctetString::new(Vec::from(p.entries_hash))?,
            timestamp: p.timestamp,
            nonce: OctetString::new(Vec::from(p.nonce))?,
            extensions: None,
        })
    }

    fn into_native(self) -> CryptoResult<ShareRequestPayloadV1> {
        if self.version != 1 {
            return Err(Error::InvalidLength("share_request_v1: version != 1"));
        }
        Ok(ShareRequestPayloadV1 {
            sender_id: bytes_to_array::<UUID_LEN>(self.sender_id.as_bytes(), "sender_id")?,
            recipient_id: bytes_to_array::<UUID_LEN>(self.recipient_id.as_bytes(), "recipient_id")?,
            recipient_pubkey_fingerprint: bytes_to_array::<SHA256_LEN>(
                self.recipient_pubkey_fingerprint.as_bytes(),
                "recipient_pubkey_fingerprint",
            )?,
            share_role: self.share_role,
            root_file_id: bytes_to_array::<UUID_LEN>(self.root_file_id.as_bytes(), "root_file_id")?,
            entries_hash: bytes_to_array::<SHA256_LEN>(
                self.entries_hash.as_bytes(),
                "entries_hash",
            )?,
            timestamp: self.timestamp,
            nonce: bytes_to_array::<NONCE_LEN>(self.nonce.as_bytes(), "nonce")?,
        })
    }
}

impl MemberSigPayloadV1Der {
    fn from_native(p: &MemberSigPayloadV1) -> CryptoResult<Self> {
        Ok(Self {
            version: 1,
            user_id: OctetString::new(Vec::from(p.user_id))?,
            pubkey: OctetString::new(p.pubkey_der.clone())?,
            fingerprint: OctetString::new(Vec::from(p.fingerprint))?,
            share_role: p.share_role,
            signed_at: p.signed_at,
            extensions: None,
        })
    }

    fn into_native(self) -> CryptoResult<MemberSigPayloadV1> {
        if self.version != 1 {
            return Err(Error::InvalidLength("member_sig_v1: version != 1"));
        }
        Ok(MemberSigPayloadV1 {
            user_id: bytes_to_array::<UUID_LEN>(self.user_id.as_bytes(), "user_id")?,
            pubkey_der: self.pubkey.as_bytes().to_owned(),
            fingerprint: bytes_to_array::<SHA256_LEN>(self.fingerprint.as_bytes(), "fingerprint")?,
            share_role: self.share_role,
            signed_at: self.signed_at,
        })
    }
}

impl FolderMemberListV1Der {
    fn from_native(p: &FolderMemberListV1) -> CryptoResult<Self> {
        let mut sorted: Vec<&FolderListMember> = p.members.iter().collect();
        sorted.sort_by_key(|a| a.user_id);
        let mut wire_members = Vec::with_capacity(sorted.len());
        for member in sorted {
            wire_members.push(FolderListMemberDer::from_native(member)?);
        }
        Ok(Self {
            version: 1,
            folder_id: OctetString::new(Vec::from(p.folder_id))?,
            folder_owner_id: OctetString::new(Vec::from(p.folder_owner_id))?,
            members: wire_members,
            members_signed_at: p.members_signed_at,
            extensions: None,
        })
    }

    fn into_native(self) -> CryptoResult<FolderMemberListV1> {
        if self.version != 1 {
            return Err(Error::InvalidLength("folder_member_list_v1: version != 1"));
        }
        let members = self
            .members
            .into_iter()
            .map(FolderListMemberDer::into_native)
            .collect::<CryptoResult<Vec<_>>>()?;
        Ok(FolderMemberListV1 {
            folder_id: bytes_to_array::<UUID_LEN>(self.folder_id.as_bytes(), "folder_id")?,
            folder_owner_id: bytes_to_array::<UUID_LEN>(
                self.folder_owner_id.as_bytes(),
                "folder_owner_id",
            )?,
            members,
            members_signed_at: self.members_signed_at,
        })
    }
}

impl AuditEventSigInputV1Der {
    fn from_native(p: &AuditEventSigInputV1) -> CryptoResult<Self> {
        let recipient_id = match p.recipient_id {
            Some(r) => Some(OctetString::new(Vec::from(r))?),
            None => None,
        };
        Ok(Self {
            version: 1,
            sender_id: OctetString::new(Vec::from(p.sender_id))?,
            recipient_id,
            file_id: OctetString::new(Vec::from(p.file_id))?,
            action: p.action,
            share_role_before: p.share_role_before,
            share_role_after: p.share_role_after,
            timestamp: p.timestamp,
            extensions: None,
        })
    }

    fn into_native(self) -> CryptoResult<AuditEventSigInputV1> {
        if self.version != 1 {
            return Err(Error::InvalidLength("audit_event_sig_v1: version != 1"));
        }
        let recipient_id = match self.recipient_id {
            Some(r) => Some(bytes_to_array::<UUID_LEN>(r.as_bytes(), "recipient_id")?),
            None => None,
        };
        Ok(AuditEventSigInputV1 {
            sender_id: bytes_to_array::<UUID_LEN>(self.sender_id.as_bytes(), "sender_id")?,
            recipient_id,
            file_id: bytes_to_array::<UUID_LEN>(self.file_id.as_bytes(), "file_id")?,
            action: self.action,
            share_role_before: self.share_role_before,
            share_role_after: self.share_role_after,
            timestamp: self.timestamp,
        })
    }
}

impl AuditEventRowV1Der {
    fn from_native(r: &AuditEventRowV1) -> CryptoResult<Self> {
        Ok(Self {
            version: 1,
            sender_id: OctetString::new(Vec::from(r.sender_id))?,
            recipient_id: OctetString::new(Vec::from(r.recipient_id))?,
            file_id: OctetString::new(Vec::from(r.file_id))?,
            action: OctetString::new(r.action.as_bytes().to_vec())?,
            share_role: r.share_role,
            created_at: r.created_at,
            extensions: None,
        })
    }

    fn into_native(self) -> CryptoResult<AuditEventRowV1> {
        if self.version != 1 {
            return Err(Error::InvalidLength("audit_event_v1: version != 1"));
        }
        let action = String::from_utf8(self.action.as_bytes().to_owned())?;
        Ok(AuditEventRowV1 {
            sender_id: bytes_to_array::<UUID_LEN>(self.sender_id.as_bytes(), "sender_id")?,
            recipient_id: bytes_to_array::<UUID_LEN>(self.recipient_id.as_bytes(), "recipient_id")?,
            file_id: bytes_to_array::<UUID_LEN>(self.file_id.as_bytes(), "file_id")?,
            action,
            share_role: self.share_role,
            created_at: self.created_at,
        })
    }
}

fn bytes_to_array<const N: usize>(bytes: &[u8], field: &'static str) -> CryptoResult<[u8; N]> {
    if bytes.len() != N {
        return Err(Error::InvalidLength(field));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(bytes);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn share_request_fixture() -> ShareRequestPayloadV1 {
        ShareRequestPayloadV1 {
            sender_id: *b"\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11\x11",
            recipient_id: *b"\x22\x22\x22\x22\x22\x22\x22\x22\x22\x22\x22\x22\x22\x22\x22\x22",
            recipient_pubkey_fingerprint: [0x33u8; SHA256_LEN],
            share_role: ShareRoleEnum::Editor,
            root_file_id: *b"\x44\x44\x44\x44\x44\x44\x44\x44\x44\x44\x44\x44\x44\x44\x44\x44",
            entries_hash: [0x55u8; SHA256_LEN],
            timestamp: 1_735_689_600,
            nonce: *b"\x66\x66\x66\x66\x66\x66\x66\x66\x66\x66\x66\x66\x66\x66\x66\x66",
        }
    }

    fn member_sig_fixture() -> MemberSigPayloadV1 {
        MemberSigPayloadV1 {
            user_id: *b"\x77\x77\x77\x77\x77\x77\x77\x77\x77\x77\x77\x77\x77\x77\x77\x77",
            pubkey_der: vec![0xAAu8; 270],
            fingerprint: [0x88u8; SHA256_LEN],
            share_role: ShareRoleEnum::CoOwner,
            signed_at: 1_735_689_700,
        }
    }

    fn audit_event_fixture() -> AuditEventRowV1 {
        AuditEventRowV1 {
            sender_id: *b"\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA",
            recipient_id: *b"\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB",
            file_id: *b"\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC",
            action: "grant".to_string(),
            share_role: Some(ShareRoleEnum::Reader),
            created_at: 1_735_689_800,
        }
    }

    fn audit_event_sig_fixture() -> AuditEventSigInputV1 {
        AuditEventSigInputV1 {
            sender_id: *b"\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA\xAA",
            recipient_id: Some(*b"\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB\xBB"),
            file_id: *b"\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC\xCC",
            action: AuditEventActionEnum::RoleChange,
            share_role_before: Some(ShareRoleEnum::Reader),
            share_role_after: Some(ShareRoleEnum::Editor),
            timestamp: 1_735_689_900,
        }
    }

    fn entries_fixture() -> Vec<ShareEntry> {
        vec![
            ShareEntry {
                file_id: *b"\xDD\xDD\xDD\xDD\xDD\xDD\xDD\xDD\xDD\xDD\xDD\xDD\xDD\xDD\xDD\xDD",
                encrypted_key: vec![0x11u8; 256],
            },
            ShareEntry {
                file_id: *b"\x99\x99\x99\x99\x99\x99\x99\x99\x99\x99\x99\x99\x99\x99\x99\x99",
                encrypted_key: vec![0x22u8; 256],
            },
        ]
    }

    #[test]
    fn share_request_round_trips() {
        let p = share_request_fixture();
        let bytes = encode_share_request_v1(&p).unwrap();
        let back = decode_share_request_v1(&bytes).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn member_sig_round_trips() {
        let p = member_sig_fixture();
        let bytes = encode_member_sig_v1(&p).unwrap();
        let back = decode_member_sig_v1(&bytes).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn audit_event_round_trips() {
        let r = audit_event_fixture();
        let bytes = encode_audit_event_v1(&r).unwrap();
        let back = decode_audit_event_v1(&bytes).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn audit_event_round_trips_without_role() {
        let r = AuditEventRowV1 {
            share_role: None,
            ..audit_event_fixture()
        };
        let bytes = encode_audit_event_v1(&r).unwrap();
        let back = decode_audit_event_v1(&bytes).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn entries_round_trip_and_sort_canonically() {
        let entries = entries_fixture();
        let bytes = encode_entries_v1(&entries).unwrap();
        let back = decode_entries_v1(&bytes).unwrap();

        let mut expected = entries.clone();
        expected.sort_by_key(|a| a.file_id);
        assert_eq!(expected, back);
    }

    #[test]
    fn entries_encoding_is_order_independent() {
        let mut entries = entries_fixture();
        let a = encode_entries_v1(&entries).unwrap();
        entries.reverse();
        let b = encode_entries_v1(&entries).unwrap();
        assert_eq!(a, b, "entries hash must be stable under input reordering");
    }

    #[test]
    fn share_request_encoding_is_deterministic() {
        let p = share_request_fixture();
        assert_eq!(
            encode_share_request_v1(&p).unwrap(),
            encode_share_request_v1(&p).unwrap()
        );
    }

    #[test]
    fn member_sig_encoding_is_deterministic() {
        let p = member_sig_fixture();
        assert_eq!(
            encode_member_sig_v1(&p).unwrap(),
            encode_member_sig_v1(&p).unwrap()
        );
    }

    #[test]
    fn audit_event_encoding_is_deterministic() {
        let r = audit_event_fixture();
        assert_eq!(
            encode_audit_event_v1(&r).unwrap(),
            encode_audit_event_v1(&r).unwrap()
        );
    }

    #[test]
    fn share_role_rejects_unknown_value() {
        let p = share_request_fixture();
        let mut bytes = encode_share_request_v1(&p).unwrap();
        let needle = ShareRoleEnum::Editor as u8;
        let idx = bytes
            .iter()
            .position(|b| *b == needle)
            .expect("editor byte present in encoding");
        bytes[idx] = 7; // outside the closed { 0, 1, 2 } enum set
        assert!(decode_share_request_v1(&bytes).is_err());
    }

    #[test]
    fn share_role_wire_values_are_stable() {
        assert_eq!(ShareRoleEnum::Reader as u8, 0);
        assert_eq!(ShareRoleEnum::Editor as u8, 1);
        assert_eq!(ShareRoleEnum::CoOwner as u8, 2);
    }

    // Forward-compat: v1.x producers add fields by writing them into the
    // `extensions: Option<OctetString>` slot already reserved by the v1
    // schema. The v1 decoder reads bytes from such a producer cleanly and
    // simply ignores the contents of `extensions` (the production native
    // type does not expose them).

    fn opaque_extension_blob() -> Vec<u8> {
        // Future producers shape this however they want; v1 decoders treat
        // it as opaque bytes. Use a non-trivial DER value here so the test
        // exercises non-empty extension payloads end-to-end.
        let inner = OctetString::new(vec![0xDEu8, 0xAD, 0xBE, 0xEF]).unwrap();
        inner.to_vec().unwrap()
    }

    #[test]
    fn audit_event_sig_input_round_trips() {
        let p = audit_event_sig_fixture();
        let bytes = encode_audit_event_sig_input_v1(&p).unwrap();
        let back = decode_audit_event_sig_input_v1(&bytes).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn audit_event_sig_input_round_trips_with_all_optionals_absent() {
        let p = AuditEventSigInputV1 {
            sender_id: [0xAAu8; UUID_LEN],
            recipient_id: None,
            file_id: [0xCCu8; UUID_LEN],
            action: AuditEventActionEnum::Grant,
            share_role_before: None,
            share_role_after: Some(ShareRoleEnum::Reader),
            timestamp: 1_735_689_900,
        };
        let bytes = encode_audit_event_sig_input_v1(&p).unwrap();
        let back = decode_audit_event_sig_input_v1(&bytes).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn audit_event_sig_input_encoding_is_deterministic() {
        let p = audit_event_sig_fixture();
        assert_eq!(
            encode_audit_event_sig_input_v1(&p).unwrap(),
            encode_audit_event_sig_input_v1(&p).unwrap()
        );
    }

    #[test]
    fn audit_event_action_wire_values_are_stable() {
        // Wire bytes are forever — adding actions appends to the enum,
        // existing discriminants must not move.
        assert_eq!(AuditEventActionEnum::Grant as u8, 0);
        assert_eq!(AuditEventActionEnum::Revoke as u8, 1);
        assert_eq!(AuditEventActionEnum::RoleChange as u8, 2);
        assert_eq!(AuditEventActionEnum::SharedFolderUpload as u8, 3);
        assert_eq!(AuditEventActionEnum::Fork as u8, 4);
        assert_eq!(AuditEventActionEnum::SharedByCoOwner as u8, 5);
        assert_eq!(AuditEventActionEnum::SharedFolderEdit as u8, 6);
        assert_eq!(AuditEventActionEnum::SharedFolderRestore as u8, 7);
        assert_eq!(AuditEventActionEnum::SharedFolderEvict as u8, 8);
        assert_eq!(AuditEventActionEnum::SharedFolderMoveOut as u8, 9);
    }

    #[test]
    fn audit_event_sig_input_decoder_accepts_future_extensions() {
        let p = audit_event_sig_fixture();
        let mut wire = AuditEventSigInputV1Der::from_native(&p).unwrap();
        wire.extensions = Some(OctetString::new(opaque_extension_blob()).unwrap());
        let bytes = wire.to_vec().unwrap();

        let back = decode_audit_event_sig_input_v1(&bytes)
            .expect("v1 decoder must accept v1.x audit-event-sig bytes with extensions");
        assert_eq!(back, p);
    }

    #[test]
    fn share_request_v1_decoder_accepts_future_extensions() {
        let p = share_request_fixture();
        let mut wire = ShareRequestPayloadV1Der::from_native(&p).unwrap();
        wire.extensions = Some(OctetString::new(opaque_extension_blob()).unwrap());
        let bytes = wire.to_vec().unwrap();

        let back = decode_share_request_v1(&bytes)
            .expect("v1 decoder must accept v1.x bytes with extensions");
        assert_eq!(back, p);
    }

    #[test]
    fn share_request_v1_decoder_accepts_empty_extensions() {
        let p = share_request_fixture();
        let bytes = encode_share_request_v1(&p).unwrap();
        let parsed = ShareRequestPayloadV1Der::from_der(&bytes).unwrap();
        assert!(parsed.extensions.is_none());
    }

    #[test]
    fn member_sig_v1_decoder_accepts_future_extensions() {
        let p = member_sig_fixture();
        let mut wire = MemberSigPayloadV1Der::from_native(&p).unwrap();
        wire.extensions = Some(OctetString::new(opaque_extension_blob()).unwrap());
        let bytes = wire.to_vec().unwrap();

        let back = decode_member_sig_v1(&bytes)
            .expect("v1 decoder must accept v1.x member-sig bytes with extensions");
        assert_eq!(back, p);
    }

    #[test]
    fn audit_event_v1_decoder_accepts_future_extensions() {
        let r = audit_event_fixture();
        let mut wire = AuditEventRowV1Der::from_native(&r).unwrap();
        wire.extensions = Some(OctetString::new(opaque_extension_blob()).unwrap());
        let bytes = wire.to_vec().unwrap();

        let back = decode_audit_event_v1(&bytes)
            .expect("v1 decoder must accept v1.x audit-event bytes with extensions");
        assert_eq!(back, r);
    }

    fn folder_member_list_fixture() -> FolderMemberListV1 {
        FolderMemberListV1 {
            folder_id: [0xF0u8; UUID_LEN],
            folder_owner_id: [0x11u8; UUID_LEN],
            members: vec![
                FolderListMember {
                    user_id: [0x11u8; UUID_LEN],
                    pubkey_fingerprint: [0xA1u8; SHA256_LEN],
                    share_role: ShareRoleEnum::Reader,
                    is_owner: true,
                    signed_by_user_id: [0x11u8; UUID_LEN],
                },
                FolderListMember {
                    user_id: [0x22u8; UUID_LEN],
                    pubkey_fingerprint: [0xB2u8; SHA256_LEN],
                    share_role: ShareRoleEnum::CoOwner,
                    is_owner: false,
                    signed_by_user_id: [0x11u8; UUID_LEN],
                },
                FolderListMember {
                    user_id: [0x33u8; UUID_LEN],
                    pubkey_fingerprint: [0xC3u8; SHA256_LEN],
                    share_role: ShareRoleEnum::Editor,
                    is_owner: false,
                    signed_by_user_id: [0x22u8; UUID_LEN],
                },
            ],
            members_signed_at: 1_736_000_000,
        }
    }

    #[test]
    fn folder_member_list_v1_round_trips() {
        let p = folder_member_list_fixture();
        let bytes = encode_folder_member_list_v1(&p).unwrap();
        let back = decode_folder_member_list_v1(&bytes).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn folder_member_list_v1_encoding_is_deterministic() {
        let p = folder_member_list_fixture();
        assert_eq!(
            encode_folder_member_list_v1(&p).unwrap(),
            encode_folder_member_list_v1(&p).unwrap()
        );
    }

    #[test]
    fn folder_member_list_v1_sorts_members_canonically() {
        let p = folder_member_list_fixture();
        let mut reversed = p.clone();
        reversed.members.reverse();
        assert_eq!(
            encode_folder_member_list_v1(&p).unwrap(),
            encode_folder_member_list_v1(&reversed).unwrap(),
            "list bytes must be stable under input member reordering"
        );
    }

    #[test]
    fn folder_member_list_v1_version_check_rejects_unknown() {
        let mut wire = FolderMemberListV1Der::from_native(&folder_member_list_fixture()).unwrap();
        wire.version = 2;
        let bytes = wire.to_vec().unwrap();
        assert!(decode_folder_member_list_v1(&bytes).is_err());
    }

    #[test]
    fn folder_member_list_v1_decoder_accepts_future_extensions() {
        let p = folder_member_list_fixture();
        let mut wire = FolderMemberListV1Der::from_native(&p).unwrap();
        wire.extensions = Some(OctetString::new(opaque_extension_blob()).unwrap());
        let bytes = wire.to_vec().unwrap();

        let back = decode_folder_member_list_v1(&bytes)
            .expect("v1 decoder must accept v1.x list bytes with extensions");
        assert_eq!(back, p);
    }

    fn key_transition_fixture() -> KeyTransitionV1 {
        KeyTransitionV1 {
            user_id: [3u8; UUID_LEN],
            old_key_spki: Vec::from(&b"old-rsa-spki-der"[..]),
            old_fingerprint: [4u8; SHA256_LEN],
            new_identity_key_spki: Vec::from(&b"new-ed25519-spki-der"[..]),
            new_wrapping_key_spki: Vec::from(&b"new-x25519-spki-der"[..]),
            new_fingerprint: [5u8; SHA256_LEN],
            issued_at: 1_783_000_000,
        }
    }

    #[test]
    fn key_transition_v1_round_trips() {
        let p = key_transition_fixture();
        let bytes = encode_key_transition_v1(&p).unwrap();
        assert_eq!(decode_key_transition_v1(&bytes).unwrap(), p);
    }

    #[test]
    fn key_transition_v1_encoding_is_deterministic() {
        let p = key_transition_fixture();
        assert_eq!(
            encode_key_transition_v1(&p).unwrap(),
            encode_key_transition_v1(&p).unwrap()
        );
    }

    #[test]
    fn key_transition_v1_version_check_rejects_unknown() {
        let mut wire = KeyTransitionV1Der::from_native(&key_transition_fixture()).unwrap();
        wire.version = 2;
        let bytes = wire.to_vec().unwrap();
        assert!(decode_key_transition_v1(&bytes).is_err());
    }

    #[test]
    fn key_transition_v1_decoder_accepts_future_extensions() {
        let p = key_transition_fixture();
        let mut wire = KeyTransitionV1Der::from_native(&p).unwrap();
        wire.extensions = Some(OctetString::new(opaque_extension_blob()).unwrap());
        let bytes = wire.to_vec().unwrap();

        let back = decode_key_transition_v1(&bytes)
            .expect("v1 decoder must accept v1.x cert bytes with extensions");
        assert_eq!(back, p);
    }
}
