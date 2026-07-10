use std::collections::HashMap;

use entity::{share_events, Uuid};
use serde::{Deserialize, Serialize};

/// Response shape for a single audit-log row. Bytes that the client needs
/// to verify the per-sender chain (`prev_event_hash`, `this_event_hash`)
/// and the per-row sender signature (`sender_signature`) are emitted as
/// base64 strings; clients walk the chain themselves before trusting any
/// individual row.
///
/// `encrypted_name` + `cipher` come from a LEFT JOIN to `files`; null when
/// the file row is gone. `encrypted_key` comes from a LEFT JOIN to
/// `user_files` scoped to the caller; null when the caller has no wrap for
/// this file (revoked recipient, never had access). Either null collapses
/// the client-side decrypt to the bare-id fallback in the audit row
/// sentence — no second round-trip.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppShareEvent {
    pub id: Uuid,
    pub sender_id: Option<Uuid>,
    pub recipient_id: Option<Uuid>,
    /// NULL on account-level rows such as `key_rotation`.
    pub file_id: Option<Uuid>,
    pub action: String,
    pub share_role_before: Option<String>,
    pub share_role_after: Option<String>,
    pub created_at: i64,
    pub prev_event_hash: Option<String>,
    pub this_event_hash: String,
    pub sender_signature: Option<String>,
    pub encrypted_name: Option<String>,
    pub cipher: Option<String>,
    pub encrypted_key: Option<String>,
}

impl AppShareEvent {
    pub(crate) fn from_parts(
        row: share_events::Model,
        encrypted_name: Option<String>,
        cipher: Option<String>,
        encrypted_key: Option<String>,
    ) -> Self {
        Self {
            id: row.id,
            sender_id: row.sender_id,
            recipient_id: row.recipient_id,
            file_id: row.file_id,
            action: row.action,
            share_role_before: row.share_role_before,
            share_role_after: row.share_role_after,
            created_at: row.created_at,
            prev_event_hash: row.prev_event_hash.as_deref().map(cryptfns::base64::encode),
            this_event_hash: cryptfns::base64::encode(&row.this_event_hash),
            sender_signature: row.sender_signature.as_deref().map(cryptfns::base64::encode),
            encrypted_name,
            cipher,
            encrypted_key,
        }
    }
}

/// Compact identity record returned alongside the audit page so the
/// client can label rows and verify per-row signatures without an
/// extra round-trip per sender. Surfaces nothing that isn't already
/// public on `/api/users/discover`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditUserRef {
    pub id: Uuid,
    pub email: String,
    pub pubkey: String,
    pub key_type: String,
    pub wrapping_pubkey: Option<String>,
    pub fingerprint: String,
    /// Present when this account rotated keys, so a client verifying a
    /// pre-rotation audit-event signature can fall back to their old key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_transition: Option<crate::data::key_transition::KeyTransitionRef>,
}

/// Paginated envelope returned by `GET /api/shares/events`. The
/// `users` map carries every sender / recipient referenced in `events`
/// (id → minimal identity record) so the client can render names and
/// verify per-row signatures locally.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShareEventPage {
    pub events: Vec<AppShareEvent>,
    pub users: HashMap<Uuid, AuditUserRef>,
    pub total: u64,
    pub limit: u64,
    pub offset: u64,
}

/// Query string supported by `GET /api/shares/events`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ShareEventQuery {
    pub file_id: Option<String>,
    pub action: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}
