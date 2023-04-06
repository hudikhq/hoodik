//! # Proxy module
//!
//! This module contains a catch-all route that will proxy all requests that
//! didn't match any of the routes in the backend server to the frontend server.
use actix_web::{web, HttpRequest, HttpResponse, Responder, ResponseError};
use context::Context;
use error::Error;
use reqwest::Client;

/// Catch all requests that don't match any internal routes and forward them to the frontend
pub async fn http(context: web::Data<Context>, req: HttpRequest) -> impl Responder {
    let client = context.config.get_client_url();
    let url = format!("{}{}?{}", client, req.path(), req.query_string());

    log::debug!("Forwarding request to {}", &url);

    let response = match Client::new()
        .request(req.method().clone(), url)
        .send()
        .await
        .map_err(Error::from)
        .map_err(|e| e.error_response())
    {
        Ok(response) => response,
        Err(e) => return e,
    };

    let mut builder = HttpResponse::build(response.status());

    for (key, value) in response.headers() {
        builder.append_header((key, value));
    }

    let body = match response
        .bytes()
        .await
        .map_err(Error::from)
        .map_err(|e| e.error_response())
    {
        Ok(body) => body,
        Err(e) => return e,
    };

    builder.body(body)
}
