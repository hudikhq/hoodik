//! # User registration data
use std::str::FromStr;

use ::error::AppResult;
use chrono::Utc;
use cryptfns::identity::KeyType;
use entity::{users::ActiveModel, ActiveValue, Uuid};
use serde::{Deserialize, Serialize};
use util::{
    password::hash,
    validation::{validate_otp, validate_password},
};
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
    /// OPAQUE registration upload for a v2 (curve25519) signup. Its presence is
    /// what makes registration create a migrated account directly: the server
    /// finishes it into the stored password file instead of hashing a password.
    pub opaque_registration_upload: Option<String>,
    pub invitation_id: Option<Uuid>,
}

impl CreateUser {
    /// Clients that predate the Curve25519 migration don't send `key_type`;
    /// their accounts are RSA.
    fn key_type(&self) -> Option<KeyType> {
        KeyType::from_str(self.key_type.as_deref().unwrap_or("rsa")).ok()
    }
}

impl Validation for CreateUser {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(email),
            rule_email!(email),
            rule_required!(pubkey),
            rule_required!(fingerprint),
            // Legacy (RSA) accounts authenticate with a bcrypt password; v2
            // (curve25519) accounts authenticate with OPAQUE and must not carry
            // one. The presence of an `opaque_registration_upload` is what marks
            // a v2 signup.
            Rule::new("password", |obj: &Self, error| match obj.key_type() {
                Some(KeyType::Curve25519) => {
                    if obj.password.is_some() {
                        error.add("password_not_allowed_for_curve25519");
                    }
                }
                _ => match &obj.password {
                    None => error.add("required"),
                    Some(v) if !validate_password(v) => error.add("weak_password"),
                    Some(_) => {}
                },
            }),
            Rule::new("opaque_registration_upload", |obj: &Self, error| {
                if matches!(obj.key_type(), Some(KeyType::Curve25519))
                    && obj.opaque_registration_upload.is_none()
                {
                    error.add("required");
                }
            }),
            Rule::new("encrypted_private_key", |obj: &Self, error| {
                if matches!(obj.key_type(), Some(KeyType::Curve25519))
                    && obj.encrypted_private_key.is_none()
                {
                    error.add("required");
                }
            }),
            Rule::new("secret", |obj: &Self, error| {
                if let Some(v) = &obj.secret {
                    if !validate_otp(v, obj.token.as_ref()) {
                        error.add("invalid_otp_token");
                    }
                }
            }),
            Rule::new("key_type", |obj: &Self, error| {
                if obj.key_type().is_none() {
                    error.add("invalid_key_type");
                }
            }),
            Rule::new("pubkey", |obj: &Self, error| {
                let (Some(v), Some(key_type)) = (&obj.pubkey, obj.key_type()) else {
                    return;
                };

                let valid = match key_type {
                    KeyType::Rsa => cryptfns::rsa::public::from_str(v).is_ok(),
                    KeyType::Curve25519 => cryptfns::ed25519::public::from_str(v).is_ok(),
                };

                if !valid {
                    error.add("invalid_pubkey_not_pkcs8_pem");
                }
            }),
            Rule::new("fingerprint", |obj: &Self, error| {
                let (Some(pubkey), Some(key_type)) = (&obj.pubkey, obj.key_type()) else {
                    return;
                };

                match key_type.fingerprint(pubkey) {
                    Ok(fp) => {
                        if let Some(fingerprint) = &obj.fingerprint {
                            if fingerprint != &fp {
                                error.add("invalid_pubkey_fingerprint");
                            }
                        }
                    }
                    Err(_) => error.add("invalid_pubkey_not_pkcs8_pem"),
                }
            }),
            Rule::new("wrapping_pubkey", |obj: &Self, error| {
                let Some(key_type) = obj.key_type() else {
                    return;
                };

                match key_type {
                    // The RSA key wraps and signs; a second key is a client bug.
                    KeyType::Rsa => {
                        if obj.wrapping_pubkey.is_some() {
                            error.add("wrapping_pubkey_not_allowed_for_rsa");
                        }
                    }
                    KeyType::Curve25519 => match &obj.wrapping_pubkey {
                        Some(v) => {
                            if cryptfns::ecdh::public::from_str(v).is_err() {
                                error.add("invalid_wrapping_pubkey");
                            }
                        }
                        None => error.add("required"),
                    },
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
        let key_type = data.key_type().unwrap();

        Ok(ActiveModel {
            id: ActiveValue::Set(entity::Uuid::new_v4()),
            role: ActiveValue::NotSet,
            quota: ActiveValue::NotSet,
            email: ActiveValue::Set(data.email.unwrap()),
            password: ActiveValue::Set(data.password.map(hash)),
            secret: ActiveValue::Set(data.secret),
            pubkey: ActiveValue::Set(data.pubkey.unwrap()),
            fingerprint: ActiveValue::Set(data.fingerprint.unwrap()),
            key_type: ActiveValue::Set(key_type.as_str().to_string()),
            wrapping_pubkey: ActiveValue::Set(data.wrapping_pubkey),
            // A v2 signup is born migrated (security_version 1); the OPAQUE
            // password file is attached by the register contract once the
            // upload is finished. Legacy accounts stay at the column default (0).
            security_version: match key_type {
                KeyType::Curve25519 => ActiveValue::Set(1),
                KeyType::Rsa => ActiveValue::NotSet,
            },
            opaque_password_file: ActiveValue::NotSet,
            encrypted_private_key: ActiveValue::Set(data.encrypted_private_key),
            email_verified_at: ActiveValue::Set(None),
            created_at: ActiveValue::Set(Utc::now().timestamp()),
            updated_at: ActiveValue::Set(Utc::now().timestamp()),
            share_notifications_enabled: ActiveValue::Set(true),
        })
    }
}
