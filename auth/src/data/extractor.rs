use std::marker::PhantomData;

use crate::data::claims::Claims;
use actix_web::HttpRequest;
use context::Context;
use entity::Uuid;
use error::AppResult;

pub(crate) struct Extractor<'ext, T> {
    extractor: T,
    _p: &'ext PhantomData<()>,
}

pub(crate) struct Useless;
pub(crate) struct Refresh<'ext>(&'ext str);
pub(crate) struct Jwt<'ext> {
    cookie_name: &'ext str,
    jwt_secret: &'ext str,
}

impl Extractor<'_, Useless> {
    fn new() -> Self {
        Extractor {
            extractor: Useless,
            _p: &PhantomData,
        }
    }
}

impl Default for Extractor<'_, Useless> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'ext> Extractor<'ext, Useless> {
    pub(crate) fn jwt(self, ctx: &'ext Context) -> Extractor<'ext, Jwt<'ext>> {
        Extractor {
            extractor: Jwt {
                cookie_name: &ctx.config.session_cookie,
                jwt_secret: &ctx.config.jwt_secret,
            },
            _p: &PhantomData,
        }
    }

    pub(crate) fn refresh(self, ctx: &'ext Context) -> Extractor<'ext, Refresh<'ext>> {
        Extractor {
            extractor: Refresh(&ctx.config.refresh_cookie),
            _p: &PhantomData,
        }
    }
}

impl<'ext> Extractor<'ext, Jwt<'ext>> {
    fn cookie_name(&self) -> &'ext str {
        self.extractor.cookie_name
    }

    fn jwt_secret(&self) -> &'ext str {
        self.extractor.jwt_secret
    }

    /// Extract the authenticated session from the regular request and verify it.
    ///
    /// It does not verify if the session is expired!
    pub(crate) fn req(&self, req: &HttpRequest) -> AppResult<Claims> {
        let cookie = req
            .cookie(self.cookie_name())
            .ok_or_else(|| error::Error::Unauthorized("missing_session_token".to_string()))?;

        crate::jwt::extract(cookie.value(), self.jwt_secret())
    }
}

impl<'ext> Extractor<'ext, Refresh<'ext>> {
    fn cookie_name(&self) -> &'ext str {
        self.extractor.0
    }

    /// Extract the refresh token from the request.
    pub(crate) fn req(&self, req: &HttpRequest) -> AppResult<Uuid> {
        let cookie = req
            .cookie(self.cookie_name())
            .ok_or_else(|| error::Error::Unauthorized("missing_refresh_token".to_string()))?;

        Uuid::parse_str(cookie.value())
            .map_err(|_| error::Error::Unauthorized("invalid_refresh_token".to_string()))
    }
}
