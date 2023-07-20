use actix_web::{route, web, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use error::AppResult;

use crate::{data::sessions::search::Search, repository::Repository};

/// List and query all the platform sessions.
///
/// Request: [crate::data::sessions::search::Search]
///
/// Response: [entity::paginated::Paginated<data::sessions::session::Session>]
#[route("/api/admin/sessions", method = "GET")]
pub(crate) async fn index(
    staff: Staff,
    context: web::Data<Context>,
    data: web::Query<Search>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let context = context.into_inner();

    let response = Repository::new(&context, &context.db)
        .sessions()
        .find(data.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(response))
}
