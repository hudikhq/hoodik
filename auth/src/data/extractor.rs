//! # Request data extractors
use crate::data::claims::Claims;
use actix_web::HttpRequest;
use context::Context;
use entity::Uuid;
use error::AppResult;
use std::marker::PhantomData;

#[derive(Debug)]
pub(crate) enum Source<'ext> {
    Http(&'ext str),
    Header(&'ext str),
}

pub(crate) struct Extractor<'ext, T> {
    extractor: T,
    _p: &'ext PhantomData<()>,
}

pub(crate) struct Useless;
pub(crate) struct Refresh<'ext>(Source<'ext>);

pub(crate) struct Jwt<'ext> {
    source: Source<'ext>,
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
        let source = if ctx.config.auth.use_headers_for_auth {
            Source::Header("Authorization")
        } else {
            Source::Http(&ctx.config.auth.session_cookie)
        };

        Extractor {
            extractor: Jwt {
                source,
                jwt_secret: &ctx.config.auth.jwt_secret,
            },
            _p: &PhantomData,
        }
    }

    pub(crate) fn refresh(self, ctx: &'ext Context) -> Extractor<'ext, Refresh<'ext>> {
        let source = if ctx.config.auth.use_headers_for_auth {
            Source::Header("x-auth-refresh")
        } else {
            Source::Http(&ctx.config.auth.refresh_cookie)
        };

        Extractor {
            extractor: Refresh(source),
            _p: &PhantomData,
        }
    }
}

impl<'ext> Extractor<'ext, Jwt<'ext>> {
    fn name(&self) -> &'ext str {
        match self.extractor.source {
            Source::Http(name) | Source::Header(name) => name,
        }
    }

    fn use_headers(&self) -> bool {
        matches!(self.extractor.source, Source::Header(_))
    }

    fn jwt_secret(&self) -> &'ext str {
        self.extractor.jwt_secret
    }

    /// Extract the authenticated session from the regular request and verify it.
    ///
    /// It does not verify if the session is expired!
    pub(crate) fn req(&self, req: &HttpRequest) -> AppResult<Claims> {
        let claims = if self.use_headers() {
            self.req_header(req)?
        } else {
            self.req_cookie(req)?
        };

        crate::jwt::extract(&claims, self.jwt_secret())
    }

    fn req_cookie(&self, req: &HttpRequest) -> AppResult<String> {
        let cookie = req
            .cookie(self.name())
            .ok_or_else(|| error::Error::Unauthorized("missing_session_token".to_string()))?;

        Ok(cookie.value().to_string())
    }

    fn req_header(&self, req: &HttpRequest) -> AppResult<String> {
        let header = req
            .headers()
            .get(self.name())
            .ok_or_else(|| error::Error::Unauthorized("missing_session_token".to_string()))?
            .to_str()
            .map_err(|_| error::Error::Unauthorized("invalid_session_token".to_string()))?;

        let header = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| error::Error::Unauthorized("invalid_session_token".to_string()))?;

        Ok(header.to_string())
    }
}

impl<'ext> Extractor<'ext, Refresh<'ext>> {
    fn name(&self) -> &'ext str {
        match self.extractor.0 {
            Source::Http(name) | Source::Header(name) => name,
        }
    }

    fn use_headers(&self) -> bool {
        matches!(self.extractor.0, Source::Header(_))
    }

    /// Extract the refresh token from the request.
    pub(crate) fn req(&self, req: &HttpRequest) -> AppResult<Uuid> {
        if self.use_headers() {
            self.req_header(req)
        } else {
            self.req_cookie(req)
        }
    }

    fn req_cookie(&self, req: &HttpRequest) -> AppResult<Uuid> {
        let cookie = req
            .cookie(self.name())
            .ok_or_else(|| error::Error::Unauthorized("missing_refresh_token".to_string()))?;

        Uuid::parse_str(cookie.value())
            .map_err(|_| error::Error::Unauthorized("invalid_refresh_token".to_string()))
    }

    fn req_header(&self, req: &HttpRequest) -> AppResult<Uuid> {
        let header = req
            .headers()
            .get(self.name())
            .ok_or_else(|| error::Error::Unauthorized("missing_refresh_token".to_string()))?
            .to_str()
            .map_err(|_| error::Error::Unauthorized("invalid_refresh_token".to_string()))?;

        Uuid::parse_str(header)
            .map_err(|_| error::Error::Unauthorized("invalid_refresh_token".to_string()))
    }
}
