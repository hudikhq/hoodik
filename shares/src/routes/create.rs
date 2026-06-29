use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;
use serde_json::json;

use crate::{
    data::create_share::CreateShareEnvelope, repository::Repository, routes::gate,
};

/// `POST /api/shares` — idempotent grant or role change. Body is the
/// JSON envelope: `{ payload_der, signature,
/// entries, event_signature }`.
#[route("/api/shares", method = "POST")]
pub(crate) async fn create(
    context: web::Data<Context>,
    authenticated: Authenticated,
    body: web::Json<CreateShareEnvelope>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let repository = Repository::new(&context);
    let result = repository
        .create_share(body.into_inner(), &authenticated.user)
        .await?;

    Ok(HttpResponse::Created().json(json!({ "shares": result.shares })))
}
