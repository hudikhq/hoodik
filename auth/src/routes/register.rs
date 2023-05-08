use actix_web::{route, web, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    data::{
        authenticated::{Authenticated, AuthenticatedJwt},
        create_user::CreateUser,
    },
    jwt,
};

/// Register a new user
///
/// Request: [crate::data::create_user::CreateUser]
///
/// Response: [AuthenticatedJwt]
#[route("/api/auth/register", method = "POST")]
pub(crate) async fn register(
    context: web::Data<Context>,
    data: web::Json<CreateUser>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let user = auth.register(data.into_inner()).await?;
    let session = auth.generate_session(&user, true).await?;
    let authenticated = Authenticated { user, session };
    let jwt = jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Created();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}
