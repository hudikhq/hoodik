use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::{AppResult, Error};
use validr::Validation;

use crate::{
    auth::Auth,
    contracts::{cookies::Cookies, register::Register, sessions::Sessions},
    data::{authenticated::Authenticated, create_user::CreateUser},
};

/// Register a new user
///
/// Request: [crate::data::create_user::CreateUser]
///
/// Response: [Authenticated]
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
        context
            .settings
            .inner()
            .await
            .users
            .can_register_or_else(&email, || {
                Err(Error::as_validation("email", "not allowed to register"))
            })?;
    }

    let user = auth.register(data).await?;
    let session = auth.generate_session(&user, &user_agent, &ip).await?;
    let authenticated = Authenticated { user, session };

    let mut response = HttpResponse::Created();

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!())?;

    response.cookie(jwt);
    response.cookie(refresh);

    Ok(response.json(authenticated))
}
