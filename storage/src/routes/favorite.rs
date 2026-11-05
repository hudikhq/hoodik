use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::{TransactionTrait, Uuid};
use error::AppResult;

use crate::{
    data::favorite::{FavoriteMany, SetFavorite},
    permission,
    repository::Repository,
};

#[route("/api/storage/{file_id}/favorite", method = "PUT")]
pub(crate) async fn set_favorite(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<SetFavorite>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let connection = context.db.begin().await?;
    let repository = Repository::new(&connection);
    let file_id: Uuid = util::actix::path_var(&req, "file_id")?;

    permission::require_read(&connection, file_id, claims.sub).await?;

    let file = repository
        .manage(claims.sub)
        .set_favorite(file_id, data.into_inner().into_value()?)
        .await?;

    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}

#[route("/api/storage/favorite-many", method = "POST")]
pub(crate) async fn set_favorite_many(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<FavoriteMany>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let (ids, favorite) = data.into_inner().into_value()?;
    let connection = context.db.begin().await?;

    Repository::new(&connection)
        .manage(claims.sub)
        .set_favorite_many(ids, favorite)
        .await?;

    connection.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
