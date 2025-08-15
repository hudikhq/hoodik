use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::{AppResult, Error};
use validr::Validation;

use crate::{
    auth::Auth,
    contracts::{cookies::Cookies, ctx::Ctx, register::Register, sessions::Sessions},
    data::{authenticated::Authenticated, create_user::CreateUser},
};

/// Register a new user.
///
/// This method will either authenticated the user right after
/// the registration. Or will simply return an empty response.
///
/// This is due the user maybe needs to activate the account first
/// based on the application settings and the availability of a sender.
///
/// Request: [crate::data::create_user::CreateUser]
///
/// Response: [Authenticated] || 204 No Content
#[route("/api/auth/register", method = "POST")]
pub(crate) async fn register(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Json<CreateUser>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let (user_agent, ip) = util::actix::extract_ip_ua(&req);

    let data = data.into_inner().validate()?;
    let email = data.email.clone().unwrap();

    if data.invitation_id.is_none() {
        auth.can_register_or_else(&email, || {
            Err(Error::as_validation("email", "not allowed to register"))
        })
        .await?;
    }

    let user = auth.register(data).await?;

    if context
        .settings
        .inner()
        .await
        .users
        .enforce_email_activation()
        && user.email_verified_at.is_none()
    {
        return Ok(HttpResponse::NoContent().finish());
    }

    let session = auth.generate(&user, &user_agent, &ip).await?;
    let authenticated = Authenticated { user, session };

    let mut response = HttpResponse::Created();

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!())?;

    if !context.config.auth.use_headers_for_auth {
        response.cookie(jwt);
        response.cookie(refresh);
    } else {
        response.append_header(("x-auth-jwt".to_string(), jwt.value()));
        response.append_header(("x-auth-refresh".to_string(), refresh.value()));
    }

    Ok(response.json(authenticated))
}
