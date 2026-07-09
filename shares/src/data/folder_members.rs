use entity::Uuid;
use serde::{Deserialize, Serialize};

/// Response for `GET /api/shares/folder/{F}/members`.
///
/// `signature_algorithm` is hard-coded to `"rsa-pss-sha256"` for v1; the
/// field exists so future protocol versions can swap primitives without a
/// new endpoint.
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
}
