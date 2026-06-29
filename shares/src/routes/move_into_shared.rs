use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;
use serde_json::json;

use crate::{
    data::multikey_upload::MoveIntoSharedBody, repository::Repository, routes::gate,
};

/// `POST /api/storage/move-into-shared`. Relocates
/// a file the caller owns into a shared folder, re-wrapping the file
/// key for every current member of the destination.
#[route("/api/storage/move-into-shared", method = "POST")]
pub(crate) async fn move_into_shared(
    context: web::Data<Context>,
    authenticated: Authenticated,
    body: web::Json<MoveIntoSharedBody>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let repository = Repository::new(&context);
    let outcome = repository
        .move_into_shared(&authenticated.user, body.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(json!({ "file_id": outcome.file_id })))
}
