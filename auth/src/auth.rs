use crate::{
    actions::UserActions,
    data::{authenticated::Authenticated, create_user::CreateUser},
};
use actix_web::cookie::{time::OffsetDateTime, Cookie, CookieBuilder, SameSite};
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
            .filter(sessions::Column::DeletedAt.is_null())
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::Unauthorized("session_not_found".to_string()))?;

        if session.expires_at < Utc::now().naive_utc() {
            return Err(Error::Unauthorized("session_expired".to_string()));
        }

        Ok(())
    }

    /// Get user and session by session id, session does not have to be valid
    pub async fn get_by_session_id(&self, id: Uuid) -> AppResult<Authenticated> {
        let result = sessions::Entity::find()
            .filter(sessions::Column::Id.eq(id))
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

    /// Get user and session by refresh token, session must be valid
    pub async fn get_by_refresh(&self, refresh: Uuid) -> AppResult<Authenticated> {
        let result = sessions::Entity::find()
            .filter(sessions::Column::Refresh.eq(refresh))
            .filter(sessions::Column::DeletedAt.is_null())
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

    /// Get user and session by device id, session must be valid
    pub async fn get_by_device_id(&self, device_id: Uuid) -> AppResult<Authenticated> {
        let result = sessions::Entity::find()
            .filter(sessions::Column::DeviceId.eq(device_id))
            .filter(sessions::Column::DeletedAt.is_null())
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
        user_agent: &str,
        ip: &str,
    ) -> AppResult<sessions::Model> {
        let expires_at =
            Utc::now() + Duration::seconds(self.context.config.short_term_session_duration_seconds);

        let id = entity::Uuid::new_v4();

        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(id),
            user_id: ActiveValue::Set(user.id),
            device_id: ActiveValue::Set(Uuid::new_v4()),
            ip: ActiveValue::Set(ip.to_string()),
            user_agent: ActiveValue::Set(user_agent.to_string()),
            refresh: ActiveValue::Set(Some(Uuid::new_v4())),
            created_at: ActiveValue::Set(Utc::now().naive_utc()),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::Set(expires_at.naive_utc()),
            deleted_at: ActiveValue::NotSet,
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
        let expires_at = Utc::now().naive_utc()
            + Duration::seconds(self.context.config.short_term_session_duration_seconds);

        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(session.id),
            user_id: ActiveValue::Set(session.user_id),
            device_id: ActiveValue::Set(session.device_id),
            ip: ActiveValue::Set(session.ip.clone()),
            user_agent: ActiveValue::Set(session.user_agent.clone()),
            refresh: ActiveValue::Set(Some(Uuid::new_v4())),
            created_at: ActiveValue::Set(session.created_at),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::Set(expires_at),
            deleted_at: ActiveValue::NotSet,
        };

        active_model.update(&self.context.db).await?;

        self.get_by_device_id(session.device_id).await
    }

    /// Perform the logout action
    pub async fn destroy_session(&self, session: &sessions::Model) -> AppResult<Authenticated> {
        let active_model = sessions::ActiveModel {
            id: ActiveValue::Set(session.id),
            user_id: ActiveValue::Set(session.user_id),
            device_id: ActiveValue::Set(session.device_id),
            ip: ActiveValue::Set(session.ip.clone()),
            user_agent: ActiveValue::Set(session.user_agent.clone()),
            refresh: ActiveValue::Set(None),
            created_at: ActiveValue::Set(session.created_at),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::Set(Utc::now().naive_utc()),
            deleted_at: ActiveValue::Set(Some(Utc::now().naive_utc())),
        };

        let session = active_model.update(&self.context.db).await?;

        self.get_by_session_id(session.id).await
    }

    /// Sets a cookie on the request
    pub async fn manage_cookies(
        &self,
        authenticated: &Authenticated,
        issuer: &str,
    ) -> AppResult<(Cookie<'static>, Cookie<'static>)> {
        let destroy =
            authenticated.session.deleted_at.is_some() || authenticated.session.refresh.is_none();

        let mut refresh = authenticated
            .session
            .refresh
            .map(|r| r.to_string())
            .unwrap_or_else(|| "destroyed".to_string());

        if destroy && &refresh != "destroyed" {
            refresh = "destroyed".to_string();
        }

        let jwt = match destroy {
            true => "destroyed".to_string(),
            false => crate::jwt::generate(authenticated, issuer, &self.context.config.jwt_secret)?,
        };

        let jwt = self.make_cookie(
            Cookie::build(self.context.config.get_session_cookie(), jwt).path("/"),
            destroy,
        )?;

        let refresh = self.make_cookie(
            Cookie::build(self.context.config.get_refresh_cookie(), refresh)
                .path(crate::REFRESH_PATH),
            destroy,
        )?;

        Ok((jwt, refresh))
    }

    /// Set configuration parameters for cookie security
    fn make_cookie(
        &self,
        cookie: CookieBuilder<'static>,
        destroy: bool,
    ) -> AppResult<Cookie<'static>> {
        let mut cookie = cookie
            .secure(self.context.config.cookie_secure)
            .http_only(self.context.config.cookie_http_only)
            .finish();

        cookie.set_domain(self.context.config.get_cookie_domain());

        if destroy {
            cookie.set_expires(OffsetDateTime::from_unix_timestamp(0).unwrap());
        } else {
            let timestamp =
                Utc::now() + Duration::days(self.context.config.long_term_session_duration_days);
            cookie.set_expires(OffsetDateTime::from_unix_timestamp(timestamp.timestamp()).unwrap());
        }

        match self.context.config.cookie_same_site.as_ref() {
            "Lax" => cookie.set_same_site(SameSite::Lax),
            "Strict" => cookie.set_same_site(SameSite::Strict),
            _ => cookie.set_same_site(SameSite::None),
        };

        Ok(cookie)
    }
}
