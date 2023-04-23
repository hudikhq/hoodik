use actix_cors::Cors;

pub fn setup() -> Cors {
    Cors::permissive()
    // .allow_any_origin()
    // .allow_any_method()
    // .expose_any_header()
    // .allow_any_header()
    // .send_wildcard()
    // .max_age(3600)
}
