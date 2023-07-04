use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::TransactionTrait;
use error::AppResult;
use fs::prelude::*;

use crate::{data::delete_many::DeleteMany, repository::Repository};

/// Delete many files and folders with their children recursively
/// all at once.
///
/// Request: [crate::data::delete_many::DeleteMany]
#[route("/api/storage/delete-many", method = "POST")]
pub(crate) async fn delete_many(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<DeleteMany>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let ids = data.into_inner().into_value()?;

    let fs = Fs::new(&context.config);
    let connection = context.db.begin().await?;
    let mut files = Repository::new(&connection)
        .manage(claims.sub)
        .delete_many(ids)
        .await?;
    connection.commit().await?;

    for file in files.iter_mut() {
        if file.is_file() {
            fs.purge(file).await?;
        }
    }

    Ok(HttpResponse::NoContent().finish())
}
