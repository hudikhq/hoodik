use actix_cors::Cors;
use actix_web::http;
use std::str::FromStr;

pub fn setup() -> Cors {
    let expose = vec!["content-type", "cache-control", "content-length"];

    Cors::default()
        .allow_any_origin()
        .allowed_origin_fn(move |_, _| true)
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .expose_headers(expose)
        .allowed_headers(vec![
            http::header::CONTENT_TYPE,
            http::header::ACCEPT,
            http::header::ORIGIN,
            http::header::AUTHORIZATION,
            http::header::HeaderName::from_str("X-Key-Fingerprint").unwrap(),
        ])
        .max_age(3600)
}
