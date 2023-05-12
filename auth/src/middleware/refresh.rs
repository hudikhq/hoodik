//! Service wrapper that validates the session and refreshes it if it's expired.

use crate::{
    auth::Auth,
    data::{claims::Claims, extractor::Extractor},
};
use actix_web::{
    body::{EitherBody, MessageBody},
    cookie::Cookie,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{self, TryIntoHeaderValue},
    web, Error,
};
use context::Context;
use error::{AppResult, Error as AppError};
use futures_util::{
    future::{self, ok, LocalBoxFuture, Ready},
    FutureExt,
};

pub(crate) struct Refresh;

impl Refresh {
    pub fn new() -> Self {
        Refresh
    }
}

impl Default for Refresh {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, B> Transform<S, ServiceRequest> for Refresh
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RefreshMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(RefreshMiddleware { service })
    }
}

#[derive(Clone)]
pub struct RefreshMiddleware<S> {
    pub(crate) service: S,
}

impl<S, B> Service<ServiceRequest> for RefreshMiddleware<S>
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
        let context = match req.app_data::<web::Data<Context>>() {
            Some(c) => c,
            None => {
                let res =
                    req.error_response(AppError::InternalError("missing_context".to_string()));

                return ok(res.map_into_right_body()).boxed_local();
            }
        };

        let claims = match Extractor::default().jwt(context).service_req(&req) {
            Ok(a) => a,
            Err(err) => {
                let res = req.error_response(err);

                return ok(res.map_into_right_body()).boxed_local();
            }
        };

        let refresh_token = match Extractor::default().refresh(context).get(&req) {
            Ok(t) => t,
            Err(err) => {
                let res = req.error_response(err);

                return ok(res.map_into_right_body()).boxed_local();
            }
        };

        let ctx = context.clone();
        let first_fut = get_cookies(ctx, claims, refresh_token);
        let fut = self.service.call(req);

        Box::pin(async move {
            let (jwt, refresh) = first_fut.await?;
            let mut res = fut.await?;

            set_cookie(&mut res, jwt);
            set_cookie(&mut res, refresh);

            Ok(res.map_into_left_body())
        })
    }
}

fn set_cookie<B>(res: &mut ServiceResponse<B>, cookie: Cookie<'_>) {
    if let Ok(v) = cookie.to_string().try_into_value() {
        res.headers_mut().append(header::SET_COOKIE, v);
    }
}

/// Generate cookies for the session and refresh token.
async fn get_cookies<'ctx>(
    context: web::Data<Context>,
    claims: Claims,
    refresh_token: String,
) -> AppResult<(Cookie<'ctx>, Cookie<'ctx>)> {
    let auth = Auth::new(&context);
    let authenticated = auth.get_by_device_id(claims.device).await?;
    let authenticated = auth
        .refresh_session(&authenticated.session, &refresh_token)
        .await?;
    let (jwt, refresh) = auth
        .manage_cookies(&authenticated, module_path!(), false)
        .await?;

    Ok((jwt, refresh))
}
