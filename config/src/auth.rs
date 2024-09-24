use url::Url;

use crate::{app::AppConfig, vars::Vars};

#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT_SECRET secret that will be used to sign the JWT tokens
    /// if you don't set this it will generate a random secret every time
    /// the application restarts, that means that all the sessions will be
    /// invalidated every time the application restarts.
    ///
    /// *optional*
    ///
    /// default: generates a random secret
    pub jwt_secret: String,

    /// APP_COOKIE_DOMAIN: If the backend is working by using cookies and not JWT this will be used as the cookie domain.
    /// it automatically defaults to be the same as the APP_URL
    ///
    /// *optional*
    pub cookie_domain: String,

    /// SESSION_COOKIE This should be the name of the cookie that will be used to store the session
    /// in your browser it is not that important and you probably don't need to set it
    ///
    /// *optional*
    ///
    /// *default: hoodik_session*
    pub session_cookie: String,

    /// REFRESH_COOKIE This is the cookie name of the refresh token that will be used to refresh the session
    /// alongside the session_cookie.
    ///
    /// *optional*
    ///
    /// *default: hoodik_refresh*
    pub refresh_cookie: String,

    /// COOKIE_HTTP_ONLY This tells us if the cookie is supposed to be http only or not. Http only cookie will
    /// only be seen by the browser and not by the javascript frontend. This is okay and its supposed
    /// to work like this.
    ///
    /// *optional*
    ///
    /// *default: true*
    pub cookie_http_only: bool,

    /// COOKIE_SECURE This tells us if the cookie is supposed to be secure or not. Secure cookie will
    /// only be sent over https.
    ///
    /// *optional*
    ///
    /// *default: true*
    pub cookie_secure: bool,

    /// COOKIE_SAME_SITE: This tells us if the cookie is supposed to be same site or not. Same site cookie will
    /// only be sent over same site.
    ///
    /// *optional*
    ///
    /// *default: Lax*
    ///
    /// *possible values: Lax, Strict, None*
    pub cookie_same_site: String,

    /// LONG_TERM_SESSION_DURATION_DAYS: This tells us for how long
    /// will the session be refreshed if the user is not using the application.
    ///
    /// *optional*
    ///
    /// default: 30
    pub long_term_session_duration_days: i64,

    /// SHORT_TERM_SESSION_DURATION_SECONDS: This is the period of time that the user will be logged in
    /// if he leaves the application (web client).
    /// While the user is browsing the application the session will keep extending for this period of time.
    ///
    /// *optional*
    ///
    /// default: 120
    pub short_term_session_duration_seconds: i64,

    /// USE_HEADERS_FOR_AUTH: This tells us if the headers should be used for authentication
    /// instead of cookies. This method will be less secure, but will allow you to use the backend
    /// with a frontend that is not on the same domain, or if you have multiple domains.
    /// 
    /// *optional*
    /// 
    /// default: false
    pub use_headers_for_auth: bool,
}

impl AuthConfig {
    pub(crate) fn new(app: &AppConfig, vars: &mut Vars) -> Self {
        let jwt_secret = vars.var_default("JWT_SECRET", uuid::Uuid::new_v4().to_string());
        let session_cookie = vars.var_default("SESSION_COOKIE", "hoodik_session".to_string());
        let refresh_cookie = vars.var_default("REFRESH_COOKIE", "hoodik_refresh".to_string());
        let cookie_http_only = vars.var_default("COOKIE_HTTP_ONLY", true);
        let cookie_secure = vars.var_default("COOKIE_SECURE", true);
        let cookie_same_site = parse_cookie_same_site(vars);
        let long_term_session_duration_days =
            vars.var_default("LONG_TERM_SESSION_DURATION_DAYS", 30);
        let short_term_session_duration_seconds =
            vars.var_default("SHORT_TERM_SESSION_DURATION_SECONDS", 120);
        let use_headers_for_auth = vars.var_default("USE_HEADERS_FOR_AUTH", false);

        let cookie_domain = get_cookie_domain(vars, &app.app_url);

        vars.panic_if_errors("AuthConfig");

        Self {
            cookie_domain,
            jwt_secret: jwt_secret.get(),
            session_cookie: session_cookie.get(),
            refresh_cookie: refresh_cookie.get(),
            cookie_http_only: cookie_http_only.get(),
            cookie_secure: cookie_secure.get(),
            cookie_same_site,
            long_term_session_duration_days: long_term_session_duration_days.get(),
            short_term_session_duration_seconds: short_term_session_duration_seconds.get(),
            use_headers_for_auth: use_headers_for_auth.get(),
        }
    }
}

fn parse_cookie_same_site(vars: &mut Vars) -> String {
    let value = vars.maybe_var::<String>("COOKIE_SAME_SITE").maybe_get();

    let mut value = match value {
        Some(x) => {
            if matches!(x.to_lowercase().as_str(), "strict" | "lax" | "none") {
                x
            } else {
                "lax".to_string()
            }
        }
        _ => "lax".to_string(),
    };

    if let Some(f) = value.get_mut(0..1) {
        f.make_ascii_uppercase();
    }

    value
}

fn get_cookie_domain(vars: &mut Vars, app_url: &Url) -> String {
    let cookie_domain = vars.var_default("COOKIE_DOMAIN", "".to_string()).get();

    let app_url_domain = app_url
        .host_str()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "localhost".to_string());

    let maybe_url = Url::parse(format!("https://{}", &cookie_domain).as_str());

    if maybe_url.is_err() {
        log::warn!(
                "'{}' couldn't be parsed into a cookie domain, will use app_url host '{}' for cookie domain",
                &cookie_domain,
                app_url_domain
            );

        return app_url_domain;
    }

    cookie_domain
}
