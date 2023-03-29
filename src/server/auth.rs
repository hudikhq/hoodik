//! # Authentication routes
//!
//! This module is authentication controller for the application
use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{
    auth::Auth,
    contract::AuthProviderContract,
    data::{authenticated::Authenticated, create_user::CreateUser, credentials::Credentials},
    middleware::verify::Verify,
    providers::credentials::CredentialsProvider,
};
use context::Context;
use error::AppResult;

/// If the user is authenticated, return the user data, this is used once the frontend refreshes
#[route("/auth/self", method = "POST", wrap = "Verify::csrf_header_default()")]
pub async fn authenticated_self(req: HttpRequest) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;

    Ok(HttpResponse::Ok().json(authenticated))
}

/// Perform user login with basic credentials
///
/// Request: [auth::data::credentials::Credentials]
/// Response: [auth::data::authenticated::Authenticated]
#[route("/auth/login", method = "POST")]
pub async fn login(
    context: web::Data<Context>,
    data: web::Json<Credentials>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let provider = CredentialsProvider::new(&auth, data.into_inner());

    let response = provider.authenticate().await?;

    let cookie = auth.manage_cookie(&response.session, false).await?;

    Ok(HttpResponse::Ok().cookie(cookie).json(response))
}

/// Refresh a session to authenticated user
///
/// Response: [entity::sessions::Model]
#[route(
    "/auth/refresh",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn refresh(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;

    let auth = Auth::new(&context);

    let response = auth.refresh_session(&authenticated.session).await?;

    let cookie = auth.manage_cookie(&response, false).await?;

    Ok(HttpResponse::Ok().cookie(cookie).json(response))
}

/// Register a new user
///
/// Request: [auth::data::create_user::CreateUser]
/// Response: [entity::users::Model]
#[route("/auth/register", method = "POST")]
pub async fn register(
    context: web::Data<Context>,
    data: web::Json<CreateUser>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let response = auth.register(data.into_inner()).await?;

    Ok(HttpResponse::Created().json(response))
}
