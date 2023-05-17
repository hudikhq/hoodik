pub mod actions;
pub mod auth;
pub mod contract;
pub mod data;
pub mod jwt;
pub mod providers;
pub mod routes;

mod emails;

pub(crate) const REFRESH_PATH: &str = "/api/auth/refresh";

#[cfg(test)]
mod test;

#[cfg(feature = "mock")]
/// Extract cookies from response headers
pub fn extract_cookies(
    headers: &actix_web::http::header::HeaderMap,
) -> (
    Option<actix_web::cookie::Cookie<'static>>,
    Option<actix_web::cookie::Cookie<'static>>,
) {
    let mut cookies = headers
        .get_all("set-cookie")
        .clone()
        .map(|h| {
            let h = h.clone().to_str().unwrap().to_string();

            actix_web::cookie::Cookie::parse(h).unwrap()
        })
        .collect::<Vec<actix_web::cookie::Cookie<'static>>>()
        .into_iter();

    let jwt = cookies.clone().find(|c| c.name() == "hoodik_session");
    let refresh = cookies.find(|c| c.name() == "hoodik_refresh");

    (jwt, refresh)
}
