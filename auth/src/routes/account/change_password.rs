use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{auth::Auth, contracts::account::Account, data::change_password::ChangePassword};

/// Change the users password with the provided current password or private key
///
/// Request: [crate::data::change_password::ChangePassword]
#[route("/api/auth/account/change-password", method = "POST")]
pub(crate) async fn change_password(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Json<ChangePassword>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let ip = util::actix::extract_ip_ua(&req).1;
    let identity = data.email.as_deref().unwrap_or_default().trim().to_lowercase();
    let identity = (!identity.is_empty()).then_some(identity.as_str());
    let now = chrono::Utc::now().timestamp();
    crate::rate_limit::check(identity, &ip, now)?;

    if let Err(e) = auth.change_password(data.into_inner()).await {
        crate::rate_limit::charge_failure(identity, &ip, now);
        return Err(e);
    }

    Ok(HttpResponse::NoContent().finish())
}
