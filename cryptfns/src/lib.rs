use bip39::{Language, Mnemonic};
use ed25519_dalek::{ExpandedSecretKey, Keypair, PublicKey, SecretKey, Signature, Verifier};
use error::{AppResult, Error};
use rand::rngs::OsRng;

/// Generate the elliptic curve keypair
pub fn generate_ed25519_keypair() -> (PublicKey, SecretKey) {
    let mut csprng = OsRng {};
    let keypair: Keypair = Keypair::generate(&mut csprng);

    (keypair.public, keypair.secret)
}

/// Convert given bytes array to mnemonic
pub fn bytes_to_mnemonic(bytes: &[u8]) -> Option<String> {
    Mnemonic::from_entropy_in(Language::English, bytes)
        .ok()
        .map(|r| r.to_string())
}

/// Convert given mnemonic to bytes vector
pub fn mnemonic_to_bytes(mnemonic: &str) -> Option<Vec<u8>> {
    let parsed_mnemonic = Mnemonic::parse_in(Language::English, mnemonic).ok()?;

    let (bytes, len) = parsed_mnemonic.to_entropy_array();

    Some(bytes[0..len].to_vec())
}

/// Sign the given message with the given secret key and public key
pub fn sign(message: &str, secret: &[u8], public: &[u8]) -> AppResult<Signature> {
    let secret = SecretKey::from_bytes(secret)?;
    let public = PublicKey::from_bytes(public)?;
    let expanded = ExpandedSecretKey::from(&secret);
    let signature = expanded.sign(message.as_bytes(), &public);

    Ok(signature)
}

/// Verify signature with the given public key
pub fn verify_signature(public_key: &str, message: &str, signature: &[u8]) -> AppResult<()> {
    let pk = public_key_from_mnemonic(public_key)
        .ok_or(Error::SignatureError("invalid_pubkey".to_string()))?;

    pk.verify(message.as_bytes(), &Signature::from_bytes(signature)?)
        .map_err(Error::from)
}

/// Convert the mnemonic phrase into a public key
pub fn public_key_from_mnemonic(mnemonic: &str) -> Option<PublicKey> {
    let bytes = mnemonic_to_bytes(mnemonic)?;

    PublicKey::from_bytes(&bytes).ok()
}

#[cfg(feature = "mock")]
pub fn get_pubkey_as_mnemonic() -> Option<String> {
    let (public, _) = generate_ed25519_keypair();

    bytes_to_mnemonic(&public.to_bytes())
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::ed25519::signature::Signature;

    use super::*;

    #[test]
    fn test_bytes_to_mnemonic() {
        let input = [
            0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8, 13u8, 14u8, 15u8,
        ];
        let expected_output =
            "abandon amount liar amount expire adjust cage candy arch gather drum buyer";

        let output = bytes_to_mnemonic(&input);
        assert_eq!(output.unwrap(), expected_output);
    }

    #[test]
    fn generate_keypair_and_convert_to_mnemonic() {
        let (public, private) = generate_ed25519_keypair();

        let mnemonic = bytes_to_mnemonic(&public.to_bytes()).unwrap();

        let bytes = mnemonic_to_bytes(&mnemonic).unwrap();

        assert_eq!(bytes, public.to_bytes());
        assert_eq!(mnemonic.split(" ").count(), 24);

        let mnemonic = bytes_to_mnemonic(&private.to_bytes()).unwrap();

        let bytes = mnemonic_to_bytes(&mnemonic).unwrap();

        assert_eq!(bytes, private.to_bytes());
        assert_eq!(mnemonic.split(" ").count(), 24);
    }

    #[test]
    fn test_mnemonic_to_bytes_gibberish() {
        let input = "this is not a valid BIP39 mnemonic phrase";

        let output = mnemonic_to_bytes(input);

        assert!(output.is_none());
    }

    #[test]
    fn test_signature_verification() {
        let (public, secret) = generate_ed25519_keypair();

        let message = "hello world";

        let signature = sign(message, &secret.to_bytes(), &public.to_bytes()).unwrap();

        let result = verify_signature(
            &bytes_to_mnemonic(&public.to_bytes()).unwrap(),
            message,
            signature.as_bytes(),
        );

        println!("{:?}", result);
        assert!(result.is_ok());
    }
}
