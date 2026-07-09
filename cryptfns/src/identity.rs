use crate::error::{CryptoResult, Error};

/// A user's key algorithm, stored in `users.key_type`.
///
/// `Rsa` accounts have one RSA keypair doing identity, signing, and wrapping.
/// `Curve25519` accounts split the jobs: an Ed25519 identity/signing key (in
/// `users.pubkey`, fingerprinted via SPKI) and an X25519 wrapping key (in
/// `users.wrapping_pubkey`). Signature verification dispatches here so the
/// branch exists exactly once server-side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyType {
    Rsa,
    Curve25519,
}

impl std::str::FromStr for KeyType {
    type Err = Error;

    fn from_str(s: &str) -> CryptoResult<Self> {
        match s {
            "rsa" | "" => Ok(Self::Rsa),
            "curve25519" => Ok(Self::Curve25519),
            other => Err(Error::KeyEncoding(format!("unknown key type: {other}"))),
        }
    }
}

impl KeyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rsa => "rsa",
            Self::Curve25519 => "curve25519",
        }
    }

    /// Verify a base64 signature over raw bytes against a PEM public key.
    pub fn verify_bytes(&self, message: &[u8], signature: &str, key: &str) -> CryptoResult<()> {
        match self {
            Self::Rsa => crate::rsa::public::verify_bytes(message, signature, key),
            Self::Curve25519 => crate::ed25519::public::verify_bytes(message, signature, key),
        }
    }

    /// Verify a base64 signature over a UTF-8 string.
    pub fn verify(&self, message: &str, signature: &str, key: &str) -> CryptoResult<()> {
        self.verify_bytes(message.as_bytes(), signature, key)
    }

    /// Fingerprint of the identity public key, using the derivation this key
    /// type registered with: legacy modulus hash for RSA, SPKI hash otherwise.
    pub fn fingerprint(&self, public_key: &str) -> CryptoResult<String> {
        match self {
            Self::Rsa => crate::rsa::fingerprint(crate::rsa::public::from_str(public_key)?),
            Self::Curve25519 => crate::spki::fingerprint(public_key),
        }
    }

    /// The `MemberSigPayloadV1.pubkey_der` canonical for a recipient of this
    /// key type: the DER body of their stored public-key PEM — PKCS#1 for
    /// RSA (what every existing signature committed to), SPKI otherwise.
    pub fn member_pubkey_der(&self, public_key: &str) -> CryptoResult<Vec<u8>> {
        match self {
            Self::Rsa => crate::rsa::public::to_pkcs1_der(public_key),
            Self::Curve25519 => {
                let (label, document) = pkcs8::Document::from_pem(public_key)
                    .map_err(|e| Error::KeyEncoding(e.to_string()))?;
                if label != "PUBLIC KEY" {
                    return Err(Error::KeyEncoding(format!(
                        "expected PUBLIC KEY pem, got {label}"
                    )));
                }
                Ok(document.as_bytes().to_vec())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::KeyType;

    #[test]
    fn from_str_round_trips_and_defaults() {
        assert_eq!(KeyType::from_str("rsa").unwrap(), KeyType::Rsa);
        assert_eq!(KeyType::from_str("").unwrap(), KeyType::Rsa);
        assert_eq!(KeyType::from_str("curve25519").unwrap(), KeyType::Curve25519);
        assert!(KeyType::from_str("ed448").is_err());

        for key_type in [KeyType::Rsa, KeyType::Curve25519] {
            assert_eq!(KeyType::from_str(key_type.as_str()).unwrap(), key_type);
        }
    }

    #[test]
    fn verify_dispatches_by_key_type() {
        let ed_private = crate::ed25519::private::generate().unwrap();
        let ed_public = crate::ed25519::public::from_private(&ed_private).unwrap();
        let signature = crate::ed25519::private::sign("message", &ed_private).unwrap();

        KeyType::Curve25519.verify("message", &signature, &ed_public).unwrap();
        assert!(KeyType::Rsa.verify("message", &signature, &ed_public).is_err());

        let rsa_private = crate::rsa::private::generate().unwrap();
        let rsa_private_pem = crate::rsa::private::to_string(&rsa_private).unwrap();
        let rsa_public = crate::rsa::public::from_private(&rsa_private).unwrap();
        let rsa_public_pem = crate::rsa::public::to_string(&rsa_public).unwrap();
        let signature = crate::rsa::private::sign("message", &rsa_private_pem).unwrap();

        KeyType::Rsa.verify("message", &signature, &rsa_public_pem).unwrap();
        assert!(KeyType::Curve25519.verify("message", &signature, &rsa_public_pem).is_err());
    }

    #[test]
    fn member_pubkey_der_matches_pem_body() {
        let ed_private = crate::ed25519::private::generate().unwrap();
        let ed_public = crate::ed25519::public::from_private(&ed_private).unwrap();

        let body: String = ed_public
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect();
        let expected = crate::base64::decode(&body).unwrap();
        assert_eq!(
            KeyType::Curve25519.member_pubkey_der(&ed_public).unwrap(),
            expected
        );

        let rsa_private = crate::rsa::private::generate().unwrap();
        let rsa_public = crate::rsa::public::from_private(&rsa_private).unwrap();
        let rsa_public_pem = crate::rsa::public::to_string(&rsa_public).unwrap();
        assert_eq!(
            KeyType::Rsa.member_pubkey_der(&rsa_public_pem).unwrap(),
            crate::rsa::public::to_pkcs1_der(&rsa_public_pem).unwrap()
        );
    }

    #[test]
    fn fingerprint_dispatches_by_key_type() {
        let ed_private = crate::ed25519::private::generate().unwrap();
        let ed_public = crate::ed25519::public::from_private(&ed_private).unwrap();
        assert_eq!(
            KeyType::Curve25519.fingerprint(&ed_public).unwrap(),
            crate::spki::fingerprint(&ed_public).unwrap()
        );

        let rsa_private = crate::rsa::private::generate().unwrap();
        let rsa_public = crate::rsa::public::from_private(&rsa_private).unwrap();
        let rsa_public_pem = crate::rsa::public::to_string(&rsa_public).unwrap();
        assert_eq!(
            KeyType::Rsa.fingerprint(&rsa_public_pem).unwrap(),
            crate::rsa::fingerprint(rsa_public).unwrap()
        );
    }
}
