//! # Authentication routes
//!
//! This module is authentication controller for the application
use crate::{
    auth::Auth,
    contract::AuthProviderContract,
    data::{
        authenticated::{Authenticated, AuthenticatedJwt},
        create_user::CreateUser,
        credentials::Credentials,
        signature::Signature,
    },
    jwt,
    middleware::verify::Verify,
    providers::{credentials::CredentialsProvider, signature::SignatureProvider},
};
use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::AppResult;

/// Register the authentication routes
/// on to the application server
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(register);
    cfg.service(authenticated_self);
    cfg.service(login);
    cfg.service(logout);
    cfg.service(signature);
    cfg.service(refresh);
}

/// If the user is authenticated, return the user data, this is used once the frontend refreshes
///
/// Response: [crate::data::authenticated::Authenticated]
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
/// Request: [crate::data::credentials::Credentials]
///
/// Response: [crate::data::authenticated::AuthenticatedJwt]
#[route("/api/auth/login", method = "POST")]
pub async fn login(
    context: web::Data<Context>,
    data: web::Json<Credentials>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let provider = CredentialsProvider::new(&auth, data.into_inner());

    let authenticated = provider.authenticate().await?;
    let jwt = jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Ok();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}

/// Perform user authentication with a key fingerprint and signature
///
/// Request: [crate::data::signature::Signature]
///
/// Response: [crate::data::authenticated::AuthenticatedJwt]
#[route("/api/auth/signature", method = "POST")]
pub async fn signature(
    context: web::Data<Context>,
    data: web::Json<Signature>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);

    let provider = SignatureProvider::new(&auth, data.into_inner());

    let authenticated = provider.authenticate().await?;
    let jwt = jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Ok();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}

/// Refresh a session to authenticated user
///
/// Response: [crate::data::authenticated::AuthenticatedJwt]
#[route(
    "/api/auth/refresh",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn refresh(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;
    let auth = Auth::new(&context);

    let authenticated = auth.refresh_session(&authenticated.session).await?;
    let jwt = jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Ok();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}

/// Logout user and perform session destroy
#[route(
    "/api/auth/logout",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn logout(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let authenticated = Authenticated::try_from(&req)?;
    let auth = Auth::new(&context);

    let authenticated = auth.destroy_session(&authenticated.session).await?;

    let mut response = HttpResponse::NoContent();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, true).await?;
        response.cookie(cookie);
    }

    Ok(response.finish())
}

/// Register a new user
///
/// Request: [crate::data::create_user::CreateUser]
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
    let jwt = jwt::generate(&authenticated, &context.config.jwt_secret)?;

    let mut response = HttpResponse::Created();

    if context.config.use_cookies {
        let cookie = auth.manage_cookie(&authenticated.session, false).await?;
        response.cookie(cookie);
    }

    Ok(response.json(AuthenticatedJwt { authenticated, jwt }))
}

/// Generate a two factor secret for the user
///
/// Response [String]
#[route("/api/auth/two-factor-secret", method = "GET")]
pub async fn generate_two_factor() -> AppResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({ "secret": Auth::generate_two_factor() })))
}
