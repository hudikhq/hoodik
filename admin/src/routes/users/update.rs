use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::{
    data::users::{response::Response, update::Update},
    repository::Repository,
};

/// Update users information
///
/// Request: [crate::data::users::update::Update]
///
/// Response: [crate::data::users::response::Response]
#[route("/api/admin/users/{id}", method = "PUT")]
pub(crate) async fn update(
    req: HttpRequest,
    staff: Staff,
    context: web::Data<Context>,
    update: web::Json<Update>,
) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let id = util::actix::path_var::<Uuid>(&req, "id")?;
    staff.forbidden_self(id)?;

    let context = context.into_inner();

    let user = Repository::new(&context, &context.db)
        .users()
        .update(id, update.into_inner())
        .await?;

    let stats = Repository::new(&context, &context.db)
        .files()
        .stats_for(user.id)
        .await?;

    Ok(HttpResponse::Ok().json(Response { user, stats }))
}
