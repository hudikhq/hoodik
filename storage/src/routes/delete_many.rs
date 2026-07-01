use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{
    permission::{permissions_for, SharePermission},
    TransactionTrait,
};
use error::{AppResult, Error};
use fs::prelude::*;

use crate::{data::delete_many::DeleteMany, repository::Repository};

/// Bulk delete. Each id is dispatched by ownership — owner ids
/// cascade, non-owner ids self-remove, ids with no row are 403. All
/// happens in one DB transaction so a partial failure rolls back the
/// caller's view of every requested target.
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

    let perms = permissions_for(&connection, &ids, claims.sub).await?;
    let mut to_cascade: Vec<entity::Uuid> = Vec::new();
    let mut to_self_remove: Vec<entity::Uuid> = Vec::new();
    for id in &ids {
        match perms.get(id).copied().unwrap_or(SharePermission::None) {
            SharePermission::Owner => to_cascade.push(*id),
            SharePermission::CoOwner | SharePermission::Editor | SharePermission::Reader => {
                to_self_remove.push(*id)
            }
            SharePermission::None => {
                return Err(Error::Forbidden("forbidden_not_owner".to_string()));
            }
        }
    }

    let mut files = if to_cascade.is_empty() {
        Vec::new()
    } else {
        Repository::new(&connection)
            .manage(claims.sub)
            .delete_many(to_cascade)
            .await?
    };
    if !to_self_remove.is_empty() {
        Repository::new(&connection)
            .manage(claims.sub)
            .self_remove_recursive(to_self_remove)
            .await?;
    }
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
