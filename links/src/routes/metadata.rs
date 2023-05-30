use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::repository::Repository;

/// Get application link by its id, return encrypted metadata
/// that will be handled by frontend to display its description and information.
///
/// Response: [crate::data::app_link::AppLink]
#[route("/api/links/{link_id}/metadata", method = "GET")]
pub(crate) async fn metadata(
    req: HttpRequest,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let link_id: Uuid = util::actix::path_var(&req, "link_id")?;
    let repository = Repository::new(&context);
    let link = repository.get(link_id).await?;

    Ok(HttpResponse::Ok().json(link))
}
