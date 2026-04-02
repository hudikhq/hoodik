use actix_web::{route, web, HttpResponse};
use chrono::{Duration, Utc};
use context::Context;
use error::AppResult;

use crate::data::{
    authenticated::Authenticated,
    transfer_claims::TransferClaims,
    transfer_token::{CreateTransferToken, TransferTokenResponse},
};

/// Create a long-lived transfer token scoped to a specific file and action.
///
/// The token is a JWT with `iss = "transfer"` and a `path` field encoding
/// the action and file ID. It is valid for `long_term_session_duration_days`
/// from the server configuration.
///
/// Request: [CreateTransferToken]
///
/// Response: [TransferTokenResponse]
#[route("/api/auth/transfer-token", method = "POST")]
pub(crate) async fn create_transfer_token(
    context: web::Data<Context>,
    authenticated: Authenticated,
    data: web::Json<CreateTransferToken>,
) -> AppResult<HttpResponse> {
    let (file_id, action) = data.into_inner().into_tuple()?;

    let duration_days = context.config.auth.long_term_session_duration_days;
    let expires_at = (Utc::now() + Duration::days(duration_days)).timestamp();

    let claims = TransferClaims {
        iss: "transfer".to_string(),
        sub: authenticated.user.id,
        exp: expires_at,
        iat: Utc::now().timestamp(),
        path: format!("{}/{}", action, file_id),
    };

    let token =
        crate::jwt::generate_transfer_token(&claims, &context.config.auth.jwt_secret)?;

    Ok(HttpResponse::Ok().json(TransferTokenResponse {
        token,
        expires_at,
        file_id,
        action,
    }))
}
