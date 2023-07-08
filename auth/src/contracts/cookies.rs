use actix_web::cookie::{time::OffsetDateTime, Cookie, CookieBuilder, SameSite};
use chrono::{Duration, Utc};
use error::AppResult;

use crate::data::authenticated::Authenticated;

use super::ctx::Ctx;

/// Cookie management
pub(crate) trait Cookies
where
    Self: Ctx,
{
    /// Sets a cookie on the request
    fn manage_cookies(
        &self,
        authenticated: &Authenticated,
        issuer: &str,
    ) -> AppResult<(Cookie<'static>, Cookie<'static>)> {
        let destroy = authenticated.session.refresh.is_none();

        let mut refresh = authenticated
            .session
            .refresh
            .map(|r| r.to_string())
            .unwrap_or_else(|| "destroyed".to_string());

        if destroy && &refresh != "destroyed" {
            refresh = "destroyed".to_string();
        }

        let jwt = match destroy {
            true => "destroyed".to_string(),
            false => {
                crate::jwt::generate(authenticated, issuer, &self.ctx().config.auth.jwt_secret)?
            }
        };

        let jwt = self.make_cookie(
            Cookie::build(self.ctx().config.auth.session_cookie.clone(), jwt).path("/"),
            destroy,
        )?;

        let refresh = self.make_cookie(
            Cookie::build(self.ctx().config.auth.refresh_cookie.clone(), refresh)
                .path(crate::REFRESH_PATH),
            destroy,
        )?;

        Ok((jwt, refresh))
    }

    /// Set configuration parameters for cookie security
    fn make_cookie(
        &self,
        cookie: CookieBuilder<'static>,
        destroy: bool,
    ) -> AppResult<Cookie<'static>> {
        let mut cookie = cookie
            .secure(self.ctx().config.auth.cookie_secure)
            .http_only(self.ctx().config.auth.cookie_http_only)
            .finish();

        cookie.set_domain(self.ctx().config.auth.cookie_domain.clone());

        if destroy {
            cookie.set_expires(OffsetDateTime::from_unix_timestamp(0).unwrap());
        } else {
            let timestamp =
                Utc::now() + Duration::days(self.ctx().config.auth.long_term_session_duration_days);
            cookie.set_expires(OffsetDateTime::from_unix_timestamp(timestamp.timestamp()).unwrap());
        }

        match self.ctx().config.auth.cookie_same_site.as_ref() {
            "Lax" => cookie.set_same_site(SameSite::Lax),
            "Strict" => cookie.set_same_site(SameSite::Strict),
            _ => cookie.set_same_site(SameSite::None),
        };

        Ok(cookie)
    }
}
