use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::TransactionTrait;
use error::AppResult;

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
    Repository::new(&connection)
        .manage(claims.sub)
        .move_many(ids, file_id)
        .await?;
    connection.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
