use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use error::AppResult;

use crate::{data::search::Search, repository::Repository};

/// List files and directories
///
/// Request: [crate::data::search::Search]
///
/// Response: [Vec<crate::data::app_file::AppFile>]
#[route("/api/storage/search", method = "POST", wrap = "Verify::default()")]
pub(crate) async fn search(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Json<Search>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;

    let data = data.into_inner();

    let file = Repository::new(&context.db)
        .tokens(&authenticated.user)
        .search(data)
        .await?;

    Ok(HttpResponse::Ok().json(file))
}
