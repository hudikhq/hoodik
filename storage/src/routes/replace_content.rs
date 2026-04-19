//! Route for replacing the content of an editable file.
//!
//! Under the versioned-chunks model this is a two-phase operation: this
//! call only allocates a fresh `pending_version` and stores the in-flight
//! upload metadata (size, chunks). The actual chunk uploads land via
//! `POST /api/storage/{file_id}` and the auto-finalize fires once the
//! last chunk arrives. The active version's chunks are NEVER touched
//! here — readers continue to see the previous content uninterrupted.
//!
//! Returns 409 `another_edit_is_in_progress` when a pending edit already
//! exists; the client can retry with `force = true` to abandon the prior
//! pending and start fresh. On force-recovery, the orphaned pending
//! directory is purged from disk after the DB commit.

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
use error::AppResult;
use fs::prelude::*;

use crate::{
    data::replace_content::ReplaceContent,
    repository::{cached::evict_file, Repository},
};

/// Request: [crate::data::replace_content::ReplaceContent]
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage/{file_id}/content", method = "PUT")]
pub(crate) async fn replace_content(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<ReplaceContent>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    let validated = data.into_inner().validate_into(file_id)?;

    let fs = Fs::new(&context.config);

    let (file, abandoned_pending) = Repository::new(&context.db)
        .manage(claims.sub)
        .replace_content(file_id, validated)
        .await?;

    // Force-recovery side effect — drop the orphaned pending dir from
    // disk now that the DB has committed the new pending version. Best
    // effort: a failure here leaves disk garbage that next purge_all will
    // sweep up, but the user's edit flow is unaffected.
    if let Some(old_pending) = abandoned_pending {
        if let Err(e) = fs.purge_version(&file, old_pending).await {
            log::warn!(
                "Failed to purge abandoned pending v{} for file {}: {}. \
                 Disk garbage left behind; cleaned up on next file delete.",
                old_pending,
                file_id,
                e
            );
        }
    }

    // Cached metadata becomes stale once pending_version flips — evict
    // so the next read sees the new state.
    evict_file(file_id).await;

    Ok(HttpResponse::Ok().json(file))
}
