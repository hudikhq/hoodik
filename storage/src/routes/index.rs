use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use error::AppResult;
use fs::prelude::*;

use crate::{data::query::Query, repository::Repository};

/// List files and directories
///
/// Request: [crate::data::query::Query]
///
/// Response: [crate::data::response::Response]
#[route("/api/storage", method = "GET")]
pub(crate) async fn index(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Query<Query>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let data = data.into_inner();
    let attributes = util::attributes::parse(data.attributes.as_deref());

    let mut response = Repository::new(&context.db)
        .manage(claims.sub)
        .find(data)
        .await?;

    for file in response.children.iter_mut() {
        if file.is_file() {
            let chunks = Fs::new(&context.config).get_uploaded_chunks(file).await?;

            file.chunks_stored = Some(chunks.len() as i64);
            file.uploaded_chunks = Some(chunks);
        }
    }

    let Some(keys) = attributes else {
        return Ok(HttpResponse::Ok().json(response));
    };

    let mut value = serde_json::to_value(&response)?;
    if let Some(rows) = value.get_mut("parents") {
        util::attributes::project_rows(rows, &keys);
    }
    if let Some(rows) = value.get_mut("children") {
        util::attributes::project_rows(rows, &keys);
    }

    Ok(HttpResponse::Ok().json(value))
}
