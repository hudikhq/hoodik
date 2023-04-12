use std::str::FromStr;

use actix_web::HttpRequest;
use error::{AppResult, Error};

/// Parse given parameter easily from the request path string
pub fn path_var<T: FromStr>(req: &HttpRequest, name: &str) -> AppResult<T> {
    let value: T = match req.match_info().get(name) {
        Some(val) => match val.parse() {
            Ok(v) => v,
            Err(_) => return Err(Error::BadRequest(format!("attribute_not_found:{name}",))),
        },
        None => return Err(Error::BadRequest(format!("attribute_not_found:{name}",))),
    };

    Ok(value)
}
