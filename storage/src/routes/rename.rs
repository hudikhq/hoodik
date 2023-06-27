use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{TransactionTrait, Uuid};
use error::AppResult;

use crate::{data::rename::Rename, repository::Repository};

/// Rename a file or a folder
///
/// Request: [crate::data::rename::Rename]
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage/{file_id}", method = "PUT")]
pub(crate) async fn rename(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<Rename>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let connection = context.db.begin().await?;
    let repository = Repository::new(&connection);
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;

    let file = repository
        .manage(claims.sub)
        .rename(file_id, data.into_inner())
        .await?;

    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}
