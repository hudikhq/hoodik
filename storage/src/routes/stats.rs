use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use error::AppResult;

use crate::{data::stats::Response, repository::Repository};

/// Get the users stats about the files and storage quota
///
/// Response: [crate::data::stats::Response]
#[route("/api/storage/stats", method = "POST")]
pub(crate) async fn stats(claims: Claims, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let repository = Repository::new(&context.db);
    let stats = repository.query(claims.sub).stats().await?;
    let used_space = repository.query(claims.sub).used_space().await?;

    Ok(HttpResponse::Ok().json(Response {
        stats,
        used_space,
        quota: claims.get_quota(&context).await,
    }))
}
