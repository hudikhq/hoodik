use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{permission::permissions_for, TransactionTrait};
use error::{AppResult, Error};

use crate::{data::move_many::MoveMany, repository::Repository};

/// Moves many files and folders into a new parent folder
///
/// Request: [crate::data::move_many::MoveMany]
#[route("/api/storage/move-many", method = "POST")]
pub(crate) async fn move_many(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<MoveMany>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let (ids, file_id) = data.into_inner().into_value()?;

    let connection = context.db.begin().await?;

    // Write access is required on every source id. The
    // destination's permission lives outside this route — callers move
    // into shared folders via `POST /api/storage/move-into-shared`
    // (re-wrap cascade), not this one.
    let perms = permissions_for(&connection, &ids, claims.sub).await?;
    for id in &ids {
        let perm = perms.get(id).copied().unwrap_or(entity::permission::SharePermission::None);
        if matches!(perm, entity::permission::SharePermission::None) {
            return Err(Error::NotFound("file_not_found".to_string()));
        }
        if !perm.can_write() {
            return Err(Error::Forbidden("forbidden_read_only".to_string()));
        }
    }

    Repository::new(&connection)
        .manage(claims.sub)
        .move_many(ids, file_id)
        .await?;
    connection.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
