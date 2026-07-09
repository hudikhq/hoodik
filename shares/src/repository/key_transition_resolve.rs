//! Resolve a signature that a possibly-migrated account produced.
//!
//! An account that migrates RSA→Curve25519 (or, in future, rotates
//! curve→curve) keeps a `key_transitions` row endorsing the new key with the
//! old. A signature the account made *before* rotating verifies under the old
//! key, not the current one — and if the signed payload embedded the account's
//! own fingerprint (as the folder roster does), that fingerprint rotated too.
//! This module surfaces the superseded key + fingerprint so a verifier can fall
//! back through the single recorded transition. A key is superseded at most once
//! (`old_fingerprint` is unique), so resolution is one lookup, never a loop.

use std::str::FromStr;

use cryptfns::identity::KeyType;
use entity::{key_transitions, users, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use error::{AppResult, Error};

/// The key a migrated account signed with before rotating, rebuilt for
/// verification.
pub(crate) struct SupersededKey {
    pub key_type: KeyType,
    pub pubkey_pem: String,
    /// Hex fingerprint the account had before rotating — what a payload signed
    /// pre-migration committed to for this account.
    pub old_fingerprint: String,
}

/// The transition that produced `signer`'s current key, if any.
pub(crate) async fn resolve_superseded_key<C: ConnectionTrait>(
    db: &C,
    signer: &users::Model,
) -> AppResult<Option<SupersededKey>> {
    let transition = key_transitions::Entity::find()
        .filter(key_transitions::Column::UserId.eq(signer.id))
        .filter(key_transitions::Column::NewFingerprint.eq(signer.fingerprint.clone()))
        .one(db)
        .await?;

    let Some(transition) = transition else {
        return Ok(None);
    };
    let key_type = KeyType::from_str(&transition.old_key_type)?;
    let pubkey_pem = key_type
        .pem_from_member_der(&transition.old_key_spki)
        .map_err(|e| Error::CryptoError(Box::new(e)))?;
    Ok(Some(SupersededKey {
        key_type,
        pubkey_pem,
        old_fingerprint: transition.old_fingerprint,
    }))
}
