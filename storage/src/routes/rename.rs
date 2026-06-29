use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{
    permission::{permission, SharePermission},
    TransactionTrait, Uuid,
};
use error::{AppResult, Error};

use crate::{data::rename::Rename, repository::Repository};

/// Rename a file or a folder
///
/// Request: [crate::data::rename::Rename]
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage/{file_id}", method = "PUT")]
pub(crate) async fn rename(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<Rename>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let connection = context.db.begin().await?;
    let repository = Repository::new(&connection);
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;

    // Owner can rename anything; non-owner writers can
    // only rename editable files that aren't directories. The
    // editable-or-directory check needs the file's metadata before we
    // run the actual update, so we cross-check here.
    let perm = permission(&connection, file_id, claims.sub).await?;
    match perm {
        SharePermission::Owner => {}
        SharePermission::CoOwner | SharePermission::Editor => {
            let target = repository
                .by_id(file_id, claims.sub)
                .await?;
            if target.mime == "dir" || !target.editable {
                return Err(Error::Forbidden("forbidden_read_only".to_string()));
            }
        }
        SharePermission::Reader => {
            return Err(Error::Forbidden("forbidden_read_only".to_string()));
        }
        SharePermission::None => {
            return Err(Error::NotFound("file_not_found".to_string()));
        }
    }

    let file = repository
        .manage(claims.sub)
        .rename(file_id, data.into_inner())
        .await?;

    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}
