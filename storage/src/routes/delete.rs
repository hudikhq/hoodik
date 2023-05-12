use std::str::FromStr;

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{TransactionTrait, Uuid};
use error::{AppResult, Error};

use crate::{contract::StorageProvider, repository::Repository, storage::Storage};

/// Delete a file or directory by its id
/// Also, deletes recursively all files and directories inside the directory
#[route("/api/storage/{file_id}", method = "DELETE")]
pub(crate) async fn delete(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;

    let mut files = Repository::new(&context.db)
        .manage(claims.sub)
        .file_tree(file_id)
        .await?;

    let mut ids = vec![];
    let connection = context.db.begin().await?;
    let repository = Repository::new(&connection);
    let manage = repository.manage(claims.sub);

    for file in files.iter_mut() {
        if !file.is_owner {
            return Err(Error::Unauthorized("not_your_file".to_string()));
        }

        if file.is_file() {
            Storage::new(&context.config)
                .purge(&file.get_filename().unwrap())
                .await?;
        }

        ids.push(file.id);
    }

    manage.delete_many(ids).await?;
    connection.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
