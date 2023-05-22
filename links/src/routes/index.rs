use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;

use crate::repository::Repository;

/// Get a list of all the users publicly shared links.
///
/// Response: [Vec<crate::data::app_link::AppLink>]
#[route("/api/links", method = "GET")]
pub(crate) async fn index(
    context: web::Data<Context>,
    authenticated: Authenticated,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let repository = Repository::new(&context);

    let response = repository.links(authenticated.user.id).await?;

    Ok(HttpResponse::Ok().json(response))
}
