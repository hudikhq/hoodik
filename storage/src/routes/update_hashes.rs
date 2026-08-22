use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::transfer_claims::StorageClaims;
use context::Context;
use entity::{TransactionTrait, Uuid};
use error::AppResult;

use crate::{data::update_hashes::UpdateHashes, repository::Repository};

/// Update file content hashes after upload completes
///
/// Request: [crate::data::update_hashes::UpdateHashes]
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage/{file_id}/hashes", method = "PUT")]
pub(crate) async fn update_hashes(
    req: HttpRequest,
    claims: StorageClaims,
    context: web::Data<Context>,
    data: web::Json<UpdateHashes>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    claims.validate_transfer_path(file_id, "upload")?;
    crate::permission::require_write(&context.db, file_id, claims.sub()).await?;

    // One transaction across the column update and the tag replacement: a
    // crash between the two would otherwise leave keyed columns whose digest
    // is not findable, with nothing marking the file as needing another pass.
    let connection = context.db.begin().await?;

    let file = Repository::new(&connection)
        .manage(claims.sub())
        .update_hashes(file_id, data.into_inner())
        .await?;

    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}
