use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::{
    data::create_share::RevokeShareBody, repository::Repository, routes::gate,
};

/// `DELETE /api/shares/{file_id}/{user_id}` — revoke. Co-owner / owner
/// only. Body carries the signed audit-event proof; the cascade fan-out
/// for revoked Co-owners is server-attributed with `sender_signature =
/// NULL`.
#[route("/api/shares/{file_id}/{user_id}", method = "DELETE")]
pub(crate) async fn delete(
    req: HttpRequest,
    context: web::Data<Context>,
    authenticated: Authenticated,
    body: Option<web::Json<RevokeShareBody>>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;
    let recipient_id: Uuid = util::actix::path_var(&req, "user_id")?;
    let body = body.map(|b| b.into_inner()).unwrap_or_default();

    let repository = Repository::new(&context);
    repository
        .revoke_share(body, &authenticated.user, file_id, recipient_id)
        .await?;

    Ok(HttpResponse::NoContent().finish())
}
