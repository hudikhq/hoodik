use actix_web::{route, web, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use error::AppResult;

use crate::{data::invitations::create::Create, repository::Repository};

/// Send an invitation that will enable user to register on the platform
/// if the platform has turned off free registration of the users.
///
/// Request: [crate::data::invitation::create::Create]
///
/// Response: [entity::invitations::Model]
#[route("/api/admin/invitations", method = "POST")]
pub(crate) async fn create(
    staff: Staff,
    context: web::Data<Context>,
    data: web::Json<Create>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let context = context.into_inner();

    let invitation = Repository::new(&context, &context.db)
        .invitations()
        .create(data.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(invitation))
}
