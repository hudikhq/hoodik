use std::str::FromStr;

use actix_web::{
    http::header::{self, HeaderMap},
    HttpRequest,
};
use error::{AppResult, Error};

/// Parse given parameter easily from the request path string
pub fn path_var<T: FromStr>(req: &HttpRequest, name: &str) -> AppResult<T> {
    let value: T = match req.match_info().get(name) {
        Some(val) => match val.parse() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::BadRequest(format!(
                    "path_attribute_cannot_be_parsed:{name}",
                )));
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
                    "query_attribute_cannot_be_parsed:{name}",
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

/// Extract user agent and ip out of the request
pub fn extract_ip_ua(req: &HttpRequest) -> (String, String) {
    let remote_ip = get_ip(req.headers());

    let user_agent = match req.headers().get(header::USER_AGENT) {
        Some(header) => header.to_str().ok().map(|s| s.to_string()),
        None => None,
    };

    (
        user_agent.unwrap_or_else(|| "missing-header".to_string()),
        remote_ip,
    )
}

/// Extract ip from headers, try to get all the possible resolutions in order to find real ip address
pub fn get_ip(headers: &HeaderMap) -> String {
    let mut ip = "127.0.0.2".to_string();

    if headers.contains_key("cf-connecting-ip") && headers.get("cf-connecting-ip").is_some() {
        ip = headers
            .get("cf-connecting-ip")
            .unwrap()
            .to_str()
            .unwrap_or("127.0.0.2")
            .to_string();
    } else if headers.contains_key("x-real-ip") && headers.get("x-real-ip").is_some() {
        ip = headers
            .get("x-real-ip")
            .unwrap()
            .to_str()
            .unwrap_or("127.0.0.2")
            .to_string();
    } else if headers.contains_key("x-forwarded-for") && headers.get("x-forwarded-for").is_some() {
        let ip_string = headers
            .get("x-forwarded-for")
            .unwrap()
            .to_str()
            .unwrap_or("127.0.0.2")
            .to_string();

        ip = ip_string
            .split(',')
            .map(|item| item.to_owned())
            .collect::<Vec<String>>()
            .pop()
            .unwrap_or_else(|| "127.0.0.2".to_string());
    }

    ip
}
