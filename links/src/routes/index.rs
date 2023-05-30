use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;
use validr::Validation;

use crate::{data::find::Find, repository::Repository};

/// Get a list of all the users publicly shared links.
///
/// Request: [crate::data::find::Find]
///
/// Response: [Vec<crate::data::app_link::AppLink>]
#[route("/api/links", method = "GET")]
pub(crate) async fn index(
    context: web::Data<Context>,
    authenticated: Authenticated,
    data: web::Query<Find>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let repository = Repository::new(&context);
    let data = data.into_inner().validate()?;
    let with_expired = data.with_expired.unwrap_or(false);

    let response = repository
        .links(authenticated.user.id, with_expired)
        .await?;

    Ok(HttpResponse::Ok().json(response))
}
