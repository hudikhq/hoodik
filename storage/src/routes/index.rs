use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use error::AppResult;

use crate::{
    contract::StorageProvider, data::query::Query, repository::Repository, storage::Storage,
};

/// List files and directories
///
/// Request: [crate::data::query::Query]
///
/// Response: [crate::data::response::Response]
#[route("/api/storage", method = "GET")]
pub async fn index(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Query<Query>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();

    let mut response = Repository::new(&context.db)
        .manage(claims.sub)
        .find(data.into_inner())
        .await?;

    for file in response.children.iter_mut() {
        if file.is_file() {
            let filename = file.get_filename().unwrap();
            let chunks = Storage::new(&context.config)
                .get_uploaded_chunks(&filename)
                .await?;
            file.chunks_stored = Some(chunks.len() as i32);
            file.uploaded_chunks = Some(chunks);
        }
    }

    Ok(HttpResponse::Ok().json(response))
}
