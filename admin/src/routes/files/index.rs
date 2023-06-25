use actix_web::{route, web, HttpResponse};
use auth::data::staff::Staff;
use context::Context;
use error::AppResult;

use crate::{data::files::response::Response, repository::Repository};

/// List break down by file types on the platform and the amount
/// of space they are taking. Together with the amount of files
/// and available space on the platform.
///
/// Response: [crate::data::files::response::Response]
#[route("/api/admin/files", method = "GET")]
pub(crate) async fn index(staff: Staff, context: web::Data<Context>) -> AppResult<HttpResponse> {
    staff.is_admin_or_err()?;

    let context = context.into_inner();
    let repository = Repository::new(&context, &context.db);

    let stats = repository.files().stats().await?;
    let available_space = repository.files().available_space().await?;

    Ok(HttpResponse::Ok().json(Response {
        available_space,
        stats,
    }))
}
