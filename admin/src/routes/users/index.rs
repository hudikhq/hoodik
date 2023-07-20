use actix_web::{route, web, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use error::AppResult;

use crate::{data::users::search::Search, repository::Repository};

/// List users present on the platform
///
/// Request: [crate::data::users::search::Search]
///
/// Response: [entity::paginated::Paginated<entity::users::Model>]
#[route("/api/admin/users", method = "GET")]
pub(crate) async fn index(
    staff: Staff,
    context: web::Data<Context>,
    data: web::Query<Search>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let context = context.into_inner();

    let response = Repository::new(&context, &context.db)
        .users()
        .find(data.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(response))
}
