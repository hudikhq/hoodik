use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use error::AppResult;

use crate::{data::query::Query, repository::Repository};

/// List files and directories
///
/// Request: [crate::data::query::Query]
///
/// Response: [crate::data::response::Response]
#[route("/api/storage", method = "GET", wrap = "Verify::default()")]
pub(crate) async fn index(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Query<Query>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;

    let file = Repository::new(&context.db)
        .manage(&authenticated.user)
        .find(data.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(file))
}
