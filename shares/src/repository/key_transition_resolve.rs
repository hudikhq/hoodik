//! Verified resolution of a migrated account's key-transition chain.
//!
//! An account that migrates RSA→Curve25519 — and, in future, rotates
//! Curve25519→Curve25519 — records a `key_transitions` row endorsing the new key
//! with the old. A signature the account made *before* rotating verifies under a
//! superseded key, not the current one; and if the signed payload embedded the
//! account's own fingerprint (as the folder roster does), that fingerprint
//! rotated too. This resolves the account's current identity back through its
//! chain to the original key it started from, so a verifier can fall back to it.
//!
//! The whole transition set is read in one query and walked in memory — no
//! per-hop round trip. Every hop's certificate is re-encoded from the stored
//! components and BOTH its signatures verified against the keys that hop rotated
//! to (persisted on the row, so an intermediate hop is verifiable even after the
//! account has rotated past it). A walk that trusted structural fingerprint
//! linkage without checking each endorsement would let a hostile server assert
//! any prior identity for the account — so an unverifiable hop aborts the walk
//! rather than being skipped.

use std::collections::HashMap;
use std::str::FromStr;

use cryptfns::identity::KeyType;
use cryptfns::transition::{Certificate, Signatures};
use entity::{key_transitions, users, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use error::{AppResult, Error};

/// Ceiling on transition hops walked before the chain is rejected. `old_fingerprint`
/// is UNIQUE, so the migration path (one row per rotation, written once) cannot
/// build a cycle; this only guards a tampered or hand-built DB — e.g. a row whose
/// `new_fingerprint` points back into the chain — from spinning the walk forever.
/// Far above any realistic rotation count: one RSA→curve migration today, a
/// future ML-DSA rotation the second.
const MAX_CHAIN_HOPS: usize = 8;

/// The original key a migrated account signed with before it ever rotated,
/// rebuilt for verification.
pub(crate) struct SupersededKey {
    pub key_type: KeyType,
    pub pubkey_pem: String,
    /// Hex fingerprint the account had before rotating — what a payload signed
    /// pre-migration committed to for this account.
    pub old_fingerprint: String,
}

/// Resolve `signer`'s current identity back through its verified transition chain
/// to the original key. `None` when the account never rotated (verify against the
/// current key only); `Err` when a hop fails to verify or the chain exceeds
/// [`MAX_CHAIN_HOPS`].
pub(crate) async fn resolve_superseded_key<C: ConnectionTrait>(
    db: &C,
    signer: &users::Model,
) -> AppResult<Option<SupersededKey>> {
    let transitions = key_transitions::Entity::find()
        .filter(key_transitions::Column::UserId.eq(signer.id))
        .all(db)
        .await?;
    if transitions.is_empty() {
        return Ok(None);
    }

    // Index by the fingerprint each hop produced, so stepping from a key to the
    // one it superseded is a map lookup, not a query.
    let by_new: HashMap<&str, &key_transitions::Model> = transitions
        .iter()
        .map(|hop| (hop.new_fingerprint.as_str(), hop))
        .collect();

    // Walk backward from the current fingerprint: each step is the hop whose new
    // key is the one we currently stand on, taking us to its old key. Verify that
    // hop before stepping through it. The genesis hop — the last one reached,
    // whose old key no further hop supersedes — carries the original key.
    let mut cursor = signer.fingerprint.as_str();
    let mut genesis: Option<&key_transitions::Model> = None;
    let mut hops = 0usize;
    while let Some(hop) = by_new.get(cursor).copied() {
        hops += 1;
        if hops > MAX_CHAIN_HOPS {
            return Err(Error::InternalError(
                "key_transition_chain_too_long".to_string(),
            ));
        }
        verify_hop(signer.id, hop)?;
        genesis = Some(hop);
        cursor = hop.old_fingerprint.as_str();
    }

    genesis.map(superseded_key_from).transpose()
}

/// Rebuild a hop's superseded (old) public key and its type from the stored
/// member-DER, shared by verification and by the returned [`SupersededKey`].
fn old_key_of(hop: &key_transitions::Model) -> AppResult<(KeyType, String)> {
    let key_type = KeyType::from_str(&hop.old_key_type)?;
    let pem = key_type
        .pem_from_member_der(&hop.old_key_spki)
        .map_err(|e| Error::CryptoError(Box::new(e)))?;
    Ok((key_type, pem))
}

/// Re-encode this hop's transition certificate from the stored components and
/// verify both signatures: the old key's endorsement and the new identity key's
/// proof of possession. The keys the hop rotated to come from the row, so the
/// canonical is reconstructed identically whether or not the account has since
/// rotated further.
fn verify_hop(user_id: entity::Uuid, hop: &key_transitions::Model) -> AppResult<()> {
    let (old_key_type, old_key_pem) = old_key_of(hop)?;
    let certificate = Certificate {
        user_id: user_id.into_bytes(),
        old_key_type,
        old_key_pem: &old_key_pem,
        old_fingerprint: &hop.old_fingerprint,
        new_identity_key_pem: &hop.new_identity_key_pem,
        new_wrapping_key_pem: &hop.new_wrapping_key_pem,
        new_fingerprint: &hop.new_fingerprint,
        issued_at: hop.issued_at,
    };
    certificate
        .verify(&Signatures {
            old_signature: cryptfns::base64::encode(&hop.old_signature),
            new_signature: cryptfns::base64::encode(&hop.new_signature),
        })
        .map_err(|_| Error::InternalError("key_transition_hop_invalid".to_string()))
}

fn superseded_key_from(hop: &key_transitions::Model) -> AppResult<SupersededKey> {
    let (key_type, pubkey_pem) = old_key_of(hop)?;
    Ok(SupersededKey {
        key_type,
        pubkey_pem,
        old_fingerprint: hop.old_fingerprint.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use entity::{ActiveValue, Expr, Uuid};

    /// One key in a chain: RSA at genesis, Curve25519 afterwards. `wrapping_pem`
    /// is present only for curve keys (RSA has no separate wrapping key), and is
    /// the value a hop rotating *to* this key commits to.
    struct ChainKey {
        private: String,
        public: String,
        fingerprint: String,
        wrapping_pem: Option<String>,
        key_type: KeyType,
    }

    fn rsa_key() -> ChainKey {
        let private = cryptfns::rsa::private::generate().unwrap();
        let private_pem = cryptfns::rsa::private::to_string(&private).unwrap();
        let public = cryptfns::rsa::public::from_private(&private).unwrap();
        let public_pem = cryptfns::rsa::public::to_string(&public).unwrap();
        let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();
        ChainKey {
            private: private_pem,
            public: public_pem,
            fingerprint,
            wrapping_pem: None,
            key_type: KeyType::Rsa,
        }
    }

    fn curve_key() -> ChainKey {
        let ed_private = cryptfns::ed25519::private::generate().unwrap();
        let ed_public = cryptfns::ed25519::public::from_private(&ed_private).unwrap();
        let fingerprint = cryptfns::spki::fingerprint(&ed_public).unwrap();
        let x_private = cryptfns::ecdh::private::generate().unwrap();
        let x_public = cryptfns::ecdh::public::from_private(&x_private).unwrap();
        ChainKey {
            private: ed_private,
            public: ed_public,
            fingerprint,
            wrapping_pem: Some(x_public),
            key_type: KeyType::Curve25519,
        }
    }

    /// Insert a genuine transition row rotating `old` → `new`, exactly as
    /// `migration_complete` would: build the certificate, sign it with both keys,
    /// and persist the components plus the keys rotated to.
    async fn insert_hop<C: ConnectionTrait>(db: &C, user_id: Uuid, old: &ChainKey, new: &ChainKey) {
        let new_wrapping = new.wrapping_pem.as_deref().expect("a hop rotates to a curve key");
        let issued_at = chrono::Utc::now().timestamp();
        let cert = Certificate {
            user_id: user_id.into_bytes(),
            old_key_type: old.key_type,
            old_key_pem: &old.public,
            old_fingerprint: &old.fingerprint,
            new_identity_key_pem: &new.public,
            new_wrapping_key_pem: new_wrapping,
            new_fingerprint: &new.fingerprint,
            issued_at,
        };
        let signatures = cert.sign(&old.private, &new.private).unwrap();
        cert.verify(&signatures).unwrap();

        key_transitions::Entity::insert(key_transitions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            user_id: ActiveValue::Set(user_id),
            old_fingerprint: ActiveValue::Set(old.fingerprint.clone()),
            old_key_spki: ActiveValue::Set(old.key_type.member_pubkey_der(&old.public).unwrap()),
            old_key_type: ActiveValue::Set(old.key_type.as_str().to_string()),
            new_fingerprint: ActiveValue::Set(new.fingerprint.clone()),
            new_identity_key_pem: ActiveValue::Set(new.public.clone()),
            new_wrapping_key_pem: ActiveValue::Set(new_wrapping.to_string()),
            old_signature: ActiveValue::Set(cryptfns::base64::decode(&signatures.old_signature).unwrap()),
            new_signature: ActiveValue::Set(cryptfns::base64::decode(&signatures.new_signature).unwrap()),
            issued_at: ActiveValue::Set(issued_at),
            created_at: ActiveValue::Set(issued_at),
        })
        .exec_without_returning(db)
        .await
        .unwrap();
    }

    /// Insert a signer at `fingerprint` and return its model. The row exists so
    /// the `key_transitions.user_id` foreign key is satisfied; the walk itself
    /// reads only `id` and `fingerprint` from the returned model.
    async fn seed_signer<C: ConnectionTrait>(db: &C, fingerprint: &str) -> users::Model {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().timestamp();
        users::Entity::insert(users::ActiveModel {
            id: ActiveValue::Set(id),
            role: ActiveValue::NotSet,
            quota: ActiveValue::NotSet,
            email: ActiveValue::Set(format!("chain-{id}@example.com")),
            password: ActiveValue::NotSet,
            secret: ActiveValue::NotSet,
            pubkey: ActiveValue::Set(String::new()),
            fingerprint: ActiveValue::Set(fingerprint.to_string()),
            key_type: ActiveValue::Set("curve25519".to_string()),
            wrapping_pubkey: ActiveValue::NotSet,
            security_version: ActiveValue::Set(1),
            opaque_password_file: ActiveValue::NotSet,
            encrypted_private_key: ActiveValue::NotSet,
            email_verified_at: ActiveValue::Set(Some(now)),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            share_notifications_enabled: ActiveValue::Set(true),
        })
        .exec_without_returning(db)
        .await
        .unwrap();
        users::Entity::find_by_id(id).one(db).await.unwrap().unwrap()
    }

    /// Flip a byte of the stored `old`- or `new`-signature of the hop whose old
    /// fingerprint is `old_fingerprint`, standing in for a tampered/hostile row.
    async fn corrupt_hop_signature<C: ConnectionTrait>(
        db: &C,
        old_fingerprint: &str,
        which_new: bool,
    ) {
        let row = key_transitions::Entity::find()
            .filter(key_transitions::Column::OldFingerprint.eq(old_fingerprint))
            .one(db)
            .await
            .unwrap()
            .unwrap();
        let mut bytes = if which_new { row.new_signature } else { row.old_signature };
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        let column = if which_new {
            key_transitions::Column::NewSignature
        } else {
            key_transitions::Column::OldSignature
        };
        key_transitions::Entity::update_many()
            .col_expr(column, Expr::value(bytes))
            .filter(key_transitions::Column::OldFingerprint.eq(old_fingerprint))
            .exec(db)
            .await
            .unwrap();
    }

    #[actix_web::test]
    async fn no_transition_resolves_to_current_key() {
        let db = context::Context::mock_sqlite().await.db;
        let signer = seed_signer(&db, "deadbeef").await;
        let resolved = resolve_superseded_key(&db, &signer).await.unwrap();
        assert!(resolved.is_none(), "an account that never rotated has no superseded key");
    }

    #[actix_web::test]
    async fn two_hop_chain_resolves_to_original_key() {
        let db = context::Context::mock_sqlite().await.db;
        let f0 = rsa_key();
        let f1 = curve_key();
        let f2 = curve_key();
        let signer = seed_signer(&db, &f2.fingerprint).await;
        insert_hop(&db, signer.id, &f0, &f1).await;
        insert_hop(&db, signer.id, &f1, &f2).await;

        let resolved = resolve_superseded_key(&db, &signer)
            .await
            .unwrap()
            .expect("a rotated account resolves to its original key");
        // Both hops verified end to end and the walk returned the genesis (F0),
        // not the immediately-prior (F1) key.
        assert_eq!(resolved.key_type, KeyType::Rsa);
        assert_eq!(resolved.old_fingerprint, f0.fingerprint);
        assert_eq!(resolved.pubkey_pem, f0.public);
    }

    #[actix_web::test]
    async fn tampered_intermediate_hop_aborts_resolution() {
        // The case invisible under the old schema: the F0→F1 hop's new keys were
        // not stored, so it could not be verified once the account rotated to F2.
        let db = context::Context::mock_sqlite().await.db;
        let f0 = rsa_key();
        let f1 = curve_key();
        let f2 = curve_key();
        let signer = seed_signer(&db, &f2.fingerprint).await;
        insert_hop(&db, signer.id, &f0, &f1).await;
        insert_hop(&db, signer.id, &f1, &f2).await;
        corrupt_hop_signature(&db, &f0.fingerprint, true).await;

        let resolved = resolve_superseded_key(&db, &signer).await;
        assert!(resolved.is_err(), "a bad intermediate hop must abort, not resolve");
    }

    #[actix_web::test]
    async fn tampered_final_hop_aborts_resolution() {
        let db = context::Context::mock_sqlite().await.db;
        let f0 = rsa_key();
        let f1 = curve_key();
        let f2 = curve_key();
        let signer = seed_signer(&db, &f2.fingerprint).await;
        insert_hop(&db, signer.id, &f0, &f1).await;
        insert_hop(&db, signer.id, &f1, &f2).await;
        corrupt_hop_signature(&db, &f1.fingerprint, false).await;

        let resolved = resolve_superseded_key(&db, &signer).await;
        assert!(resolved.is_err(), "a bad final hop must abort, not resolve");
    }

    #[actix_web::test]
    async fn chain_longer_than_cap_is_rejected() {
        let db = context::Context::mock_sqlite().await.db;
        // One genesis + (MAX_CHAIN_HOPS + 1) rotations exceeds the cap. All curve
        // keys keep the keygen cheap; the walk is key-type agnostic.
        let mut keys = vec![curve_key()];
        for _ in 0..=MAX_CHAIN_HOPS {
            keys.push(curve_key());
        }
        let signer = seed_signer(&db, &keys.last().unwrap().fingerprint).await;
        for pair in keys.windows(2) {
            insert_hop(&db, signer.id, &pair[0], &pair[1]).await;
        }

        let resolved = resolve_superseded_key(&db, &signer).await;
        assert!(resolved.is_err(), "a chain past the hop cap is rejected, not walked forever");
    }

    #[actix_web::test]
    async fn single_hop_resolves_like_production() {
        // The only chain length that exists in production today: one RSA→curve
        // migration. A chain of length 1 must behave exactly as before.
        let db = context::Context::mock_sqlite().await.db;
        let f0 = rsa_key();
        let f1 = curve_key();
        let signer = seed_signer(&db, &f1.fingerprint).await;
        insert_hop(&db, signer.id, &f0, &f1).await;

        let resolved = resolve_superseded_key(&db, &signer)
            .await
            .unwrap()
            .expect("a single migration resolves to the pre-migration key");
        assert_eq!(resolved.old_fingerprint, f0.fingerprint);
        assert_eq!(resolved.pubkey_pem, f0.public);
    }
}
