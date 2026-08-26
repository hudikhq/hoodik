use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;

use crate::{repository::queries, routes::gate};

/// `GET /api/shares/keys` — every file shared with the caller, as
/// `(file_id, encrypted_key)` pairs.
///
/// Distinct from `/api/shares/mine`, which reports share *roots* — it trims
/// any row whose parent is also shared, because a recipient browsing their
/// shares wants the folder, not a flat dump of its contents. Search needs the
/// opposite: files inside a shared folder are tagged under their own keys, so
/// the caller has to hold every one of those keys to build a query that
/// reaches them.
///
/// Carries no names, no thumbnails and no metadata — a wrapped key per row and
/// nothing else, so it stays cheap on an account with a large shared folder.
///
/// Response: [Vec<crate::data::incoming::IncomingKey>]
#[route("/api/shares/keys", method = "GET")]
pub(crate) async fn keys(
    context: web::Data<Context>,
    authenticated: Authenticated,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let keys = queries::incoming_keys(&context.db, authenticated.user.id).await?;

    Ok(HttpResponse::Ok().json(keys))
}
