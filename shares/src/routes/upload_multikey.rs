use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;
use serde_json::json;

use crate::{
    data::multikey_upload::UploadMultikeyBody, repository::Repository, routes::gate,
};

/// `POST /api/storage/upload-multikey`. Lives on the
/// shares route surface because the validation is sharing-aware and the
/// transaction touches `share_events`. The actual chunk upload still
/// goes through `POST /api/storage/{file_id}` against the returned id.
#[route("/api/storage/upload-multikey", method = "POST")]
pub(crate) async fn upload_multikey(
    context: web::Data<Context>,
    authenticated: Authenticated,
    body: web::Json<UploadMultikeyBody>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let repository = Repository::new(&context);
    let outcome = repository
        .upload_multikey(&authenticated.user, body.into_inner())
        .await?;

    Ok(HttpResponse::Created().json(json!({ "file_id": outcome.file_id })))
}
