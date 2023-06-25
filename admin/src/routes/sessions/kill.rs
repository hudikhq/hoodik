use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::repository::Repository;

/// KIll a session.
#[route("/api/admin/sessions/{id}", method = "DELETE")]
pub(crate) async fn kill(
    req: HttpRequest,
    staff: Staff,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let context = context.into_inner();
    let id = util::actix::path_var::<Uuid>(&req, "id")?;

    Repository::new(&context, &context.db)
        .sessions()
        .kill(id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
