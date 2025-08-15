use async_trait::async_trait;
use chrono::Utc;
use context::DatabaseConnection;
use entity::{
    invitations, sessions, users, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait,
    PaginatorTrait, QueryFilter, Uuid,
};
use error::{AppResult, Error};

use crate::data::authenticated::Authenticated;

use super::ctx::Ctx;

/// Expose the generic database interactions for authentication
/// and authorization to the implementor
#[async_trait]
pub(crate) trait Repository
where
    Self: Ctx,
{
    fn connection(&self) -> &DatabaseConnection {
        &self.ctx().db
    }

    /// Count the number of users
    async fn count_users(&self) -> AppResult<u64> {
        users::Entity::find()
            .count(self.connection())
            .await
            .map_err(Error::from)
    }

    /// Get a user by id
    async fn get_by_id(&self, id: Uuid) -> AppResult<users::Model> {
        users::Entity::find_by_id(id)
            .one(self.connection())
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{id}")))
    }

    /// Get a user by email
    async fn get_by_email(&self, email: &str) -> AppResult<users::Model> {
        users::Entity::find()
            .filter(users::Column::Email.contains(email))
            .one(self.connection())
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{email}")))
    }

    /// Load the invitation when registering the user
    async fn get_invitation(&self, id: Uuid) -> AppResult<invitations::Model> {
        let invitation = invitations::Entity::find_by_id(id)
            .one(self.connection())
            .await?
            .ok_or_else(|| Error::NotFound("invitation_not_found".to_string()))?;

        if invitation.expires_at < Utc::now().timestamp() {
            return Err(Error::as_not_found("invitation_not_found"));
        }

        Ok(invitation)
    }

    /// Get a user by fingerprint
    async fn get_by_fingerprint(&self, fingerprint: &str) -> AppResult<users::Model> {
        users::Entity::find()
            .filter(users::Column::Fingerprint.contains(fingerprint))
            .one(self.connection())
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{fingerprint}")))
    }

    /// Get user and session by session id, session does not have to be valid
    async fn get_by_session_id(&self, id: Uuid) -> AppResult<Authenticated> {
        let result = sessions::Entity::find()
            .filter(sessions::Column::Id.eq(id))
            .inner_join(users::Entity)
            .select_also(users::Entity)
            .one(self.connection())
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::Unauthorized("session_not_found".to_string()))?;

        // inner_join makes sure the second parameter options is
        // always Some so we can unwrap it safely
        let (session, user) = (result.0, result.1.unwrap());

        Ok(Authenticated { user, session })
    }

    /// Get user and session by refresh token, session must be valid
    async fn get_by_refresh(&self, refresh: Uuid) -> AppResult<Authenticated> {
        let result = sessions::Entity::find()
            .filter(sessions::Column::Refresh.eq(refresh))
            .inner_join(users::Entity)
            .select_also(users::Entity)
            .one(self.connection())
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::Unauthorized("session_not_found".to_string()))?;

        // inner_join makes sure the second parameter options is
        // always Some so we can unwrap it safely
        let (session, user) = (result.0, result.1.unwrap());

        Ok(Authenticated { user, session })
    }

    /// Get user and session by device id, session must be valid
    async fn get_by_device_id(&self, device_id: Uuid) -> AppResult<Authenticated> {
        let result = sessions::Entity::find()
            .filter(sessions::Column::DeviceId.eq(device_id))
            .filter(sessions::Column::Refresh.is_not_null())
            .inner_join(users::Entity)
            .select_also(users::Entity)
            .one(self.connection())
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::Unauthorized("session_not_found".to_string()))?;

        // inner_join makes sure the second parameter options is
        // always Some so we can unwrap it safely
        let (session, user) = (result.0, result.1.unwrap());

        Ok(Authenticated { user, session })
    }

    /// Create a new user
    async fn create_user(&self, active_model: users::ActiveModel) -> AppResult<users::Model> {
        let id: Uuid = entity::active_value_to_uuid(active_model.id.clone())
            .ok_or(Error::as_wrong_id("user"))?;

        users::Entity::insert(active_model)
            .exec_without_returning(self.connection())
            .await?;

        self.get_by_id(id).await
    }

    /// Update the user data
    async fn update_user(
        &self,
        id: Uuid,
        mut active_model: users::ActiveModel,
    ) -> AppResult<users::Model> {
        active_model.id = ActiveValue::Set(id);
        active_model.updated_at = ActiveValue::Set(Utc::now().timestamp());

        active_model.update(self.connection()).await?;

        self.get_by_id(id).await
    }
}
