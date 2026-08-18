use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use error::AppResult;

use crate::repository::Repository;

/// Find the caller's files whose bytes hash to `hash`.
///
/// A first-class content lookup rather than a corner of search: "do you
/// already have these bytes" is a question about content, and anything that
/// syncs or backs up asks it constantly. Routing it through the text index
/// made it depend on whether a file happened to be indexed, which has nothing
/// to do with the question.
///
/// Matches any of the four digests stored at upload (md5, sha1, sha256,
/// blake2b), so a caller uses whichever it already computes. Access is the
/// usual `user_files` join: a digest from elsewhere reveals nothing the caller
/// could not already list.
///
/// Response: [Vec<crate::data::app_file::AppFile>]
#[route("/api/storage/by-hash/{hash}", method = "GET")]
pub(crate) async fn by_hash(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let hash = util::actix::path_var::<String>(&req, "hash")?;
    let compact = util::actix::query_var::<bool>(&req, "compact").unwrap_or(false);

    let files = Repository::new(&context.db)
        .query(claims.sub)
        .by_hash(&hash, compact)
        .await?;

    Ok(HttpResponse::Ok().json(files))
}
