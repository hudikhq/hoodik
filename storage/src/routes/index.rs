use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use error::AppResult;

use crate::{data::query::Query, repository::Repository};

/// List files and directories
///
/// Request: [crate::data::query::Query]
///
/// Response: [crate::data::response::Response]
#[route("/api/storage", method = "GET")]
pub(crate) async fn index(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Query<Query>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();

    let file = Repository::new(&context.db)
        .manage(claims.sub)
        .find(data.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(file))
}
