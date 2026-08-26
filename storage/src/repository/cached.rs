//! # Cached repository
//! This is a cached repository that uses the `cached` crate to cache the results of the queries.
//! All the functions in here are shortcuts to the functions in the `Repository` struct.

use cached::proc_macro::cached;
use cached::SizedCache;
use context::Context;
use entity::Uuid;

use crate::data::app_file::AppFile;

use super::Repository;

/// Get a file from the database, as seen by `owner_id`.
///
/// Keyed on the caller as well as the file: the returned row carries that
/// caller's `user_files` join (their `is_owner`, their `encrypted_key`) and
/// only exists at all if they have access, so a file-id-only key would serve
/// one user the row — and the working presigned URLs built from it — for
/// another user's file.
#[cached(
    name = "REPOSITORY_GET_FILE",
    type = "SizedCache<(Uuid, Uuid), Option<AppFile>>",
    create = "{ SizedCache::with_size(100) }",
    convert = r#"{ (owner_id, file_id) }"#
)]
pub(crate) async fn get_file(context: &Context, owner_id: Uuid, file_id: Uuid) -> Option<AppFile> {
    Repository::new(&context.db)
        .manage(owner_id)
        .file(file_id)
        .await
        .ok()
}

/// Evict a file from the cache so subsequent reads fetch fresh data.
/// Called after content replacement to avoid stale metadata during re-upload.
///
/// One file can hold several entries now — the owner's plus any recipient who
/// read it — and a content change stales every one, so drop them all.
pub(crate) async fn evict_file(file_id: Uuid) {
    let mut cache = REPOSITORY_GET_FILE.lock().await;
    cache.retain(|(_, id), _| *id != file_id);
}
