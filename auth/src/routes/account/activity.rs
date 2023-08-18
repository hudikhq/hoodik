use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    contracts::account::Account,
    data::{activity_query::ActivityQuery, claims::Claims},
};

/// Get all the users sessions
///
/// Request: [entity::paginated::Paginated<entity::sessions::Model>]
#[route("/api/auth/account/activity", method = "GET")]
pub(crate) async fn activity(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Query<ActivityQuery>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let response = auth
        .activity(data.into_inner().set_user(claims.sub))
        .await?;

    Ok(HttpResponse::Ok().json(response))
}
