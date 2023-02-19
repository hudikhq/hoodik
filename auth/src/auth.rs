use crate::data::{authenticated::Authenticated, create_user::CreateUser};
use chrono::{Duration, Utc};
use context::Context;
use entity::{
    sessions, users, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter,
};
use error::{AppResult, Error};

pub struct Auth<'ctx> {
    pub context: &'ctx Context,
}

impl<'ctx> Auth<'ctx> {
    pub fn new(context: &'ctx Context) -> Auth<'ctx> {
        Auth { context }
    }

    /// Create a new user
    pub async fn register(&self, data: CreateUser) -> AppResult<users::Model> {
        let active_model = data.into_active_model()?;

        active_model
            .insert(&self.context.db)
            .await
            .map_err(Error::from)
    }

    /// Get a user by id
    pub async fn get_by_id(&self, id: i32) -> AppResult<users::Model> {
        users::Entity::find_by_id(id)
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{}", id)))
    }

    /// Get a user by email
    pub async fn get_by_email(&self, email: &str) -> AppResult<users::Model> {
        users::Entity::find()
            .filter(users::Column::Email.contains(email))
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{}", email)))
    }

    /// Get user and session by token and csrf
    pub async fn get_by_token_and_csrf(&self, token: &str, csrf: &str) -> AppResult<Authenticated> {
        let result = sessions::Entity::find()
            .filter(sessions::Column::Token.eq(token))
            .filter(sessions::Column::Csrf.eq(csrf))
            .inner_join(users::Entity)
            .select_also(users::Entity)
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::Unauthorized("session_not_found".to_string()))?;

        // inner_join makes sure the second parameter options is
        // always Some so we can unwrap it safely
        let (session, user) = (result.0, result.1.unwrap());

        Ok(Authenticated { user, session })
    }

    /// Get user and session by token
    pub async fn get_by_token(&self, token: &str) -> AppResult<Authenticated> {
        let result = sessions::Entity::find()
            .filter(sessions::Column::Token.eq(token))
            .inner_join(users::Entity)
            .select_also(users::Entity)
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::Unauthorized("session_not_found".to_string()))?;

        // inner_join makes sure the second parameter options is
        // always Some so we can unwrap it safely
        let (session, user) = (result.0, result.1.unwrap());

        Ok(Authenticated { user, session })
    }

    /// Generate a new session for a user
    pub async fn generate_session(
        &self,
        user: &users::Model,
        remember: bool,
    ) -> AppResult<sessions::Model> {
        let expires_at = match remember {
            true => Utc::now() + Duration::days(365),
            false => Utc::now() + Duration::minutes(10),
        };

        let active_model = sessions::ActiveModel {
            id: ActiveValue::NotSet,
            user_id: ActiveValue::Set(user.id),
            token: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
            csrf: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
            created_at: ActiveValue::Set(Utc::now().naive_utc()),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::Set(expires_at.naive_utc()),
        };

        active_model
            .insert(&self.context.db)
            .await
            .map_err(Error::from)
    }

    /// Refresh session, if it's not expired. Refreshing a session will extend the expiration date by 10 minutes.
    pub async fn refresh_session(&self, session: &sessions::Model) -> AppResult<sessions::Model> {
        if session.expires_at < Utc::now().naive_utc() {
            return Err(Error::Unauthorized("session_expired".to_string()));
        }

        let expires_at = session.expires_at + Duration::minutes(10);

        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(session.id),
            user_id: ActiveValue::Set(session.user_id),
            token: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
            csrf: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
            created_at: ActiveValue::Set(session.created_at),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::Set(expires_at),
        };

        active_model
            .update(&self.context.db)
            .await
            .map_err(Error::from)
    }
}
