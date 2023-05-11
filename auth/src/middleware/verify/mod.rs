//! Session verification middleware that will verify if the request has a valid user and session.
//! It can also serve as a session refresher.

mod extractor;
mod refreshing;
mod regular;

use actix_web::{
    body::{EitherBody, MessageBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures_util::future::{self, Ready};

#[derive(Clone)]
pub struct Verify<T: Clone> {
    _refresh: T,
}

#[derive(Clone)]
pub struct Refresh;

#[derive(Clone)]
pub struct NoRefresh;

impl Verify<Refresh> {
    pub fn new_refresh() -> Verify<Refresh> {
        Verify { _refresh: Refresh }
    }
}

impl Verify<NoRefresh> {
    pub fn new_regular() -> Verify<NoRefresh> {
        Verify {
            _refresh: NoRefresh,
        }
    }
}

impl Default for Verify<NoRefresh> {
    fn default() -> Self {
        Self::new_regular()
    }
}

impl<S, B> Transform<S, ServiceRequest> for Verify<NoRefresh>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = regular::VerifyMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(regular::VerifyMiddleware { service })
    }
}

impl<S, B> Transform<S, ServiceRequest> for Verify<Refresh>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = refreshing::VerifyMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(refreshing::VerifyMiddleware { service })
    }
}
