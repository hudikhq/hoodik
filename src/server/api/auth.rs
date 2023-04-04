//! # Authentication routes
//!
//! This module is authentication controller for the application
use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{
    auth::Auth,
    contract::AuthProviderContract,
    data::{
        authenticated::{Authenticated, AuthenticatedJwt},
        create_user::CreateUser,
        credentials::Credentials,
    },
    middleware::verify::Verify,
    providers::credentials::CredentialsProvider,
};
use context::Context;
use error::AppResult;

/// If the user is authenticated, return the user data, this is used once the frontend refreshes
///
/// Response: [auth::data::authenticated::Authenticated]
#[route(
    "/api/auth/self",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn authenticated_self(req: HttpRequest) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;

    Ok(HttpResponse::Ok().json(authenticated))
}

/// Perform user login with basic credentials
///
/// Request: [auth::data::credentials::Credentials]
///
/// Response: [auth::data::authenticated::AuthenticatedJwt]
#[route("/api/auth/login", method = "POST")]
pub async fn login(
    context: web::Data<Context>,
    data: web::Json<Credentials>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let provider = CredentialsProvider::new(&auth, data.into_inner());

    let authenticated = provider.authenticate().await?;
    let jwt = auth::jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Ok();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}

/// Refresh a session to authenticated user
///
/// Response: [auth::data::authenticated::AuthenticatedJwt]
#[route(
    "/api/auth/refresh",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn refresh(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;
    let auth = Auth::new(&context);

    let authenticated = auth.refresh_session(&authenticated.session).await?;
    let jwt = auth::jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Ok();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}

/// Register a new user
///
/// Request: [auth::data::create_user::CreateUser]
///
/// Response: [AuthenticatedJwt]
#[route("/api/auth/register", method = "POST")]
pub async fn register(
    context: web::Data<Context>,
    data: web::Json<CreateUser>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let user = auth.register(data.into_inner()).await?;
    let session = auth.generate_session(&user, true).await?;
    let authenticated = Authenticated { user, session };
    let jwt = auth::jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Created();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}
