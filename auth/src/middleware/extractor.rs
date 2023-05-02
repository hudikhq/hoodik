use cached::proc_macro::cached;
use cached::{Cached, SizedCache};
use chrono::Utc;
use context::Context;
use cryptfns::sha256;

use crate::auth::Auth;
use crate::data::authenticated::Authenticated;
use crate::jwt;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum TokenExtractor {
    Header(String),
    Cookie(String),
}

#[derive(Clone, Debug)]
pub(crate) enum ExtractionResult {
    Token(String),
    Claims(String),
    None,
}

impl std::fmt::Display for ExtractionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let w = match self {
            ExtractionResult::Token(t) => {
                format!("Token({})", sha256::digest(t.as_bytes()))
            }
            ExtractionResult::Claims(c) => {
                format!("Claims({})", sha256::digest(c.as_bytes()))
            }
            ExtractionResult::None => "None".to_string(),
        };
        write!(f, "{}", w)
    }
}
pub(crate) async fn extract_session(
    extraction: &ExtractionResult,
    context: &Context,
) -> Option<Authenticated> {
    if let ExtractionResult::Claims(claims) = &extraction {
        match jwt::extract(claims, context.config.jwt_secret.as_str()) {
            Ok(authenticated) => {
                let token = authenticated.session.token.clone();

                return authenticated_cache(&token, context, Some(authenticated)).await;
            }
            Err(e) => {
                log::debug!("auth::middleware::load|jwt|verify: {}", e);
            }
        }
    }

    if let ExtractionResult::Token(token) = &extraction {
        return authenticated_cache(token, context, None).await;
    }

    None
}

/// Cached store and retrieve of the authenticated session
/// cache is kept in memory so if the application is restarted
/// the cache is cleared
#[cached(
    name = "AUTHENTICATED_CACHE",
    type = "SizedCache<String, Option<Authenticated>>",
    create = "{ SizedCache::with_size(100) }",
    convert = r#"{ format!("{}", &token) }"#
)]
async fn authenticated_cache(
    token: &str,
    context: &Context,
    authenticated: Option<Authenticated>,
) -> Option<Authenticated> {
    let auth = Auth::new(context);

    if let Some(authenticated) = authenticated {
        match auth.validate(authenticated.session.id).await {
            Ok(_) => return Some(authenticated),
            Err(e) => {
                log::debug!("auth::middleware::load|jwt|session-id-verify: {}", e);
            }
        }
    }

    match auth.get_by_token(token).await {
        Ok(authenticated) => {
            if authenticated.session.expires_at > Utc::now().naive_utc() {
                return Some(authenticated);
            } else {
                log::debug!("auth::middleware::load|token|expires_at_verify");
            }
        }
        Err(e) => {
            log::debug!("auth::middleware::load|token|error: {}", e);
        }
    }

    None
}

/// Remove the entry from the cache
pub(crate) async fn remove_authenticated_session(token: &String) {
    AUTHENTICATED_CACHE.lock().await.cache_remove(token);
}
