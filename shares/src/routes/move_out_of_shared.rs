use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;
use serde_json::json;

use crate::{
    data::multikey_upload::MoveOutOfSharedBody, repository::Repository, routes::gate,
};

/// `POST /api/storage/move-out-of-shared`. The file
/// owner detaches their own file or folder subtree from a shared folder,
/// dropping every other member's access in one transaction.
#[route("/api/storage/move-out-of-shared", method = "POST")]
pub(crate) async fn move_out_of_shared(
    context: web::Data<Context>,
    authenticated: Authenticated,
    body: web::Json<MoveOutOfSharedBody>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let repository = Repository::new(&context);
    let file_id = repository
        .move_out_of_shared(&authenticated.user, body.into_inner())
        .await?;

    Ok(HttpResponse::Ok().json(json!({ "file_id": file_id })))
}
