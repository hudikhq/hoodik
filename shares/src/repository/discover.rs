//! `GET /api/users/discover` — return one user's pubkey +
//! fingerprint by email lookup. Rate-limited per caller, with hit and
//! miss sharing the same bucket so an attacker can't enumerate via
//! differential 200/404 timing.

use entity::{users, ColumnTrait, EntityTrait, QueryFilter};
use error::{AppResult, Error};

use crate::{
    data::discover::DiscoveredUser,
    repository::{discover_rate_limit, Repository},
};

impl Repository<'_> {
    pub(crate) async fn discover_user(
        &self,
        caller: &users::Model,
        query_email: &str,
        now: i64,
    ) -> AppResult<DiscoveredUser> {
        // Bump the counter on every call.
        if discover_rate_limit::over_limit(caller.id, now) {
            return Err(Error::TooManyRequests("rate_limited".to_string()));
        }

        let trimmed = query_email.trim();
        if trimmed.is_empty() {
            return Err(Error::BadRequest("email_required".to_string()));
        }
        // Self-lookup rejected before the DB hit so a caller cannot
        // discover their own pubkey through this surface.
        if trimmed.eq_ignore_ascii_case(&caller.email) {
            return Err(Error::BadRequest("cannot_discover_self".to_string()));
        }

        let lower = trimmed.to_ascii_lowercase();
        let row = users::Entity::find()
            .filter(users::Column::Email.eq(lower))
            .one(&self.context.db)
            .await?;

        let user = row.ok_or_else(|| Error::NotFound("user_not_found".to_string()))?;
        if user.id == caller.id {
            // Defence in depth — the email comparison above already
            // catches self-lookup; this catches the case where the
            // caller renamed their email and the DB row diverged.
            return Err(Error::BadRequest("cannot_discover_self".to_string()));
        }

        Ok(DiscoveredUser {
            user_id: user.id,
            email: user.email,
            pubkey: user.pubkey,
            key_type: user.key_type,
            wrapping_pubkey: user.wrapping_pubkey,
            fingerprint: user.fingerprint,
        })
    }
}
