//! Request bodies for the editable-folder endpoints:
//!
//! * `POST /api/storage/upload-multikey`
//! * `POST /api/storage/{file_id}/evict-from-folder`
//! * `POST /api/storage/move-into-shared`
//!
//! All three carry a signed audit-event timestamp + signature so the
//! server reconstructs the same input the client signed and stores it on
//! the `share_events` row.

use serde::{Deserialize, Serialize};
use validr::*;

/// Body for `POST /api/storage/upload-multikey`.
///
/// `new_file_id` is supplied by the client so the audit-event
/// signature it produces covers a stable file_id the server can match
/// against. Reusing a pre-existing id collides at insert time and 409s.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadMultikeyBody {
    pub new_file_id: Option<String>,
    pub parent_file_id: Option<String>,
    pub name_hash: Option<String>,
    pub encrypted_name: Option<String>,
    pub encrypted_thumbnail: Option<String>,
    pub mime: Option<String>,
    pub size: Option<i64>,
    pub chunks: Option<i64>,
    pub sha256: Option<String>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub blake2b: Option<String>,
    pub cipher: Option<String>,
    pub editable: Option<bool>,
    pub file_modified_at: Option<String>,
    pub search_tokens_hashed: Option<Vec<String>>,
    pub member_keys: Option<Vec<MemberKey>>,
    pub members_list_snapshot: Option<MembersListSnapshot>,
    pub event_signature: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberKey {
    pub user_id: Option<String>,
    pub encrypted_key: Option<String>,
    #[serde(default)]
    pub is_owner_of_file: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MembersListSnapshot {
    pub members_signed_at: Option<i64>,
    pub members_list_signature: Option<String>,
}

impl Validation for UploadMultikeyBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(new_file_id),
            rule_required!(parent_file_id),
            rule_required!(name_hash),
            rule_required!(encrypted_name),
            rule_required!(mime),
            rule_required!(chunks),
            rule_required!(event_signature),
            rule_required!(timestamp),
        ]
    }
}

impl Validation for MemberKey {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(user_id), rule_required!(encrypted_key)]
    }
}


/// Body for `POST /api/storage/{file_id}/evict-from-folder`. The folder
/// owner authorises the eviction; the file_id in the URL is the
/// contributor's file to be detached. The signed audit input carries
/// `file_id = evicted_file_id`, `sender = folder owner`, `recipient =
/// file owner`.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EvictFromFolderBody {
    pub event_signature: Option<String>,
    pub timestamp: Option<i64>,
}

impl Validation for EvictFromFolderBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(event_signature), rule_required!(timestamp)]
    }
}

/// Body for `POST /api/storage/move-into-shared`. The caller pre-wraps
/// the file key for every current member of the destination folder and
/// submits the wraps alongside the snapshot the client just verified.
///
/// Two shapes share this endpoint:
///
/// * **Single file** — `member_keys` carries one wrap per destination
///   member of the moved file. `entries` is absent. The original wire
///   shape; existing web/mobile callers keep working unchanged.
/// * **Folder cascade** — `entries` carries one [`CascadeEntry`] per node
///   in the moved subtree (root + every descendant), each with its own
///   per-member wraps. `member_keys` is absent. The server recomputes the
///   subtree from its own state and rejects any client whose entry set
///   doesn't match it exactly.
///
/// A folder must always use the cascade shape: moving it with the flat
/// `member_keys` would re-parent the folder node while leaving every
/// descendant encrypted for the original owner alone.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveIntoSharedBody {
    pub file_id: Option<String>,
    pub destination_folder_id: Option<String>,
    pub member_keys: Option<Vec<MemberKey>>,
    pub entries: Option<Vec<CascadeEntry>>,
    pub members_list_snapshot: Option<MembersListSnapshot>,
    pub event_signature: Option<String>,
    pub timestamp: Option<i64>,
}

/// One node of a moved subtree: a file id (the moved root or any
/// descendant) and the per-member wraps of that node's file key.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CascadeEntry {
    pub file_id: Option<String>,
    pub member_keys: Option<Vec<MemberKey>>,
}

impl Validation for MoveIntoSharedBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(file_id),
            rule_required!(destination_folder_id),
            rule_required!(event_signature),
            rule_required!(timestamp),
        ]
    }
}

impl Validation for CascadeEntry {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(file_id)]
    }
}

/// Body for `POST /api/storage/move-out-of-shared`. The file owner detaches
/// their own file (or folder subtree) from the shared folder it lives in.
/// No key wraps: the moved nodes revert to private files the owner already
/// holds the keys for, so the server only drops the other members' rows.
/// `destination_folder_id` is the new private parent (absent / null = the
/// owner's drive root).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveOutOfSharedBody {
    pub file_id: Option<String>,
    pub destination_folder_id: Option<String>,
    pub event_signature: Option<String>,
    pub timestamp: Option<i64>,
}

impl Validation for MoveOutOfSharedBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(file_id),
            rule_required!(event_signature),
            rule_required!(timestamp),
        ]
    }
}
