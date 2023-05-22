use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::repository::Repository;

/// Delete a link by its id. (This won't delete the file)
///
/// Response: -
#[route("/api/links/{link_id}", method = "DELETE")]
pub(crate) async fn delete(
    req: HttpRequest,
    context: web::Data<Context>,
    authenticated: Authenticated,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let repository = Repository::new(&context);

    let id: Uuid = util::actix::path_var(&req, "link_id")?;

    repository.delete(id, authenticated.user.id).await?;

    Ok(HttpResponse::NoContent().finish())
}
