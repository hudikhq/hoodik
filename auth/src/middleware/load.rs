use std::{cell::RefCell, rc::Rc};

use crate::auth::Auth;
use actix_web::{
    body::BoxBody,
    dev::ServiceResponse,
    dev::{Service, ServiceRequest, Transform},
    web, Error, HttpMessage, ResponseError,
};
use context::Context;
use error::Error as AppError;
use futures_util::future::{ok, LocalBoxFuture, Ready};

#[derive(Clone, PartialEq, Debug)]
pub enum TokenExtractor {
    Header(String),
    Cookie(String),
}

/// Middleware that will load the session and user from the database on each request
/// and add them to the request extensions.
///
/// The middleware can be configured to ignore certain routes, in which case it will skip any session check
/// and just pass the request through.
///
/// IMPORTANT: This middleware DOES NOT protect the route from unauthenticated users. It only loads the session on the request.
///
/// This middleware works by extracting the session token from the header via `Authorization` header, or
/// via extracting it from the cookie. The token is then used to find the session in the database.
/// There are two possible ways to extract the token:
///
///  - `Bearer <token>`
///     - the token is looked up in the database and the session is loaded (if its currently active)
///  - `Signature <signature-base64> <pubkey-base64>`
///     - middleware will look into the database for the user with the given pubkey and will verify the signature
///     - in case you are using the signature the session doesn't exist so the csrf token is not used
#[derive(Clone)]
pub struct Load {
    pub(crate) ignore: Vec<String>,
    pub(crate) token_extractor: TokenExtractor,
}

impl Load {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Load {
        Load {
            ignore: vec![],
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
            token_extractor: self.token_extractor.clone(),
        })
    }
}

pub struct LoadMiddleware<S> {
    service: Rc<RefCell<S>>,
    ignore: Vec<String>,
    token_extractor: TokenExtractor,
}

impl<S> LoadMiddleware<S> {
    /// Extracts the token from the request to try and find the active session
    fn extract_token(&self, req: &ServiceRequest) -> Option<String> {
        match &self.token_extractor {
            TokenExtractor::Header(name) => {
                let header = req.headers().get(name)?;
                let mut header_value = header.to_str().ok()?.split(' ');
                let header_type = header_value.next()?;

                if header_type == "Bearer" {
                    Some(header_value.next()?.to_string())
                } else {
                    None
                }
            }
            TokenExtractor::Cookie(name) => {
                let cookie = req.cookie(name)?;
                let token = cookie.value();
                Some(token.to_string())
            }
        }
    }

    /// Extracts the signature and pubkey from the request to run the authentication by using the pubkey
    /// Header name: Authorization
    /// Header format: Signature <signature-base64> <pubkey-hex>
    fn extract_signature_and_pubkey(&self, req: &ServiceRequest) -> Option<(String, String)> {
        let header = req.headers().get("Authorization")?;
        let mut header_value = header.to_str().ok()?.split(' ');
        let header_type = header_value.next()?;

        if header_type == "Signature" {
            // Extract both the signature and pubkey out of the header value
            let signature = header_value.next()?;
            let pubkey = header_value.next()?;

            Some((signature.to_string(), pubkey.to_string()))
        } else {
            None
        }
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
        let route = req.match_pattern().unwrap_or_default();

        if self.ignore.contains(&route) {
            let fut = self.service.call(req);

            return Box::pin(async move { fut.await });
        }

        let svc = self.service.clone();
        let maybe_token = self.extract_token(&req);
        let maybe_signature_and_pubkey = self.extract_signature_and_pubkey(&req);

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

            let mut have_session = false;

            if let Some(token) = &maybe_token {
                if let Ok(authenticated) = Auth::new(context).get_by_token(token).await {
                    req.extensions_mut().insert(authenticated);
                    have_session = true;
                }
            }

            if !have_session {
                if let Some((signature, pubkey)) = &maybe_signature_and_pubkey {
                    if let Ok(authenticated) = Auth::new(context)
                        .get_by_signature_and_pubkey(signature, pubkey)
                        .await
                    {
                        req.extensions_mut().insert(authenticated);
                    }
                }
            }

            svc.call(req).await
        })
    }
}
