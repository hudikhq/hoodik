//! `POST /api/shares/{file_id}/fork` — save-to-my-drive.

use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use serde_json::json;

use crate::{data::fork::ForkBody, repository::Repository, routes::gate};

#[route("/api/shares/{file_id}/fork", method = "POST")]
pub(crate) async fn fork(
    context: web::Data<Context>,
    authenticated: Authenticated,
    path: web::Path<String>,
    body: web::Json<ForkBody>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;
    let source_file_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| Error::BadRequest("file_id_invalid".to_string()))?;
    let repository = Repository::new(&context);
    let outcome = repository
        .fork_file(&authenticated.user, source_file_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(json!({
        "file_id": outcome.new_file_id,
        "created_at": outcome.created_at,
    })))
}
