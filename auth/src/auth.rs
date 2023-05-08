use crate::{
    actions::UserActions,
    data::{authenticated::Authenticated, create_user::CreateUser},
};
use actix_web::cookie::{time::OffsetDateTime, Cookie, SameSite};
use chrono::{Duration, Utc};
use context::Context;
use entity::{
    sessions, users, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter,
    TransactionTrait, Uuid,
};
use error::{AppResult, Error};

pub struct Auth<'ctx> {
    pub context: &'ctx Context,
}

impl<'ctx> Auth<'ctx> {
    pub fn generate_nonce_seconds() -> String {
        format!("{}", Utc::now().timestamp())
    }

    pub fn generate_nonce_minutes() -> String {
        format!("{}", Utc::now().timestamp() / 60)
    }

    pub fn generate_fingerprint_nonce(fingerprint: &str) -> String {
        format!("{}-{}", fingerprint, Self::generate_nonce_minutes())
    }

    pub fn generate_two_factor() -> String {
        util::generate::generate_secret()
    }
}

impl<'ctx> Auth<'ctx> {
    pub fn new(context: &'ctx Context) -> Auth<'ctx> {
        Auth { context }
    }

    /// Create a new user
    pub async fn register(&self, data: CreateUser) -> AppResult<users::Model> {
        let email = data.email.clone();
        let mut active_model = data.into_active_model()?;

        let id: Uuid = entity::active_value_to_uuid(active_model.id.clone())
            .ok_or(Error::as_wrong_id("user"))?;

        // We can unwrap here because it would fail validation before this
        if self.get_by_email(email.unwrap().as_str()).await.is_ok() {
            return Err(Error::as_validation("email", "invalid_email"));
        }

        if self.context.sender.is_none() {
            active_model.email_verified_at = ActiveValue::Set(Some(Utc::now().naive_utc()));
        }

        users::Entity::insert(active_model)
            .exec_without_returning(&self.context.db)
            .await?;

        let user = self.get_by_id(id).await?;

        crate::emails::activate::send(self.context, &user).await?;

        Ok(user)
    }

    /// Perform activation of the user
    pub async fn activate(&self, user_action_id: Uuid) -> AppResult<users::Model> {
        let tx = self.context.db.begin().await.unwrap();

        let user_action = UserActions::new(self.context).with_connection(&tx);

        let (action, user) = user_action.get_by_id(user_action_id).await?;

        if action.action != "activate-email" {
            return Err(Error::as_not_found("wrong_user_action"));
        }

        if user.email_verified_at.is_some() {
            return Err(Error::as_not_found("email_already_verified"));
        }

        let id = user.id;

        let mut active_model: users::ActiveModel = user.into();

        active_model.email_verified_at = ActiveValue::Set(Some(Utc::now().naive_utc()));

        active_model.update(&tx).await?;

        user_action.delete(user_action_id).await?;

        tx.commit().await?;

        self.get_by_id(id).await
    }

    /// Get a user by id
    pub async fn get_by_id(&self, id: Uuid) -> AppResult<users::Model> {
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

    /// Get a user by fingerprint
    pub async fn get_by_fingerprint(&self, fingerprint: &str) -> AppResult<users::Model> {
        users::Entity::find()
            .filter(users::Column::Fingerprint.contains(fingerprint))
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{}", fingerprint)))
    }

    /// Validate a session by its device id
    pub async fn validate(&self, id: Uuid) -> AppResult<()> {
        let session = sessions::Entity::find()
            .filter(sessions::Column::Id.eq(id))
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::Unauthorized("session_not_found".to_string()))?;

        if session.expires_at < Utc::now().naive_utc() {
            return Err(Error::Unauthorized("session_expired".to_string()));
        }

        Ok(())
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
    pub async fn get_by_device_id(&self, id: &str) -> AppResult<Authenticated> {
        let result = sessions::Entity::find()
            .filter(sessions::Column::DeviceId.eq(id))
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
            true => {
                Utc::now() + Duration::days(self.context.config.long_term_session_duration_days)
            }
            false => {
                Utc::now()
                    + Duration::minutes(self.context.config.short_term_session_duration_minutes)
            }
        };

        let id = entity::Uuid::new_v4();

        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(id),
            user_id: ActiveValue::Set(user.id),
            device_id: ActiveValue::Set(Uuid::new_v4().to_string()),
            token: ActiveValue::Set(Uuid::new_v4().to_string()),
            csrf: ActiveValue::Set(Uuid::new_v4().to_string()),
            created_at: ActiveValue::Set(Utc::now().naive_utc()),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::Set(expires_at.naive_utc()),
        };

        sessions::Entity::insert(active_model)
            .exec_without_returning(&self.context.db)
            .await?;

        let result = sessions::Entity::find_by_id(id)
            .one(&self.context.db)
            .await?;

        result.ok_or(Error::NotFound("session_not_found".to_string()))
    }

    /// Refresh session, if it's not expired. Refreshing a session will extend the expiration date by 10 minutes.
    pub async fn refresh_session(&self, session: &sessions::Model) -> AppResult<Authenticated> {
        let duration = session.expires_at - session.updated_at;
        let expires_at = Utc::now().naive_utc() + duration;

        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(session.id),
            user_id: ActiveValue::Set(session.user_id),
            device_id: ActiveValue::Set(session.device_id.clone()),
            token: ActiveValue::Set(Uuid::new_v4().to_string()),
            csrf: ActiveValue::Set(Uuid::new_v4().to_string()),
            created_at: ActiveValue::Set(session.created_at),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::Set(expires_at),
        };

        active_model.update(&self.context.db).await?;

        self.get_by_device_id(&session.device_id).await
    }

    /// Perform the logout action
    pub async fn destroy_session(&self, session: &sessions::Model) -> AppResult<Authenticated> {
        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(session.id),
            user_id: ActiveValue::Set(session.user_id),
            device_id: ActiveValue::Set(session.device_id.clone()),
            token: ActiveValue::Set(Uuid::new_v4().to_string()),
            csrf: ActiveValue::Set(Uuid::new_v4().to_string()),
            created_at: ActiveValue::Set(session.created_at),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::Set(Utc::now().naive_utc()),
        };

        active_model.update(&self.context.db).await?;

        self.get_by_device_id(&session.device_id).await
    }

    /// Sets a cookie on the request
    pub async fn manage_cookie(
        &self,
        session: &sessions::Model,
        destroy: bool,
    ) -> AppResult<Cookie<'static>> {
        let mut cookie = Cookie::build(
            self.context.config.cookie_name.clone(),
            session.token.clone(),
        )
        .path("/")
        .secure(self.context.config.cookie_secure)
        .http_only(self.context.config.cookie_http_only)
        .finish();

        if let Some(domain) = &self.context.config.cookie_domain {
            cookie.set_domain(domain.clone());
        }

        match self.context.config.cookie_same_site.as_ref() {
            "Lax" => cookie.set_same_site(SameSite::Lax),
            "Strict" => cookie.set_same_site(SameSite::Strict),
            _ => cookie.set_same_site(SameSite::None),
        };

        // If we are not destroying the cookie we will set
        // The proper expiration time on it, but if we are
        // We will set it to 1970-01-01
        if !destroy {
            let timestamp = session.expires_at.timestamp();
            cookie.set_expires(OffsetDateTime::from_unix_timestamp(timestamp).unwrap());
        } else {
            cookie.set_expires(OffsetDateTime::from_unix_timestamp(0).unwrap());
        }

        Ok(cookie)
    }
}
