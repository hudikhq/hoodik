use serde::{Deserialize, Serialize};
use validr::*;

/// Request envelope for `POST /api/shares`. The signed bytes are
/// `payload_der` (base64); the `entries` array travels alongside but is
/// covered by the DER payload's `entries_hash` field, so the server can
/// reject tampered entries without bloating the signed body. The audit
/// row's `sender_signature` arrives as `event_signature` and is verified
/// against the per-row `AuditEventSigInputV1` the server constructs from
/// the validated request fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateShareEnvelope {
    pub payload_der: Option<String>,
    pub signature: Option<String>,
    pub entries: Option<Vec<CreateShareEntry>>,
    pub event_signature: Option<String>,
    /// Required when the root file is a folder. Carries the signer's
    /// RSA-PSS signature over the post-share `FolderMemberListV1`, the
    /// timestamp embedded in that payload, and the signer's user id
    /// (folder owner or current Co-owner). Optional in the envelope so
    /// the same DTO serves non-folder shares; the repository enforces
    /// presence-when-required at validation time.
    pub members_list_signature: Option<FolderMemberListSig>,
    /// Per-recipient signature over `MemberSigPayloadV1`. The granting
    /// actor (folder owner on a fresh
    /// grant, Co-owner on a reshare) signs the recipient's pubkey,
    /// fingerprint, and resulting share_role at the moment of issue. The
    /// signature is the same across every `user_files` row this envelope
    /// creates for the recipient — the payload does NOT include file_id,
    /// so the same σ binds the recipient to the same role on every entry
    /// in the request. Server-side: stored verbatim in
    /// `user_files.member_signature` so the SPA's verifier can chain
    /// trust from owner → Co-owner → reshares. Optional in the envelope
    /// to keep legacy clients parseable; the server treats
    /// `None` as a legacy fallback per `editable.ts::verifyFolderMemberList`.
    pub member_signature: Option<String>,
    /// Unix-seconds timestamp embedded in `MemberSigPayloadV1`. Server
    /// re-encodes the payload from `(recipient, role, signed_at)` and
    /// verifies the signature against the granting actor's pubkey.
    pub member_signed_at: Option<i64>,
}

/// Embedded folder-list signature carrier — shared shape across
/// create / revoke / role-change envelopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderMemberListSig {
    pub signature: String,
    pub signed_at: i64,
    pub signed_by_user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateShareEntry {
    pub file_id: Option<String>,
    pub encrypted_key: Option<String>,
}

impl Validation for CreateShareEnvelope {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(payload_der),
            rule_required!(signature),
            rule_required!(event_signature),
        ]
    }
}

impl Validation for CreateShareEntry {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(file_id), rule_required!(encrypted_key)]
    }
}

/// Body for `DELETE /api/shares/{file_id}/{user_id}`. Carries the
/// caller's RSA-PSS signature over the `AuditEventSigInputV1` for the
/// revoke action plus the timestamp used to construct that signed
/// input — the server reconstructs the input from these exact fields,
/// so the timestamp is part of the signed contract, not a server-side
/// clock read.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RevokeShareBody {
    pub event_signature: Option<String>,
    pub timestamp: Option<i64>,
    /// Required when the revoked row is on a folder. Mirrors
    /// `CreateShareEnvelope::members_list_signature`.
    pub members_list_signature: Option<FolderMemberListSig>,
}

impl Validation for RevokeShareBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(event_signature), rule_required!(timestamp)]
    }
}
