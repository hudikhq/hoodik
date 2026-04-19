use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{TransactionTrait, Uuid};
use error::AppResult;

use crate::{
    data::set_editable::SetEditable,
    repository::{cached::evict_file, Repository},
};

/// Toggle the `editable` flag on a file.
///
/// Request: [crate::data::set_editable::SetEditable]
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage/{file_id}/editable", method = "PUT")]
pub(crate) async fn set_editable(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<SetEditable>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let connection = context.db.begin().await?;
    let repository = Repository::new(&connection);
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;

    let file = repository
        .manage(claims.sub)
        .set_editable(file_id, data.into_inner())
        .await?;

    connection.commit().await?;

    // `editable` drives `use_versioned_layout` in upload/download — a
    // stale cache entry would send the next request down the wrong path.
    evict_file(file_id).await;

    Ok(HttpResponse::Ok().json(file))
}
