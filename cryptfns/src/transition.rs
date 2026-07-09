//! Key-transition certificate: the signed endorsement that lets a user re-key
//! (RSA → Curve25519) without breaking their identity. The old key vouches for
//! the new keys; the new identity key proves possession. Verifiers re-encode
//! the canonical from authoritative state and check both signatures against
//! that — the wire's own bytes are never trusted.

use crate::asn1::{encode_key_transition_v1, KeyTransitionV1, KEY_TRANSITION_V1_PREFIX};
use crate::error::{CryptoResult, Error};
use crate::identity::KeyType;

/// The fields a verifier reconstructs from its own records, plus the two
/// signatures from the wire. `old_*` come from the pre-migration user row;
/// `new_*` are the keys the client is migrating to.
pub struct Certificate<'a> {
    pub user_id: [u8; 16],
    pub old_key_type: KeyType,
    pub old_key_pem: &'a str,
    pub old_fingerprint: &'a str,
    pub new_identity_key_pem: &'a str,
    pub new_wrapping_key_pem: &'a str,
    pub new_fingerprint: &'a str,
    pub issued_at: i64,
}

impl Certificate<'_> {
    fn signing_input(&self) -> CryptoResult<Vec<u8>> {
        let payload = KeyTransitionV1 {
            user_id: self.user_id,
            old_key_spki: self.old_key_type.member_pubkey_der(self.old_key_pem)?,
            old_fingerprint: hex32(self.old_fingerprint)?,
            new_identity_key_spki: KeyType::Curve25519.member_pubkey_der(self.new_identity_key_pem)?,
            new_wrapping_key_spki: spki_der(self.new_wrapping_key_pem)?,
            new_fingerprint: hex32(self.new_fingerprint)?,
            issued_at: self.issued_at,
        };
        let der = encode_key_transition_v1(&payload)?;

        let mut input = Vec::with_capacity(KEY_TRANSITION_V1_PREFIX.len() + der.len());
        input.extend_from_slice(KEY_TRANSITION_V1_PREFIX);
        input.extend_from_slice(&der);
        Ok(input)
    }

    /// Sign the certificate with both keys. The client holds the old private
    /// key (still decryptable at migration time) and the freshly generated new
    /// identity key.
    pub fn sign(&self, old_private_key: &str, new_identity_private_key: &str) -> CryptoResult<Signatures> {
        let input = self.signing_input()?;

        let old_signature = match self.old_key_type {
            KeyType::Rsa => crate::rsa::private::sign_bytes(&input, old_private_key)?,
            KeyType::Curve25519 => crate::ed25519::private::sign_bytes(&input, old_private_key)?,
        };
        let new_signature = crate::ed25519::private::sign_bytes(&input, new_identity_private_key)?;

        Ok(Signatures { old_signature, new_signature })
    }

    /// Verify both signatures against the canonical re-encoded here. The old
    /// signature must verify under the old key, the new under the new identity
    /// key — a valid certificate proves the same person controls both.
    pub fn verify(&self, signatures: &Signatures) -> CryptoResult<()> {
        let input = self.signing_input()?;

        self.old_key_type
            .verify_bytes(&input, &signatures.old_signature, self.old_key_pem)
            .map_err(|_| Error::KeyEncoding("key_transition_old_signature_invalid".into()))?;
        KeyType::Curve25519
            .verify_bytes(&input, &signatures.new_signature, self.new_identity_key_pem)
            .map_err(|_| Error::KeyEncoding("key_transition_new_signature_invalid".into()))?;

        Ok(())
    }
}

/// The base64 signatures carried on the wire alongside the new keys.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct Signatures {
    pub old_signature: String,
    pub new_signature: String,
}

/// Binding-friendly wrapper over [`Certificate::sign`]: all fields owned, no
/// lifetimes, for the WASM and FFI edges. `user_id` is the 16 UUID bytes and
/// `old_key_type` is `"rsa"` or `"curve25519"`.
#[allow(clippy::too_many_arguments)]
pub fn sign_certificate(
    user_id: &[u8],
    old_key_type: &str,
    old_key_pem: &str,
    old_fingerprint: &str,
    new_identity_key_pem: &str,
    new_wrapping_key_pem: &str,
    new_fingerprint: &str,
    issued_at: i64,
    old_private_key: &str,
    new_identity_private_key: &str,
) -> CryptoResult<Signatures> {
    let user_id: [u8; 16] = user_id
        .try_into()
        .map_err(|_| Error::InvalidLength("user_id must be 16 bytes"))?;
    let old_key_type = <KeyType as std::str::FromStr>::from_str(old_key_type)?;

    Certificate {
        user_id,
        old_key_type,
        old_key_pem,
        old_fingerprint,
        new_identity_key_pem,
        new_wrapping_key_pem,
        new_fingerprint,
        issued_at,
    }
    .sign(old_private_key, new_identity_private_key)
}

