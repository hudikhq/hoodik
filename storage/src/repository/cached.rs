//! # Cached repository
//! This is a cached repository that uses the `cached` crate to cache the results of the queries.
//! All the functions in here are shortcuts to the functions in the `Repository` struct.

use cached::proc_macro::cached;
use cached::SizedCache;
use context::Context;
use uuid::Uuid;

use crate::data::app_file::AppFile;

use super::Repository;

/// Get a file from the database
#[cached(
    name = "REPOSITORY_GET_FILE",
    type = "SizedCache<Uuid, Option<AppFile>>",
    create = "{ SizedCache::with_size(100) }",
    convert = r#"{ file_id }"#
)]
pub(crate) async fn get_file(context: &Context, owner_id: Uuid, file_id: Uuid) -> Option<AppFile> {
    Repository::new(&context.db)
        .manage(owner_id)
        .file(file_id)
        .await
        .ok()
}
