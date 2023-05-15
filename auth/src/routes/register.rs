use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::AppResult;

use crate::{
    auth::Auth,
    data::{authenticated::Authenticated, create_user::CreateUser},
};

/// Register a new user
///
/// Request: [crate::data::create_user::CreateUser]
///
/// Response: [Authenticated]
#[route("/api/auth/register", method = "POST")]
pub async fn register(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Json<CreateUser>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let (user_agent, ip) = util::actix::extract_ip_ua(&req);

    let user = auth.register(data.into_inner()).await?;
    let session = auth.generate_session(&user, &user_agent, &ip).await?;
    let authenticated = Authenticated { user, session };

    let mut response = HttpResponse::Created();

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!()).await?;

    response.cookie(jwt);
    response.cookie(refresh);

    Ok(response.json(authenticated))
}
