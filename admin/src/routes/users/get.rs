use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::{data::users::response::Response, repository::Repository};

/// Get single user with all of its data and stats.
///
/// Response: [crate::data::users::response::Response]
#[route("/api/admin/users/{id}", method = "GET")]
pub(crate) async fn get(
    req: HttpRequest,
    staff: Staff,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let id = util::actix::path_var::<Uuid>(&req, "id")?;
    let context = context.into_inner();

    let user = Repository::new(&context, &context.db)
        .users()
        .get(id)
        .await?;

    let stats = Repository::new(&context, &context.db)
        .files()
        .stats_for(user.id)
        .await?;

    Ok(HttpResponse::Ok().json(Response { user, stats }))
}
