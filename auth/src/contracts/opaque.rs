use chrono::{Duration, Utc};
use entity::{
    opaque_config, opaque_login_sessions, users, ActiveValue, ColumnTrait, EntityTrait, Expr,
    OnConflict, QueryFilter, Uuid,
};
use error::{AppResult, Error};

use crate::data::{
    authenticated::Authenticated,
    opaque::{OpaqueLoginStartResponse, OpaqueRegisterStartResponse},
};

use super::{repository::Repository, sessions::Sessions};

/// How long a login-start server state stays valid before the client must
/// restart the login.
const LOGIN_STATE_TTL_SECONDS: i64 = 60;

/// Server side of the OPAQUE handshake. Registration is driven from inside an
/// authenticated session (migration, password change, or new-account setup);
/// login start/finish are public. The password never reaches the server in
/// any of them.
#[async_trait::async_trait]
pub(crate) trait Opaque
where
    Self: Repository + Sessions,
{
    /// Read the singleton server OPRF seed, generating and persisting it on
    /// first use. It is never rotated — every registration is bound to it.
    async fn opaque_server_setup(&self) -> AppResult<String> {
        if let Some(row) = opaque_config::Entity::find_by_id(opaque_config::Model::SINGLETON_ID)
            .one(self.connection())
            .await?
        {
            return Ok(row.server_setup);
        }

        let server_setup = cryptfns::opaque::server_setup_new();
        opaque_config::Entity::insert(opaque_config::ActiveModel {
            id: ActiveValue::Set(opaque_config::Model::SINGLETON_ID),
            server_setup: ActiveValue::Set(server_setup),
        })
        .on_conflict(OnConflict::column(opaque_config::Column::Id).do_nothing().to_owned())
        .exec_without_returning(self.connection())
        .await?;

        // Re-read: a racing request may have inserted first, and both must
        // end up with the same persisted seed.
        opaque_config::Entity::find_by_id(opaque_config::Model::SINGLETON_ID)
            .one(self.connection())
            .await?
            .map(|row| row.server_setup)
            .ok_or_else(|| Error::InternalError("opaque_server_setup_missing".to_string()))
    }

    async fn opaque_register_start(
        &self,
        user_id: Uuid,
        registration_request: &str,
    ) -> AppResult<OpaqueRegisterStartResponse> {
        let user = self.get_by_id(user_id).await?;
        let setup = self.opaque_server_setup().await?;

        let registration_response = cryptfns::opaque::server_registration_start(
            &setup,
            registration_request,
            user.email.as_bytes(),
        )
        .map_err(|_| Error::BadRequest("opaque_registration_request_invalid".to_string()))?;

        Ok(OpaqueRegisterStartResponse { registration_response })
    }

    /// Unauthenticated OPAQUE registration start for a brand-new signup, keyed
    /// by the email (the OPAQUE credential identifier — it must match the email
    /// the account is created with and later logs in with). Rejects emails that
    /// already exist so it never emits registration state for a taken account.
    async fn opaque_signup_register_start(
        &self,
        email: &str,
        registration_request: &str,
    ) -> AppResult<OpaqueRegisterStartResponse> {
        let email = email.trim().to_lowercase();
        if self.get_by_email(&email).await.is_ok() {
            return Err(Error::as_validation("email", "invalid_email"));
        }

        let setup = self.opaque_server_setup().await?;
        let registration_response = cryptfns::opaque::server_registration_start(
            &setup,
            registration_request,
            email.as_bytes(),
        )
        .map_err(|_| Error::BadRequest("opaque_registration_request_invalid".to_string()))?;

        Ok(OpaqueRegisterStartResponse { registration_response })
    }

    async fn opaque_register_finish(
        &self,
        user_id: Uuid,
        registration_upload: &str,
    ) -> AppResult<()> {
        let password_file = cryptfns::opaque::server_registration_finish(registration_upload)
            .map_err(|_| Error::BadRequest("opaque_registration_upload_invalid".to_string()))?;

        users::Entity::update_many()
            .col_expr(users::Column::OpaquePasswordFile, Expr::value(password_file))
            .filter(users::Column::Id.eq(user_id))
            .exec(self.connection())
            .await?;

        Ok(())
    }

    /// Begin an OPAQUE login. Returns [`OpaqueLoginStartResponse::Password`]
    /// for accounts without an OPAQUE record (legacy, or nonexistent — the two
    /// are indistinguishable here so start does not leak account existence).
    async fn opaque_login_start(
        &self,
        email: &str,
        credential_request: &str,
    ) -> AppResult<OpaqueLoginStartResponse> {
        // During the migration window the response necessarily discloses the
        // method (`password` for legacy/unknown accounts so the client can fall
        // back to bcrypt, `opaque` for migrated ones), so there is no point
        // masking the timing difference between the two branches — the method
        // field already reveals it. Full anti-enumeration (always a KE2, real
        // or a dummy record) becomes possible once legacy accounts are retired
        // and the password fallback is removed.
        let user = match self.get_by_email(email).await {
            Ok(user) => user,
            Err(e) if e.is_not_found() => return Ok(OpaqueLoginStartResponse::Password),
            Err(e) => return Err(e),
        };

        let Some(password_file) = user.opaque_password_file.clone() else {
            return Ok(OpaqueLoginStartResponse::Password);
        };

        let setup = self.opaque_server_setup().await?;
        let started = cryptfns::opaque::server_login_start(
            &setup,
            &password_file,
            credential_request,
            user.email.as_bytes(),
        )
        .map_err(|_| Error::Unauthorized("invalid_credentials".to_string()))?;

        // Abandoned logins (no finish) would otherwise accumulate forever.
        // Purge anything already expired before recording this one.
        opaque_login_sessions::Entity::delete_many()
            .filter(opaque_login_sessions::Column::ExpiresAt.lt(Utc::now().timestamp()))
            .exec(self.connection())
            .await?;

        let login_id = Uuid::new_v4();
        opaque_login_sessions::Entity::insert(opaque_login_sessions::ActiveModel {
            id: ActiveValue::Set(login_id),
            user_id: ActiveValue::Set(user.id),
            server_login_state: ActiveValue::Set(started.state),
            expires_at: ActiveValue::Set(
                (Utc::now() + Duration::seconds(LOGIN_STATE_TTL_SECONDS)).timestamp(),
            ),
        })
        .exec_without_returning(self.connection())
        .await?;

        Ok(OpaqueLoginStartResponse::Opaque {
            login_id,
            credential_response: started.response,
        })
    }

    /// Finish an OPAQUE login: consume the server state, verify the client's
    /// proof, run TOTP and activation checks, and mint a session.
    async fn opaque_login_finish(
        &self,
        login_id: Uuid,
        credential_finalization: &str,
        token: Option<String>,
        user_agent: &str,
        ip: &str,
    ) -> AppResult<Authenticated> {
        let session = opaque_login_sessions::Entity::find_by_id(login_id)
            .one(self.connection())
            .await?
            .ok_or_else(|| Error::Unauthorized("invalid_credentials".to_string()))?;

        // Consume it regardless of outcome — a login state is single-use.
        opaque_login_sessions::Entity::delete_by_id(login_id)
            .exec(self.connection())
            .await?;

        if session.expires_at < Utc::now().timestamp() {
            return Err(Error::Unauthorized("login_expired".to_string()));
        }

        cryptfns::opaque::server_login_finish(&session.server_login_state, credential_finalization)
            .map_err(|_| Error::Unauthorized("invalid_credentials".to_string()))?;

        let mut user = self.get_by_id(session.user_id).await?;

        if !user.verify_tfa(token) {
            return Err(Error::Unauthorized("invalid_otp_token".to_string()));
        }

        if self.enforce_email_activation().await && user.email_verified_at.is_none() {
            return Err(Error::Unauthorized("inactive_account".to_string()));
        }

        if user.quota.is_none() {
            user.quota = self
                .ctx()
                .settings
                .inner()
                .await
                .users
                .quota_bytes()
                .map(|v| v as i64);
        }

        let session = self.generate(&user, user_agent, ip).await?;

        Ok(Authenticated { user, session })
    }
}
