//! Service wrapper for only doing the JWT session verification.

use actix_web::{
    body::{EitherBody, MessageBody},
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse},
    web, Error, HttpMessage,
};
use context::Context;
use error::Error as AppError;
use futures_util::{
    future::{ok, LocalBoxFuture},
    FutureExt,
};

use super::extractor::Extractor;

#[derive(Clone)]
pub struct VerifyMiddleware<S> {
    pub(crate) service: S,
}

impl<S, B> Service<ServiceRequest> for VerifyMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,

    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<ServiceResponse<EitherBody<B>>, Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let authenticated = match req
            .app_data::<web::Data<Context>>()
            .map(|ctx| Extractor::default().jwt(ctx).get(&req))
        {
            Some(Ok(a)) => a,
            Some(Err(err)) => {
                let res = req.error_response(err);

                return ok(res.map_into_right_body()).boxed_local();
            }
            None => {
                let res =
                    req.error_response(AppError::InternalError("missing_context".to_string()));

                return ok(res.map_into_right_body()).boxed_local();
            }
        };

        if authenticated.is_expired() {
            let res = req.error_response(AppError::Unauthorized("expired".to_string()));

            return ok(res.map_into_right_body()).boxed_local();
        }

        req.extensions_mut().insert(authenticated);

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await;

            Ok(res?.map_into_left_body())
        })
    }
}
