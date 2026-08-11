use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::repository::Repository;

/// Mark a user's email as verified on their behalf.
#[route("/api/admin/users/{id}/verify-email", method = "POST")]
pub(crate) async fn verify_email(
    req: HttpRequest,
    staff: Staff,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let id = util::actix::path_var::<Uuid>(&req, "id")?;
    let context = context.into_inner();

    Repository::new(&context, &context.db)
        .users()
        .verify_email(id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
