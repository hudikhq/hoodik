use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::contracts::capabilities::resolve;

/// Public capability advertisement. Auth-free so old clients can probe a
/// pre-1.16 server (which 404s) and 1.16 clients can fail closed on a
/// missing `sharing` block.
#[route("/api/capabilities", method = "GET")]
pub(crate) async fn capabilities(context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let caps = resolve(&context).await;
    Ok(HttpResponse::Ok().json(caps))
}
