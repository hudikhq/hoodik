use entity::Uuid;
use serde::{Deserialize, Serialize};

/// Client's registration request (`ClientRegistration::start` output).
#[derive(Clone, Deserialize)]
pub struct OpaqueRegisterStart {
    pub registration_request: String,
}

/// Unauthenticated OPAQUE registration start for a brand-new (v2) signup. The
/// email is the OPAQUE credential identifier, so it must match the one the
/// account is created with and the one login later uses.
#[derive(Clone, Deserialize)]
pub struct SignupRegisterStart {
    pub email: String,
    pub registration_request: String,
}

#[derive(Clone, Serialize)]
pub struct OpaqueRegisterStartResponse {
    pub registration_response: String,
}

/// A password change for an OPAQUE account: the client's registration upload
/// (`ClientRegistration::finish` output) for the new password, plus the
/// private-key envelope re-sealed under the KEK derived from that new
/// password's `export_key`. Both are committed together — writing the password
/// file without the matching envelope would strand the account's keys on the
/// next login.
#[derive(Clone, Deserialize)]
pub struct OpaqueRegisterFinish {
    pub registration_upload: String,
    pub encrypted_private_key: String,
}

/// Client's login request (`ClientLogin::start` output) plus the account it is
/// for.
#[derive(Clone, Deserialize)]
pub struct OpaqueLoginStart {
    pub email: String,
    pub credential_request: String,
}

/// Login-start answer. `password` tells a client the account has not migrated
/// to OPAQUE, so it should fall back to the legacy `/api/auth/login`. The same
/// answer is returned for unknown emails so start does not reveal which
/// accounts exist.
#[derive(Clone, Serialize)]
#[serde(tag = "method", rename_all = "lowercase")]
pub enum OpaqueLoginStartResponse {
    Password,
    Opaque {
        login_id: Uuid,
        credential_response: String,
    },
}

/// Client's login finalization (`ClientLogin::finish` output), the login id
/// from start, and the optional TOTP token.
#[derive(Clone, Deserialize)]
pub struct OpaqueLoginFinish {
    pub login_id: Uuid,
    pub credential_finalization: String,
    pub token: Option<String>,
}

/// One re-wrapped file key: the file the caller holds, with its key now
/// wrapped under the new X25519 key instead of the old RSA key.
#[derive(Clone, Deserialize)]
pub struct RewrappedKey {
    pub file_id: Uuid,
    pub encrypted_key: String,
}

/// The `{file_id, encrypted_key}` list the client fetches at the start of
/// migration to re-wrap every key it holds.
#[derive(Clone, Serialize)]
pub struct MigrationKey {
    pub file_id: Uuid,
    pub encrypted_key: String,
}

/// Everything the client submits to finish its one-shot migration onto
/// Curve25519 + OPAQUE. The server re-derives the transition-certificate
/// canonical from its own record and verifies both signatures before
/// committing any of it.
#[derive(Clone, Deserialize)]
pub struct MigrationComplete {
    pub new_identity_pubkey: String,
    pub new_wrapping_pubkey: String,
    pub new_fingerprint: String,
    pub transition_old_signature: String,
    pub transition_new_signature: String,
    pub transition_issued_at: i64,
    pub opaque_registration_upload: String,
    pub encrypted_private_key: String,
    pub rewrapped_keys: Vec<RewrappedKey>,
}
