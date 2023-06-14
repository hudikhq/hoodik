use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::repository::Repository;

/// Expire the invitation which makes it impossible for user to register
/// on the platform using this invitation.
#[route("/api/admin/invitations/{id}", method = "DELETE")]
pub(crate) async fn expire(
    req: HttpRequest,
    staff: Staff,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;
    let id = util::actix::path_var::<Uuid>(&req, "id")?;
    let context = context.into_inner();

    let invitation = Repository::new(&context, &context.db)
        .invitations()
        .expire(id)
        .await?;

    Ok(HttpResponse::Ok().json(invitation))
}
