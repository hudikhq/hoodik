pub mod actions;
pub mod auth;
pub mod contract;
pub mod data;
pub mod jwt;
pub mod providers;
pub mod routes;

mod emails;
#[cfg(test)]
mod test;

pub(crate) const REFRESH_PATH: &str = "/api/auth/refresh";

#[cfg(feature = "mock")]
/// Extract cookies from response headers
pub fn extract_cookies(
    headers: &actix_web::http::header::HeaderMap,
) -> (
    Option<actix_web::cookie::Cookie<'static>>,
    Option<actix_web::cookie::Cookie<'static>>,
) {
    let cookies = headers
        .get_all("set-cookie")
        .clone()
        .map(|h| {
            let h = h.clone().to_str().unwrap().to_string();

            actix_web::cookie::Cookie::parse(h).unwrap()
        })
        .collect::<Vec<actix_web::cookie::Cookie<'static>>>()
        .into_iter();

    let jwt = cookies
        .clone()
        .filter(|c| c.name() == "hoodik_session")
        .next();
    let refresh = cookies.filter(|c| c.name() == "hoodik_refresh").next();

    (jwt, refresh)
}
