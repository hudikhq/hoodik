use actix_web::{route, web, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use error::AppResult;

use crate::{data::invitations::search::Search, repository::Repository};

/// List invitations present on the platform
///
/// Request: [crate::data::invitations::search::Search]
///
/// Response: [Vec<entity::invitations::Model>]
#[route("/api/admin/invitations", method = "GET")]
pub(crate) async fn index(
    staff: Staff,
    context: web::Data<Context>,
    data: web::Query<Search>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let context = context.into_inner();

    let response = Repository::new(&context, &context.db)
        .invitations()
        .find(data.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(response))
}
