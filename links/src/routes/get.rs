use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;

use crate::repository::Repository;

/// Get application link by its id, return encrypted metadata
/// that will be handled by frontend to display its description and information.
///
/// Response: [crate::data::app_link::AppLink]
#[route("/api/links/{link_id}", method = "GET")]
pub(crate) async fn index(context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let link_id: Uuid = util::actix::path_var(&req, "link_id")?;
    let repository = Repository::new(&context);
    let link = repository.get(link_id).await?;

    Ok(HttpResponse::Ok().json(link))
}
