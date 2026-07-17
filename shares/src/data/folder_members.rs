use entity::Uuid;
use serde::{Deserialize, Serialize};

use crate::data::key_transition::KeyTransitionRef;

/// Response for `GET /api/shares/folder/{F}/members`.
///
/// `signature_algorithm` names the algorithm `members_list_signature` was
/// produced with — `"rsa-pss-sha256"` or `"ed25519"` — derived from the signer's
/// key at signing time. Clients dispatch verification on each member's
/// `key_type`; this records what actually signed, the way `files.cipher` records
/// a file's cipher.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FolderMembersResponse {
    pub folder_id: Uuid,
    pub folder_owner_id: Uuid,
    pub folder_owner_pubkey_fingerprint: String,
    pub signature_algorithm: &'static str,
    pub members: Vec<FolderMember>,
    pub members_signed_at: Option<i64>,
    pub members_list_signature: Option<String>,
    pub members_list_signed_by_user_id: Option<Uuid>,
}

/// One row of `members[]`. Every member of a folder share sees every
/// other member's email — sharing a file shares the roster.
/// `member_signature` is the already-base64'd σ_member from the
/// recipient's `user_files` row.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FolderMember {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub pubkey: String,
    pub key_type: String,
    pub wrapping_pubkey: Option<String>,
    pub pubkey_fingerprint: String,
    pub share_role: String,
    pub is_owner: bool,
    pub added_at: Option<i64>,
    pub signed_by_user_id: Option<Uuid>,
    pub member_signature: Option<String>,
    /// Present when this member rotated keys, so a client verifying the
    /// roster or per-member signature can fall back to their old key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_transition: Option<KeyTransitionRef>,
}
