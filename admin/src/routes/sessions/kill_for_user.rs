use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::repository::Repository;

/// Kill all the session for a given user.
#[route("/api/admin/sessions/{user_id}/kill-for-user", method = "DELETE")]
pub(crate) async fn kill_for_user(
    req: HttpRequest,
    staff: Staff,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let context = context.into_inner();
    let user_id = util::actix::path_var::<Uuid>(&req, "user_id")?;

    Repository::new(&context, &context.db)
        .sessions()
        .kill_for(user_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
