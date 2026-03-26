use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
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
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<UpdateHashes>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let repository = Repository::new(&context.db);
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;

    let file = repository
        .manage(claims.sub)
        .update_hashes(file_id, data.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(file))
}
