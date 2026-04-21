use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{TransactionTrait, Uuid};
use error::AppResult;
use fs::prelude::*;

use crate::repository::Repository;

/// Delete a file or directory by its id
/// Also, deletes recursively all files and directories inside the directory
#[route("/api/storage/{file_id}", method = "DELETE")]
pub(crate) async fn delete(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;

    let fs = Fs::new(&context.config);
    let connection = context.db.begin().await?;
    let mut files = Repository::new(&connection)
        .manage(claims.sub)
        .delete_many(vec![file_id])
        .await?;
    connection.commit().await?;

    for file in files.iter_mut() {
        if file.is_file() {
            // purge_all wipes every version directory AND any leftover
            // legacy chunks — safe on files never touched post-migration.
            fs.purge_all(file).await?;
        }
    }

    Ok(HttpResponse::NoContent().finish())
}
