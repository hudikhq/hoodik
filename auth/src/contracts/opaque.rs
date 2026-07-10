use chrono::{Duration, Utc};
use cryptfns::identity::KeyType;
use entity::{
    opaque_config, opaque_ksf, opaque_login_sessions, users, ActiveValue, ColumnTrait, EntityTrait,
    Expr, OnConflict, QueryFilter, Uuid,
};
use error::{AppResult, Error};
use std::str::FromStr;

use crate::data::{
    authenticated::Authenticated,
    opaque::{
        KsfParamsResponse, OpaqueLoginStartResponse, OpaqueRegisterFinish,
        OpaqueRegisterStartResponse,
    },
};

/// The OPAQUE protocol version new registrations and migrations record. Bumped
/// only on a breaking protocol change, which forces lazy re-registration.
pub(crate) const CURRENT_OPAQUE_PROTOCOL_VERSION: i32 = 1;

/// The KSF parameters a client without a stored record should use: today's
/// compile-time defaults. Returned for legacy and unknown accounts so the two
/// are indistinguishable at `login/start`.
fn default_ksf_response() -> KsfParamsResponse {
    let params = cryptfns::opaque::current_ksf_params();
    KsfParamsResponse {
        algorithm: params.algorithm,
        m_cost: params.m_cost,
        t_cost: params.t_cost,
        p_cost: params.p_cost,
        protocol_version: CURRENT_OPAQUE_PROTOCOL_VERSION,
    }
}

use super::{repository::Repository, sessions::Sessions};

/// How long a login-start server state stays valid before the client must
/// restart the login.
const LOGIN_STATE_TTL_SECONDS: i64 = 60;

/// Domain-separation tag the password-change signature commits to. The client
/// signs `PREFIX\0registration_upload\0issued_at`; the server re-encodes that
/// canonical from the request's own fields and verifies it against the
/// account's stored identity key — a wire-supplied canonical is never trusted.
const PAKE_REGISTER_CANONICAL_PREFIX: &str = "hoodik-pake-register-v1";

/// A password-change signature older than this (server clock) is rejected, so a
/// captured request cannot be replayed. Matches the migration replay window.
const PASSWORD_CHANGE_REPLAY_WINDOW_SECONDS: i64 = 300;

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

    /// Finish an authenticated OPAQUE re-registration — the server half of a
    /// password change for a v2 account. A valid session is not sufficient
    /// authorization: it is bearer state a stolen cookie can replay, and the
    /// change overwrites the private-key envelope — after migration the only
    /// copy of the account's identity key. Authorization therefore requires an
    /// affirmative ownership proof: a signature by that identity key (which
    /// never lives in the session), plus TOTP and a replay window — the same
    /// standard the legacy password change enforces. Only a v2 account may
    /// reach here; running it on a legacy one would leave both a bcrypt hash and
    /// an OPAQUE password file live, the hybrid state migration forbids. The new
    /// password file and re-sealed envelope commit in one UPDATE so the account
    /// can never hold a password file whose `export_key` no longer opens its
    /// envelope.
    async fn opaque_register_finish(
        &self,
        user_id: Uuid,
        data: &OpaqueRegisterFinish,
    ) -> AppResult<()> {
        let user = self.get_by_id(user_id).await?;

        if user.security_version != 1 {
            return Err(Error::BadRequest(
                "password_change_requires_migration".to_string(),
            ));
        }

        let now = Utc::now().timestamp();
        if (now - data.issued_at).abs() > PASSWORD_CHANGE_REPLAY_WINDOW_SECONDS {
            return Err(Error::BadRequest("signature_timestamp_skew".to_string()));
        }

        if !user.verify_tfa(data.token.clone()) {
            return Err(Error::Unauthorized("invalid_otp_token".to_string()));
        }

        let canonical = format!(
            "{PAKE_REGISTER_CANONICAL_PREFIX}\0{}\0{}",
            data.registration_upload, data.issued_at
        );
        KeyType::from_str(&user.key_type)?
            .verify(&canonical, &data.signature, &user.pubkey)
            .map_err(|_| Error::Unauthorized("ownership_proof_required".to_string()))?;

        // The envelope is opaque to the server, but an empty one would replace
        // the user's only copy of their private key with nothing — a one-way
        // brick. Same guard as migration/complete.
        if data.encrypted_private_key.trim().is_empty() {
            return Err(Error::BadRequest("encrypted_private_key_required".to_string()));
        }

        let password_file =
            cryptfns::opaque::server_registration_finish(&data.registration_upload)
                .map_err(|_| Error::BadRequest("opaque_registration_upload_invalid".to_string()))?;

        users::Entity::update_many()
            .col_expr(users::Column::OpaquePasswordFile, Expr::value(password_file))
            .col_expr(
                users::Column::EncryptedPrivateKey,
                Expr::value(data.encrypted_private_key.clone()),
            )
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
            Err(e) if e.is_not_found() => {
                return Ok(OpaqueLoginStartResponse::Password { ksf: default_ksf_response() })
            }
            Err(e) => return Err(e),
        };

        let Some(password_file) = user.opaque_password_file.clone() else {
            return Ok(OpaqueLoginStartResponse::Password { ksf: default_ksf_response() });
        };

        let ksf = match opaque_ksf::Entity::find_by_id(user.id)
            .one(self.connection())
            .await?
        {
            Some(row) => KsfParamsResponse {
                algorithm: row.ksf_algorithm,
                m_cost: row.ksf_m_cost as u32,
                t_cost: row.ksf_t_cost as u32,
                p_cost: row.ksf_p_cost as u32,
                protocol_version: row.opaque_protocol_version,
            },
            None => default_ksf_response(),
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
            ksf,
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
