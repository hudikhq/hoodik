use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};

use crate::{
    data::share_event::{ShareEventPage, ShareEventQuery},
    repository::queries,
    routes::gate,
};

const DEFAULT_LIMIT: u64 = 100;
const MAX_LIMIT: u64 = 1000;

/// `GET /api/shares/events` — audit-log view for the caller. Returns
/// every row they authored, every row that targets them, and every row
/// on a file they own. Filterable by `file_id` and `action`; paginated
/// `limit`/`offset` with the caps applied here.
#[route("/api/shares/events", method = "GET")]
pub(crate) async fn events(
    context: web::Data<Context>,
    authenticated: Authenticated,
    query: web::Query<ShareEventQuery>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;

    let query = query.into_inner();
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = query.offset.unwrap_or(0);

    let file_id = match query.file_id.as_deref() {
        Some(value) => Some(
            Uuid::parse_str(value)
                .map_err(|_| Error::BadRequest("file_id_invalid".to_string()))?,
        ),
        None => None,
    };

    let (events, users, total) = queries::events_for_user(
        &context.db,
        authenticated.user.id,
        file_id,
        query.action.clone(),
        limit,
        offset,
    )
    .await?;

    Ok(HttpResponse::Ok().json(ShareEventPage {
        events,
        users,
        total,
        limit,
        offset,
    }))
}
