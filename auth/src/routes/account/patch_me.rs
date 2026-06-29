use actix_web::{route, web, HttpResponse};
use context::Context;
use entity::{users, ActiveValue};
use error::AppResult;
use serde::{Deserialize, Serialize};

use crate::{auth::Auth, contracts::repository::Repository, data::authenticated::Authenticated};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PatchMe {
    pub share_notifications_enabled: Option<bool>,
}

/// Apply a partial update to the caller's own user row. The body is a
/// thin patch object — fields left unset stay untouched. Currently the
/// only supported field is the share-notifications opt-out flag.
#[route("/api/users/me", method = "PATCH")]
pub(crate) async fn patch_me(
    context: web::Data<Context>,
    authenticated: Authenticated,
    body: web::Json<PatchMe>,
) -> AppResult<HttpResponse> {
    let payload = body.into_inner();
    let mut active = users::ActiveModel {
        ..Default::default()
    };
    if let Some(enabled) = payload.share_notifications_enabled {
        active.share_notifications_enabled = ActiveValue::Set(enabled);
    }
    let auth = Auth::new(&context);
    let updated = auth.update_user(authenticated.user.id, active).await?;
    Ok(HttpResponse::Ok().json(updated))
}
