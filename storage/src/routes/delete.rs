use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{
    permission::{permission, SharePermission},
    TransactionTrait, Uuid,
};
use error::{AppResult, Error};
use fs::prelude::*;

use crate::repository::Repository;

/// Delete a file or directory by id. Owner → cascade delete (file +
/// chunks + recipients). Reader / Editor / Co-owner → self-remove only
/// (drop caller's `user_files` row, recursively for folders). None →
/// 403.
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

    let perm = permission(&connection, file_id, claims.sub).await?;
    match perm {
        SharePermission::Owner => {
            let mut files = Repository::new(&connection)
                .manage(claims.sub)
                .delete_many(vec![file_id])
                .await?;
            connection.commit().await?;
            for file in files.iter_mut() {
                if file.is_file() {
                    fs.purge_all(file).await?;
                }
            }
        }
        SharePermission::CoOwner | SharePermission::Editor | SharePermission::Reader => {
            Repository::new(&connection)
                .manage(claims.sub)
                .self_remove_recursive(vec![file_id])
                .await?;
            connection.commit().await?;
        }
        SharePermission::None => {
            return Err(Error::Forbidden("forbidden_not_owner".to_string()));
        }
    }

    Ok(HttpResponse::NoContent().finish())
}
