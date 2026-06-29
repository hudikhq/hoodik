use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;

use crate::{data::discover::DiscoverQuery, repository::Repository, routes::gate};

/// `GET /api/users/discover?email=...` — find a verified user by email.
/// Authenticated; 20 requests/min/caller, counted on every call so
/// hit/miss timing can't be used for enumeration.
#[route("/api/users/discover", method = "GET")]
pub(crate) async fn discover(
    context: web::Data<Context>,
    authenticated: Authenticated,
    query: web::Query<DiscoverQuery>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let email = query.into_inner().email.unwrap_or_default();
    let now = chrono::Utc::now().timestamp();
    let repository = Repository::new(&context);
    let user = repository
        .discover_user(&authenticated.user, &email, now)
        .await?;

    Ok(HttpResponse::Ok().json(user))
}