fn hex32(fingerprint: &str) -> CryptoResult<[u8; 32]> {
    let bytes = crate::hex::decode(fingerprint)?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| Error::InvalidLength("fingerprint must be 32 bytes"))
}

fn spki_der(public_key_pem: &str) -> CryptoResult<Vec<u8>> {
    let (label, document) =
        pkcs8::Document::from_pem(public_key_pem).map_err(|e| Error::KeyEncoding(e.to_string()))?;
    if label != "PUBLIC KEY" {
        return Err(Error::KeyEncoding(format!("expected PUBLIC KEY pem, got {label}")));
    }
    Ok(document.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Keys {
        rsa_private: String,
        rsa_public: String,
        rsa_fingerprint: String,
        ed_private: String,
        ed_public: String,
        ed_fingerprint: String,
        x_public: String,
    }

    fn keys() -> Keys {
        let rsa = crate::rsa::private::generate().unwrap();
        let rsa_private = crate::rsa::private::to_string(&rsa).unwrap();
        let rsa_public_key = crate::rsa::public::from_private(&rsa).unwrap();
        let rsa_public = crate::rsa::public::to_string(&rsa_public_key).unwrap();
        let rsa_fingerprint = crate::rsa::fingerprint(rsa_public_key).unwrap();

        let ed_private = crate::ed25519::private::generate().unwrap();
        let ed_public = crate::ed25519::public::from_private(&ed_private).unwrap();
        let ed_fingerprint = crate::spki::fingerprint(&ed_public).unwrap();

        let x_private = crate::ecdh::private::generate().unwrap();
        let x_public = crate::ecdh::public::from_private(&x_private).unwrap();

        Keys {
            rsa_private,
            rsa_public,
            rsa_fingerprint,
            ed_private,
            ed_public,
            ed_fingerprint,
            x_public,
        }
    }

    fn cert<'a>(k: &'a Keys) -> Certificate<'a> {
        Certificate {
            user_id: [9u8; 16],
            old_key_type: KeyType::Rsa,
            old_key_pem: &k.rsa_public,
            old_fingerprint: &k.rsa_fingerprint,
            new_identity_key_pem: &k.ed_public,
            new_wrapping_key_pem: &k.x_public,
            new_fingerprint: &k.ed_fingerprint,
            issued_at: 1_783_000_000,
        }
    }

    #[test]
    fn sign_and_verify_round_trip() {
        let k = keys();
        let c = cert(&k);
        let signatures = c.sign(&k.rsa_private, &k.ed_private).unwrap();
        c.verify(&signatures).unwrap();
    }

    #[test]
    fn wrong_old_key_rejected() {
        let k = keys();
        let other = keys();
        let c = cert(&k);
        // Signed by a different old key than the certificate names.
        let signatures = c.sign(&other.rsa_private, &k.ed_private).unwrap();
        assert!(c.verify(&signatures).is_err());
    }

    #[test]
    fn new_key_must_prove_possession() {
        let k = keys();
        let other = keys();
        let c = cert(&k);
        // New signature from a key that isn't the named new identity key.
        let signatures = c.sign(&k.rsa_private, &other.ed_private).unwrap();
        assert!(c.verify(&signatures).is_err());
    }

    #[test]
    fn tampered_new_fingerprint_breaks_verification() {
        let k = keys();
        let c = cert(&k);
        let signatures = c.sign(&k.rsa_private, &k.ed_private).unwrap();

        let mut tampered = cert(&k);
        tampered.new_fingerprint = &k.rsa_fingerprint;
        assert!(tampered.verify(&signatures).is_err());
    }

    #[test]
    fn curve25519_to_curve25519_rotation() {
        let old_private = crate::ed25519::private::generate().unwrap();
        let old_public = crate::ed25519::public::from_private(&old_private).unwrap();
        let old_fingerprint = crate::spki::fingerprint(&old_public).unwrap();
        let k = keys();

        let c = Certificate {
            user_id: [1u8; 16],
            old_key_type: KeyType::Curve25519,
            old_key_pem: &old_public,
            old_fingerprint: &old_fingerprint,
            new_identity_key_pem: &k.ed_public,
            new_wrapping_key_pem: &k.x_public,
            new_fingerprint: &k.ed_fingerprint,
            issued_at: 1_783_000_000,
        };
        let signatures = c.sign(&old_private, &k.ed_private).unwrap();
        c.verify(&signatures).unwrap();
    }
}
