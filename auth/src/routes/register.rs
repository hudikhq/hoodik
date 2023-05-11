use actix_web::{route, web, HttpResponse};
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
pub(crate) async fn register(
    context: web::Data<Context>,
    data: web::Json<CreateUser>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let user = auth.register(data.into_inner()).await?;
    let session = auth.generate_session(&user, true).await?;
    let authenticated = Authenticated { user, session };

    let mut response = HttpResponse::Created();

    let (jwt, refresh) = auth.manage_cookies(&authenticated, false).await?;
    response.cookie(jwt);
    response.cookie(refresh);

    Ok(response.json(authenticated))
}
