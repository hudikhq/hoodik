use chrono::{Duration, Utc};
use entity::{sessions, users, ActiveModelTrait, ActiveValue, EntityTrait, Uuid};
use error::{AppResult, Error};

use crate::data::authenticated::Authenticated;

use super::{ctx::Ctx, repository::Repository};

/// Session management contract
#[async_trait::async_trait]
pub(crate) trait Sessions
where
    Self: Ctx + Repository,
{
    /// Generate a new session for a user
    async fn generate_session(
        &self,
        user: &users::Model,
        user_agent: &str,
        ip: &str,
    ) -> AppResult<sessions::Model> {
        let expires_at = Utc::now()
            + Duration::seconds(self.ctx().config.auth.short_term_session_duration_seconds);

        let id = entity::Uuid::new_v4();

        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(id),
            user_id: ActiveValue::Set(user.id),
            device_id: ActiveValue::Set(Uuid::new_v4()),
            ip: ActiveValue::Set(ip.to_string()),
            user_agent: ActiveValue::Set(user_agent.to_string()),
            refresh: ActiveValue::Set(Some(Uuid::new_v4())),
            created_at: ActiveValue::Set(Utc::now().timestamp()),
            updated_at: ActiveValue::Set(Utc::now().timestamp()),
            expires_at: ActiveValue::Set(expires_at.timestamp()),
        };

        sessions::Entity::insert(active_model)
            .exec_without_returning(self.connection())
            .await?;

        let result = sessions::Entity::find_by_id(id)
            .one(self.connection())
            .await?;

        result.ok_or(Error::NotFound("session_not_found".to_string()))
    }

    /// Refresh session, if it's not expired. Refreshing a session will extend the expiration date by 10 minutes.
    async fn refresh_session(&self, session: &sessions::Model) -> AppResult<Authenticated> {
        let expires_at = Utc::now().naive_utc()
            + Duration::seconds(self.ctx().config.auth.short_term_session_duration_seconds);

        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(session.id),
            user_id: ActiveValue::Set(session.user_id),
            device_id: ActiveValue::Set(session.device_id),
            ip: ActiveValue::Set(session.ip.clone()),
            user_agent: ActiveValue::Set(session.user_agent.clone()),
            refresh: ActiveValue::Set(Some(Uuid::new_v4())),
            created_at: ActiveValue::Set(session.created_at),
            updated_at: ActiveValue::Set(Utc::now().timestamp()),
            expires_at: ActiveValue::Set(expires_at.timestamp()),
        };

        active_model.update(self.connection()).await?;

        self.get_by_device_id(session.device_id).await
    }

    /// Perform the logout action
    async fn destroy_session(&self, session: &sessions::Model) -> AppResult<Authenticated> {
        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(session.id),
            user_id: ActiveValue::Set(session.user_id),
            device_id: ActiveValue::Set(session.device_id),
            ip: ActiveValue::Set(session.ip.clone()),
            user_agent: ActiveValue::Set(session.user_agent.clone()),
            refresh: ActiveValue::Set(None),
            created_at: ActiveValue::Set(session.created_at),
            updated_at: ActiveValue::Set(Utc::now().timestamp()),
            expires_at: ActiveValue::Set(Utc::now().timestamp()),
        };

        let session = active_model.update(self.connection()).await?;

        self.get_by_session_id(session.id).await
    }
}