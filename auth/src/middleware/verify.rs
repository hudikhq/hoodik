use actix_web::{
    body::BoxBody,
    dev::ServiceResponse,
    dev::{Service, ServiceRequest, Transform},
    Error, HttpMessage, ResponseError,
};
use error::Error as AppError;
use futures_util::future::{ok, LocalBoxFuture, Ready};

use crate::data::authenticated::Authenticated;

#[derive(PartialEq, Debug, Clone)]
pub enum CsrfVerify {
    Header(String),
    Query(String),
}

pub struct Verify {
    pub(crate) csrf_verify: Option<CsrfVerify>,
}

impl Verify {
    pub fn new() -> Verify {
        Verify { csrf_verify: None }
    }
    pub fn csrf_header_name(mut self, name: String) -> Self {
        self.csrf_verify = Some(CsrfVerify::Header(name));

        self
    }

    pub fn csrf_query_name(mut self, name: String) -> Self {
        self.csrf_verify = Some(CsrfVerify::Query(name));

        self
    }
}
impl<S> Transform<S, ServiceRequest> for Verify
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = VerifyMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(VerifyMiddleware {
            service,
            csrf_verify: self.csrf_verify.clone(),
        })
    }
}

pub struct VerifyMiddleware<S> {
    service: S,
    csrf_verify: Option<CsrfVerify>,
}

impl<S> VerifyMiddleware<S> {
    fn should_verify_csrf(&self) -> bool {
        self.csrf_verify.is_some()
    }

    fn extract_csrf(&self, req: &ServiceRequest) -> Option<String> {
        if let Some(csrf_verify) = &self.csrf_verify {
            match csrf_verify {
                CsrfVerify::Header(name) => {
                    let header = req.headers().get(name)?;
                    let csrf = header.to_str().ok()?;
                    Some(csrf.to_string())
                }
                CsrfVerify::Query(name) => {
                    let query = req.query_string().to_string().replace("?", "");
                    let mut query_value = query.split("&");

                    let csrf = query_value
                        .find(|v| v.starts_with(format!("{name}=").as_str()))
                        .map(|v| v.split("=").nth(1))?;

                    csrf.map(|i| i.to_string())
                }
            }
        } else {
            None
        }
    }
}

impl<S> Service<ServiceRequest> for VerifyMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let csrf = req
            .extensions()
            .get::<Authenticated>()
            .map(|a| a.session.csrf.clone());

        if csrf.is_none() {
            return Box::pin(async move {
                Ok(ServiceResponse::new(
                    req.into_parts().0,
                    AppError::Unauthorized("no_session".to_string()).error_response(),
                ))
            });
        }

        if self.should_verify_csrf() {
            if self.extract_csrf(&req) != csrf {
                return Box::pin(async move {
                    Ok(ServiceResponse::new(
                        req.into_parts().0,
                        AppError::Unauthorized("csrf_mismatch".to_string()).error_response(),
                    ))
                });
            }
        }

        let fut = self.service.call(req);
        Box::pin(async move { fut.await })
    }
}
