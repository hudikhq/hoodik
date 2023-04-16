use std::str::FromStr;

use actix_web::HttpRequest;
use error::{AppResult, Error};

/// Parse given parameter easily from the request path string
pub fn path_var<T: FromStr>(req: &HttpRequest, name: &str) -> AppResult<T> {
    let value: T = match req.match_info().get(name) {
        Some(val) => match val.parse() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::BadRequest(format!(
                    "path_attribute_not_found:{name}",
                )))
            }
        },
        None => {
            return Err(Error::BadRequest(format!(
                "path_attribute_not_found:{name}",
            )))
        }
    };

    Ok(value)
}

/// Extract parameters from the url query string
pub fn query_var<T: FromStr>(req: &HttpRequest, name: &str) -> AppResult<T> {
    let value: T = match qstring::QString::from(req.query_string()).get(name) {
        Some(val) => match val.parse() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::BadRequest(format!(
                    "query_attribute_not_found:{name}",
                )))
            }
        },
        None => {
            return Err(Error::BadRequest(format!(
                "query_attribute_not_found:{name}",
            )))
        }
    };

    Ok(value)
}
