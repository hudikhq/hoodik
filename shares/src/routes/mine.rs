use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::{
    contracts::shares::IncomingQueryExt,
    data::incoming::{IncomingShareQuery, IncomingSharePage},
    repository::queries,
    routes::gate,
};

/// `GET /api/shares/mine` — paged list of incoming shares for the caller.
#[route("/api/shares/mine", method = "GET")]
pub(crate) async fn mine(
    context: web::Data<Context>,
    authenticated: Authenticated,
    query: web::Query<IncomingShareQuery>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let query = query.into_inner();
    let (items, total) = queries::incoming_for_recipient(
        &context.db,
        authenticated.user.id,
        None,
        query.resolved_limit(),
        query.resolved_offset(),
    )
    .await?;

    let page = IncomingSharePage {
        items,
        total,
        limit: query.resolved_limit(),
        offset: query.resolved_offset(),
    };
    Ok(HttpResponse::Ok().json(page))
}

/// `GET /api/shares/mine/by/{user_id}` — same as `/mine`, narrowed to
/// grants whose `shared_by_user_id` matches `user_id`.
#[route("/api/shares/mine/by/{user_id}", method = "GET")]
pub(crate) async fn mine_by(
    req: HttpRequest,
    context: web::Data<Context>,
    authenticated: Authenticated,
    query: web::Query<IncomingShareQuery>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let sender_id: Uuid = util::actix::path_var(&req, "user_id")?;
    let query = query.into_inner();
    let (items, total) = queries::incoming_for_recipient(
        &context.db,
        authenticated.user.id,
        Some(sender_id),
        query.resolved_limit(),
        query.resolved_offset(),
    )
    .await?;

    let page = IncomingSharePage {
        items,
        total,
        limit: query.resolved_limit(),
        offset: query.resolved_offset(),
    };
    Ok(HttpResponse::Ok().json(page))
}
