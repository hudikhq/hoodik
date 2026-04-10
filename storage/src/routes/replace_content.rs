//! Route for replacing the content of an editable file.
//! Purges existing chunks and resets the file for a fresh upload.

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{TransactionTrait, Uuid};
use error::AppResult;
use fs::prelude::*;

use crate::{
    data::replace_content::ReplaceContent,
    repository::{cached::evict_file, Repository},
};

/// Replace the content of an editable file
///
/// Purges existing chunks on disk and resets the file metadata
/// so new chunks can be uploaded via the regular upload endpoint.
///
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

    let fs = Fs::new(&context.config);
    let connection = context.db.begin().await?;

    let file = Repository::new(&connection)
        .manage(claims.sub)
        .replace_content(file_id, data.into_inner())
        .await?;

    connection.commit().await?;

    // Purge old chunks from disk after DB commit.
    // If purge fails, the DB state is still correct for re-upload — old chunks
    // become orphans but the user flow is unbroken. Log and continue.
    if let Err(e) = fs.purge(&file).await {
        log::error!(
            "Failed to purge old chunks for file {}: {}. Orphaned chunks may remain on storage.",
            file_id,
            e
        );
    }

    // Evict from cache so subsequent uploads see fresh metadata
    evict_file(file_id).await;

    Ok(HttpResponse::Ok().json(file))
}
