use ::auth::middleware::load::Load;
use actix_web::{
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web, App, HttpServer,
};
use context::Context;

pub mod auth;

pub fn app(
    context: Context,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let cookie_name = context.config.get_cookie_name();

    App::new()
        .app_data(web::Data::new(context))
        // Authentication load middleware that only sets it up on the app
        .wrap(Load::new().token_cookie_name(cookie_name))
        // PRETTY PLEASE: keep the routes in alphabetical order
        //  There is a VSCode extension "Alphabetical Sorter" that can help you with this
        .service(auth::login)
        .service(auth::refresh)
        .service(auth::register)
}

/// Start the server
pub async fn engage(context: Context) -> std::io::Result<()> {
    let bind_address = context.config.get_full_bind_address();

    HttpServer::new(move || app(context.clone()))
        .bind(&bind_address)?
        .run()
        .await
}
