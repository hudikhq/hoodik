use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::{data::reindex::Reindex, repository::Repository};

/// How many rows one poll returns. The client works in small batches and
/// re-polls, so this only has to be comfortably larger than a batch — it is
/// not a page the user ever sees.
const PENDING_LIMIT: u64 = 500;

/// List files that still need re-indexing against the keyed search scheme.
///
/// Membership is derived, not tracked: a file is pending exactly while it has
/// no root-scope tags. Writing its tags removes it from this list, so a client
/// that is interrupted resumes simply by asking again.
///
/// Response: [Vec<crate::data::app_file::AppFile>]
#[route("/api/storage/reindex", method = "GET")]
pub(crate) async fn pending(claims: Claims, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();

    let files = Repository::new(&context.db)
        .tokens(claims.sub)
        .pending_reindex(PENDING_LIMIT)
        .await?;

    Ok(HttpResponse::Ok().json(files))
}

/// Replace one file's search tags and `name_hash`.
///
/// Request: [crate::data::reindex::Reindex]
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage/{file_id}/reindex", method = "PUT")]
pub(crate) async fn reindex(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<Reindex>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id = util::actix::path_var::<Uuid>(&req, "file_id")?;

    let file = Repository::new(&context.db)
        .manage(claims.sub)
        .reindex(file_id, data.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(file))
}
