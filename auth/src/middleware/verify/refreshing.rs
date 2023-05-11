//! Service wrapper that validates the session and refreshes it if it's expired.

use actix_web::{
    body::{EitherBody, MessageBody},
    cookie::Cookie,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse},
    http::header::{self, TryIntoHeaderValue},
    web, Error,
};
use context::Context;
use error::{AppResult, Error as AppError};
use futures_util::{
    future::{ok, LocalBoxFuture},
    FutureExt,
};

use crate::{auth::Auth, data::authenticated::Authenticated};

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
        let context = match req.app_data::<web::Data<Context>>() {
            Some(c) => c,
            None => {
                let res =
                    req.error_response(AppError::InternalError("missing_context".to_string()));

                return ok(res.map_into_right_body()).boxed_local();
            }
        };

        let authenticated = match Extractor::default().jwt(context).get(&req) {
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
        let first_fut = get_cookies(ctx, authenticated, refresh_token);
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
    authenticated: Authenticated,
    refresh_token: String,
) -> AppResult<(Cookie<'ctx>, Cookie<'ctx>)> {
    let auth = Auth::new(&context);
    let authenticated = auth
        .refresh_session(&authenticated.session, &refresh_token)
        .await?;
    let (jwt, refresh) = auth.manage_cookies(&authenticated, false).await?;

    Ok((jwt, refresh))
}
