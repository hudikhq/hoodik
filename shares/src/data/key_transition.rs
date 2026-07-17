use std::str::FromStr;

use cryptfns::identity::KeyType;
use entity::key_transitions;
use serde::{Deserialize, Serialize};

/// A signer's single key rotation, attached to any response that carries a
/// signature the signer may have produced before rotating. Clients that verify
/// stored signatures locally (E2EE) use it to fall back to the old key when the
/// current one fails — after verifying the certificate itself, which needs
/// every field the transition canonical covers. Absent means the signer never
/// rotated — verify against the current key only. Carries only public
/// certificate material; no private or password state is ever included.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyTransitionRef {
    /// The superseded public key, PEM-armored to match how `pubkey` is served.
    pub old_key_pem: String,
    pub old_key_type: String,
    pub old_fingerprint: String,
    pub new_identity_key_pem: String,
    pub new_wrapping_key_pem: String,
    pub new_fingerprint: String,
    pub old_signature: String,
    pub new_signature: String,
    pub issued_at: i64,
}

impl KeyTransitionRef {
    /// Rebuild the client-facing record from a stored transition row. The
    /// stored `old_key_spki` is the key's member-DER; re-armor it to PEM so the
    /// client parses it exactly like a `pubkey`.
    pub(crate) fn from_row(row: &key_transitions::Model) -> Option<Self> {
        let old_key_type = KeyType::from_str(&row.old_key_type).ok()?;
        let old_key_pem = old_key_type.pem_from_member_der(&row.old_key_spki).ok()?;
        Some(Self {
            old_key_pem,
            old_key_type: row.old_key_type.clone(),
            old_fingerprint: row.old_fingerprint.clone(),
            new_identity_key_pem: row.new_identity_key_pem.clone(),
            new_wrapping_key_pem: row.new_wrapping_key_pem.clone(),
            new_fingerprint: row.new_fingerprint.clone(),
            old_signature: cryptfns::base64::encode(&row.old_signature),
            new_signature: cryptfns::base64::encode(&row.new_signature),
            issued_at: row.issued_at,
        })
    }
}
