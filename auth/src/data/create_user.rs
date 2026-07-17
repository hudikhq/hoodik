//! # User registration data
use ::error::AppResult;
use chrono::Utc;
use entity::{users::ActiveModel, ActiveValue, Uuid};
use serde::{Deserialize, Serialize};
use util::validation::validate_otp;
use validr::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub email: Option<String>,
    pub password: Option<String>,
    pub secret: Option<String>,
    pub token: Option<String>,
    pub pubkey: Option<String>,
    pub fingerprint: Option<String>,
    pub key_type: Option<String>,
    pub wrapping_pubkey: Option<String>,
    pub encrypted_private_key: Option<String>,
    /// OPAQUE registration upload the server finishes into the stored password
    /// file. A brand-new account authenticates only via OPAQUE, so this is
    /// required and a plaintext-`password` field is refused.
    pub opaque_registration_upload: Option<String>,
    pub invitation_id: Option<Uuid>,
}

impl Validation for CreateUser {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(email),
            rule_email!(email),
            rule_required!(pubkey),
            rule_required!(fingerprint),
            // New accounts are Curve25519 + OPAQUE only. Legacy RSA accounts
            // still exist and still migrate on login, but they can no longer be
            // created; the registration surface is the one place we hold that
            // line so a stray RSA payload never becomes a fresh RSA account.
            Rule::new("key_type", |obj: &Self, error| {
                if obj.key_type.as_deref() != Some("curve25519") {
                    error.add("must_be_curve25519");
                }
            }),
            // OPAQUE is the only authenticator; a plaintext password is a client
            // bug or a downgrade attempt.
            Rule::new("password", |obj: &Self, error| {
                if obj.password.is_some() {
                    error.add("password_not_allowed");
                }
            }),
            rule_required!(wrapping_pubkey),
            rule_required!(encrypted_private_key),
            rule_required!(opaque_registration_upload),
            Rule::new("secret", |obj: &Self, error| {
                if let Some(v) = &obj.secret {
                    if !validate_otp(v, obj.token.as_ref()) {
                        error.add("invalid_otp_token");
                    }
                }
            }),
            Rule::new("pubkey", |obj: &Self, error| {
                if let Some(v) = &obj.pubkey {
                    if cryptfns::ed25519::public::from_str(v).is_err() {
                        error.add("invalid_pubkey_not_pkcs8_pem");
                    }
                }
            }),
            Rule::new("fingerprint", |obj: &Self, error| {
                let (Some(pubkey), Some(fingerprint)) = (&obj.pubkey, &obj.fingerprint) else {
                    return;
                };

                match cryptfns::spki::fingerprint(pubkey) {
                    Ok(fp) if fingerprint == &fp => {}
                    Ok(_) => error.add("invalid_pubkey_fingerprint"),
                    Err(_) => error.add("invalid_pubkey_not_pkcs8_pem"),
                }
            }),
            Rule::new("wrapping_pubkey", |obj: &Self, error| {
                if let Some(v) = &obj.wrapping_pubkey {
                    if cryptfns::ecdh::public::from_str(v).is_err() {
                        error.add("invalid_wrapping_pubkey");
                    }
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![
            modifier_trim!(email),
            modifier_lowercase!(email),
            Modifier::new("secret", |obj: &mut Self| {
                if let Some(secret) = &obj.secret {
                    if secret.is_empty() {
                        obj.secret = None;
                    }
                }
            }),
            Modifier::new("token", |obj: &mut Self| {
                if let Some(token) = &obj.token {
                    if token.is_empty() {
                        obj.token = None;
                    }
                }
            }),
            Modifier::new("secret", |obj: &mut Self| {
                if obj.secret.is_some() && obj.token.is_none() {
                    obj.secret = None;
                }

                if obj.secret.is_none() && obj.token.is_some() {
                    obj.token = None;
                }
            }),
        ]
    }
}

impl CreateUser {
    pub fn into_active_model(self) -> AppResult<ActiveModel> {
        let data = self.validate()?;

        Ok(ActiveModel {
            id: ActiveValue::Set(entity::Uuid::new_v4()),
            role: ActiveValue::NotSet,
            quota: ActiveValue::NotSet,
            email: ActiveValue::Set(data.email.unwrap()),
            password: ActiveValue::Set(None),
            secret: ActiveValue::Set(data.secret),
            pubkey: ActiveValue::Set(data.pubkey.unwrap()),
            fingerprint: ActiveValue::Set(data.fingerprint.unwrap()),
            key_type: ActiveValue::Set("curve25519".to_string()),
            wrapping_pubkey: ActiveValue::Set(data.wrapping_pubkey),
            // Born migrated. The OPAQUE password file is attached by the
            // register contract once the upload is finished.
            security_version: ActiveValue::Set(1),
            opaque_password_file: ActiveValue::NotSet,
            encrypted_private_key: ActiveValue::Set(data.encrypted_private_key),
            email_verified_at: ActiveValue::Set(None),
            created_at: ActiveValue::Set(Utc::now().timestamp()),
            updated_at: ActiveValue::Set(Utc::now().timestamp()),
            share_notifications_enabled: ActiveValue::Set(true),
        })
    }
}
