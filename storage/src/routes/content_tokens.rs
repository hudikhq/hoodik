use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{TransactionTrait, Uuid};
use error::AppResult;

use crate::{data::content_tokens::ContentTokens, repository::Repository};

/// Replace the note-body search tags for a file.
///
/// Request: [crate::data::content_tokens::ContentTokens]
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage/{file_id}/content-tokens", method = "PUT")]
pub(crate) async fn content_tokens(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<ContentTokens>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    crate::permission::require_write(&context.db, file_id, claims.sub).await?;

    let connection = context.db.begin().await?;
    let file = Repository::new(&connection)
        .manage(claims.sub)
        .replace_content_tokens(file_id, data.into_inner())
        .await?;
    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}
