use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use error::AppResult;

use crate::{data::search::Search, repository::Repository};

/// List files and directories
///
/// Request: [crate::data::search::Search]
///
/// Response: [Vec<crate::data::app_file::AppFile>]
#[route("/api/storage/search", method = "POST")]
pub(crate) async fn search(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<Search>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();

    let data = data.into_inner();
    let attributes = util::attributes::parse(data.attributes.as_deref());

    let files = Repository::new(&context.db)
        .tokens(claims.sub)
        .search(data)
        .await?;

    let Some(keys) = attributes else {
        return Ok(HttpResponse::Ok().json(files));
    };

    let mut value = serde_json::to_value(&files)?;
    util::attributes::project_rows(&mut value, &keys);

    Ok(HttpResponse::Ok().json(value))
}
