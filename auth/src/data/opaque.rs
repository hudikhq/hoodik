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
///
/// A session cookie alone must not authorize this: it is bearer state a stolen
/// cookie can replay, and the change also overwrites the private-key envelope.
/// `signature` is the account's signature over
/// `hoodik-pake-register-v1\0` + `registration_upload` + `\0` + `issued_at`,
/// proving possession of the identity key (which never lives in the session);
/// `issued_at` bounds the replay window and `token` carries TOTP when 2FA is on.
#[derive(Clone, Deserialize)]
pub struct OpaqueRegisterFinish {
    pub registration_upload: String,
    pub encrypted_private_key: String,
    pub signature: String,
    pub issued_at: i64,
    pub token: Option<String>,
}

/// Client's login request (`ClientLogin::start` output) plus the account it is
/// for.
#[derive(Clone, Deserialize)]
pub struct OpaqueLoginStart {
    pub email: String,
    pub credential_request: String,
}

/// The KSF parameters a client must feed its OPAQUE finish so `export_key`
/// comes out identical to registration. Returned by `login/start` for every
/// answer — a migrated account's stored values, or the current defaults for a
/// legacy or unknown account so the two stay indistinguishable.
#[derive(Clone, Serialize)]
pub struct KsfParamsResponse {
    pub algorithm: String,
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
    pub protocol_version: i32,
}

/// Login-start answer. `password` tells a client the account has not migrated
/// to OPAQUE, so it should fall back to the legacy `/api/auth/login`. The same
/// answer is returned for unknown emails so start does not reveal which
/// accounts exist.
#[derive(Clone, Serialize)]
#[serde(tag = "method", rename_all = "lowercase")]
pub enum OpaqueLoginStartResponse {
    Password {
        ksf: KsfParamsResponse,
    },
    Opaque {
        login_id: Uuid,
        credential_response: String,
        ksf: KsfParamsResponse,
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

/// A page of re-wrapped keys the client stages before completing migration.
/// Both arrays are optional; a page may carry only files, only links, or a mix.
/// The whole set is applied atomically by `migration/complete`, so batching here
/// only bounds the request body — it never partially commits the migration.
#[derive(Clone, Deserialize)]
pub struct RewrapBatch {
    #[serde(default)]
    pub keys: Vec<RewrappedKey>,
    #[serde(default)]
    pub link_keys: Vec<RewrappedLinkKey>,
}

/// Cursor into the migration key set. `offset` walks the caller's file keys
/// first, then their link keys; `limit` bounds the page so the server never
/// materializes a whole account's worth of hybrid-wrapped keys at once.
#[derive(Clone, Deserialize)]
pub struct MigrationKeysQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

/// The `{file_id, encrypted_key}` list the client fetches at the start of
/// migration to re-wrap every key it holds.
#[derive(Clone, Serialize)]
pub struct MigrationKey {
    pub file_id: Uuid,
    pub encrypted_key: String,
}

/// One public link the caller owns, its symmetric key still wrapped under the
/// old RSA identity. Re-wrapped under the new X25519 key during migration,
/// exactly like a file key. `file_id` is the canonical the owner re-signs so
/// the client need not fetch it separately.
#[derive(Clone, Serialize)]
pub struct MigrationLinkKey {
    pub link_id: Uuid,
    pub encrypted_link_key: String,
    pub file_id: Uuid,
}

/// One page of the keys the caller must re-wrap at migration: the file keys they
/// hold and the public link keys they own. Link keys are wrapped under the
/// owner's key too, so a migration that re-wrapped only file keys would lock the
/// owner out of every link they created before migrating. `next_offset` is the
/// cursor for the next page, or `None` once the whole set has been returned.
#[derive(Clone, Serialize)]
pub struct MigrationKeys {
    pub keys: Vec<MigrationKey>,
    pub link_keys: Vec<MigrationLinkKey>,
    pub next_offset: Option<i64>,
}

/// One re-wrapped link key: a link the caller owns, its key now wrapped under
/// the new X25519 key instead of the old RSA key, plus the owner's re-signature
/// over the link's `file_id` under the new identity key. Re-signing converges
/// the row to the current key; leaving the old RSA signature would make every
/// pre-migration link report an invalid owner signature once the account is
/// Ed25519.
#[derive(Clone, Deserialize)]
pub struct RewrappedLinkKey {
    pub link_id: Uuid,
    pub encrypted_link_key: String,
    pub signature: String,
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
    /// The new identity key's signature over the key-rotation audit canonical
    /// (user id, old fingerprint, new fingerprint, `transition_issued_at`). The
    /// server re-encodes that canonical from its own state and verifies it
    /// before appending the hash-chained `key_rotation` audit event.
    pub audit_event_signature: String,
}
