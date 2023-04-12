use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use error::AppResult;

use crate::repository::Repository;

/// Get file metadata by its id
///
/// Response: [crate::data::app_file::AppFile]
#[route(
    "/api/storage/{file_id}/metadata",
    method = "GET",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn metadata(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let file_id = util::actix::path_var(&req, "file_id")?;

    let file = Repository::new(&context.db)
        .manage(&authenticated.user)
        .get(file_id)
        .await?;

    Ok(HttpResponse::Ok().json(file))
}
