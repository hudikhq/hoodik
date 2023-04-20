use std::{cell::RefCell, rc::Rc};

use actix_web::{
    body::BoxBody,
    dev::ServiceResponse,
    dev::{Service, ServiceRequest, Transform},
    http::header::HeaderMap,
    web, Error, HttpMessage, ResponseError,
};
use chrono::Utc;
use context::Context;
use error::Error as AppError;
use futures_util::future::{ok, LocalBoxFuture, Ready};

use super::extractor::*;

/// Middleware that will load the session and user from the database on each request
/// and add them to the request extensions.
///
/// The middleware can be configured to ignore certain routes, in which case it will skip any session check
/// and just pass the request through.
///
/// IMPORTANT: This middleware DOES NOT protect the route from unauthenticated users. It only loads the session on the request.
///
/// This middleware works by extracting the session token from the header via `Authorization` header or by extracting it from the cookie.
///
///  - Via header: `Authorization: Bearer <JWT>`
///     - JWT is the session token that contains everything important about the session and will be deserialized into the `Authenticated` struct.
///  - Via cookie: `cookie_name=<session.token>`
///    - Cookie name can be configured via .env variables
#[derive(Clone)]
pub struct Load {
    pub(crate) ignore: Vec<String>,
    pub(crate) ignore_methods: Vec<String>,
    pub(crate) token_extractor: TokenExtractor,
}

impl Load {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Load {
        Load {
            ignore: vec![],
            ignore_methods: vec!["OPTIONS".to_string()],
            token_extractor: TokenExtractor::Header("Authorization".to_string()),
        }
    }
    pub fn token_cookie_name(mut self, name: String) -> Self {
        self.token_extractor = TokenExtractor::Cookie(name);

        self
    }

    pub fn token_header_name(mut self, name: String) -> Self {
        self.token_extractor = TokenExtractor::Header(name);

        self
    }

    pub fn add_ignore(mut self, route: String) -> Self {
        self.ignore.push(route);

        self
    }
}

impl<S> Transform<S, ServiceRequest> for Load
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = LoadMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(LoadMiddleware {
            service: Rc::new(RefCell::new(service)),
            ignore: self.ignore.clone(),
            ignore_methods: self.ignore_methods.clone(),
            token_extractor: self.token_extractor.clone(),
        })
    }
}

pub struct LoadMiddleware<S> {
    service: Rc<RefCell<S>>,
    ignore: Vec<String>,
    ignore_methods: Vec<String>,
    token_extractor: TokenExtractor,
}

impl<S> LoadMiddleware<S> {
    /// Extracts the method to acquire the authenticated session
    fn extract(&self, req: &ServiceRequest) -> ExtractionResult {
        match &self.token_extractor {
            TokenExtractor::Header(name) => self
                .header_extractor(name, req.headers())
                .map(ExtractionResult::Claims)
                .unwrap_or(ExtractionResult::None),
            TokenExtractor::Cookie(name) => self
                .token_extractor(name, req)
                .map(ExtractionResult::Token)
                .unwrap_or(ExtractionResult::None),
        }
    }

    /// Runs the extraction through the Authorization header
    fn header_extractor(&self, name: &str, headers: &HeaderMap) -> Option<String> {
        let header = headers.get(name)?;
        let mut header_value = header.to_str().ok()?.split(' ');
        let header_type = header_value.next()?;

        if header_type == "Bearer" {
            Some(header_value.next()?.to_string())
        } else {
            None
        }
    }

    /// Runs the extraction through the request cookie
    fn token_extractor(&self, name: &str, req: &ServiceRequest) -> Option<String> {
        let cookie = req.cookie(name)?;
        let token = cookie.value();

        Some(token.to_string())
    }
}

impl<S> Service<ServiceRequest> for LoadMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        log::info!(
            "{}: {:?}: Accessing route {}",
            Utc::now(),
            req.connection_info().peer_addr(),
            req.path()
        );
        let route = req.match_pattern().unwrap_or_default();
        let method = req.method().to_string();

        if self.ignore.contains(&route) {
            let fut = self.service.call(req);

            return Box::pin(async move { fut.await });
        }

        if self.ignore_methods.contains(&method) {
            let fut = self.service.call(req);

            return Box::pin(async move { fut.await });
        }

        let svc = self.service.clone();
        let extraction = self.extract(&req);

        Box::pin(async move {
            let context = match req.app_data::<web::Data<Context>>() {
                Some(v) => v,
                None => {
                    return Ok(ServiceResponse::new(
                        req.into_parts().0,
                        AppError::InternalError(
                            "auth::middleware::load|no_context_provided".to_string(),
                        )
                        .error_response(),
                    ))
                }
            };

            let authenticated = extract_session(&extraction, context).await;

            log::debug!(
                "auth::middleware::load|have_session: {} with extraction: {}; Route: {}",
                authenticated.is_some(),
                extraction,
                &route,
            );

            if let Some(a) = authenticated {
                req.extensions_mut().insert(a);
            }

            svc.call(req).await
        })
    }
}
