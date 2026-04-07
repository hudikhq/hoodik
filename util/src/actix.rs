use std::str::FromStr;

use actix_web::{
    http::header::{self},
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
    let remote_ip = get_ip(req);

    let user_agent = match req.headers().get(header::USER_AGENT) {
        Some(header) => header.to_str().ok().map(|s| s.to_string()),
        None => None,
    };

    (
        user_agent.unwrap_or_else(|| "missing-header".to_string()),
        remote_ip,
    )
}

/// Extract ip from the request, checking proxy headers first, then falling back to peer_addr
fn get_ip(req: &HttpRequest) -> String {
    let headers = req.headers();

    if let Some(value) = headers.get("cf-connecting-ip") {
        if let Ok(ip) = value.to_str() {
            let trimmed = ip.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    if let Some(value) = headers.get("x-real-ip") {
        if let Ok(ip) = value.to_str() {
            let trimmed = ip.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    if let Some(value) = headers.get("x-forwarded-for") {
        if let Ok(ip_string) = value.to_str() {
            // X-Forwarded-For: client, proxy1, proxy2
            // The first entry is the original client IP
            if let Some(first) = ip_string.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    // Fallback to the TCP peer address
    req.peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn test_xff_takes_first_ip() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "1.2.3.4, 5.6.7.8, 9.10.11.12"))
            .to_http_request();
        assert_eq!(get_ip(&req), "1.2.3.4");
    }

    #[test]
    fn test_xff_single_ip() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", "1.2.3.4"))
            .to_http_request();
        assert_eq!(get_ip(&req), "1.2.3.4");
    }

    #[test]
    fn test_xff_trims_whitespace() {
        let req = TestRequest::default()
            .insert_header(("x-forwarded-for", " 1.2.3.4 , 5.6.7.8"))
            .to_http_request();
        assert_eq!(get_ip(&req), "1.2.3.4");
    }

    #[test]
    fn test_cf_connecting_ip_priority() {
        let req = TestRequest::default()
            .insert_header(("cf-connecting-ip", "10.0.0.1"))
            .insert_header(("x-forwarded-for", "1.2.3.4"))
            .to_http_request();
        assert_eq!(get_ip(&req), "10.0.0.1");
    }

    #[test]
    fn test_x_real_ip_priority_over_xff() {
        let req = TestRequest::default()
            .insert_header(("x-real-ip", "10.0.0.2"))
            .insert_header(("x-forwarded-for", "1.2.3.4"))
            .to_http_request();
        assert_eq!(get_ip(&req), "10.0.0.2");
    }

    #[test]
    fn test_fallback_no_headers() {
        let req = TestRequest::default().to_http_request();
        assert_eq!(get_ip(&req), "127.0.0.1");
    }

    #[test]
    fn test_empty_cf_connecting_ip_falls_through() {
        let req = TestRequest::default()
            .insert_header(("cf-connecting-ip", "  "))
            .insert_header(("x-real-ip", "10.0.0.3"))
            .to_http_request();
        assert_eq!(get_ip(&req), "10.0.0.3");
    }
}
