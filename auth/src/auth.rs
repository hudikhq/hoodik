use crate::data::{authenticated::Authenticated, create_user::CreateUser};
use actix_web::cookie::{time::OffsetDateTime, Cookie, SameSite};
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

        Ok(Authenticated {
            user,
            session: Some(session),
        })
    }

    /// Get Authenticated by signature and pubkey
    pub async fn get_by_signature_and_pubkey(
        &self,
        signature: &str,
        pubkey: &str,
    ) -> AppResult<Authenticated> {
        let user = users::Entity::find()
            .filter(users::Column::Pubkey.eq(pubkey))
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::Unauthorized("invalid_signature".to_string()))?;

        let message = (Utc::now().timestamp() / 60).to_string();

        cryptfns::verify_signature(&user.pubkey, &message, signature)?;

        Ok(Authenticated {
            user,
            session: None,
        })
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

        Ok(Authenticated {
            user,
            session: Some(session),
        })
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
