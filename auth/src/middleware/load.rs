use std::{cell::RefCell, rc::Rc};

use crate::auth::Auth;
use actix_web::{
    body::BoxBody,
    dev::ServiceResponse,
    dev::{Service, ServiceRequest, Transform},
    Error, HttpMessage, ResponseError,
};
use context::Context;
use error::Error as AppError;
use futures_util::future::{ok, LocalBoxFuture, Ready};

#[derive(Clone, PartialEq, Debug)]
pub enum TokenExtractor {
    Header(String),
    Cookie(String),
}

#[derive(Clone)]
pub struct Load {
    pub(crate) ignore: Vec<String>,
    pub(crate) token_extractor: TokenExtractor,
}

impl Load {
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
    fn extract_token(&self, req: &ServiceRequest) -> Option<String> {
        match &self.token_extractor {
            TokenExtractor::Header(name) => {
                let header = req.headers().get(name)?;
                let mut header_value = header.to_str().ok()?.clone().split(" ");
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

        Box::pin(async move {
            let context = match req.app_data::<Context>() {
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

            if let Some(token) = maybe_token {
                if let Ok(authenticated) = Auth::new(context).get_by_token(&token).await {
                    req.extensions_mut().insert(authenticated);
                }
            }

            svc.call(req).await
        })
    }
}
