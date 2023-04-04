use actix_cors::Cors;
use actix_web::http;
use std::str::FromStr;

pub fn setup() -> Cors {
    Cors::default()
        .supports_credentials()
        .allow_any_origin()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allowed_headers(vec![
            http::header::CONTENT_TYPE,
            http::header::ACCEPT,
            http::header::ORIGIN,
            http::header::AUTHORIZATION,
            http::header::HeaderName::from_str("X-Key-Fingerprint").unwrap(),
        ])
        .max_age(3600)
}
