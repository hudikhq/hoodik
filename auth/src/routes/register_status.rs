use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;
use serde::Serialize;

#[derive(Serialize)]
struct RegisterStatus {
    allow_register: bool,
}

/// Public flag the SPA uses to decide whether to render the registration form
/// and the "Create an Account" link on the login pages. Whitelist and blacklist
/// rules stay server-side: only the boolean is exposed.
#[route("/api/auth/register/status", method = "GET")]
pub(crate) async fn register_status(context: web::Data<Context>) -> AppResult<HttpResponse> {
    let allow_register = context.settings.inner().await.users.allow_register();

    Ok(HttpResponse::Ok().json(RegisterStatus { allow_register }))
}
