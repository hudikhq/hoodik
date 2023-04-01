use bip39::{Language, Mnemonic};
use error::{AppResult, Error};
use secp256k1::{ecdsa::Signature, hashes::sha256, Message, PublicKey, Secp256k1, SecretKey};
use std::str::FromStr;

/// Generate the elliptic curve keypair
pub fn generate_secp256k1_keypair() -> (PublicKey, SecretKey) {
    let private_input = Mnemonic::generate(24).unwrap().to_string();

    let private_bytes = input_to_bytes(&private_input).unwrap();

    let secret_key = SecretKey::from_slice(&private_bytes).unwrap();
    let secp = Secp256k1::signing_only();

    (secret_key.public_key(&secp), secret_key)
}

/// Convert given bytes array to input
pub fn bytes_to_input(bytes: &[u8]) -> AppResult<String> {
    Mnemonic::from_entropy_in(Language::English, bytes)
        .map(|r| r.to_string())
        .map_err(Error::from)
}

/// Convert given input to bytes vector
pub fn input_to_bytes(input: &str) -> AppResult<Vec<u8>> {
    let parsed_input = Mnemonic::parse_in(Language::English, input)?;

    let (bytes, len) = parsed_input.to_entropy_array();

    Ok(bytes[0..len].to_vec())
}

/// Generate a secret key from provided input
pub fn secret_key_from_input(input: &str) -> AppResult<SecretKey> {
    let bytes = input_to_bytes(input)?;

    Ok(SecretKey::from_slice(&bytes)?)
}

/// Convert the given hex into PublicKey
pub fn pubkey_from_hex(hex: &str) -> AppResult<PublicKey> {
    PublicKey::from_str(hex).map_err(Error::from)
}

/// Sign the given message with the given secret key and public key
pub fn sign(message: &str, secret: &[u8]) -> AppResult<Signature> {
    let message = Message::from_hashed_data::<sha256::Hash>(message.as_bytes());
    let secret = SecretKey::from_slice(secret)?;
    let secp = Secp256k1::signing_only();
    let signature = secp.sign_ecdsa(&message, &secret);

    Ok(signature)
}

/// Verify signature with the given public key
pub fn verify_signature(public_key: &str, message: &str, signature: &str) -> AppResult<()> {
    let signature = hex::decode(signature).unwrap_or_default();
    let signature = Signature::from_der(&signature)?;
    let secp = Secp256k1::verification_only();
    // let message = Message::from(message);
    let message = Message::from_hashed_data::<sha256::Hash>(message.as_bytes());
    let pk = PublicKey::from_str(public_key)?;

    secp.verify_ecdsa(&message, &signature, &pk)
        .map_err(Error::from)
}

/// Convert the input phrase into a public key
pub fn public_key_from_input(input: &str) -> AppResult<PublicKey> {
    let bytes = input_to_bytes(input)?;
    let secp = Secp256k1::signing_only();

    Ok(SecretKey::from_slice(&bytes)?.public_key(&secp))
}

#[cfg(feature = "mock")]
pub fn get_hex_pubkey() -> AppResult<String> {
    let (public, _) = generate_secp256k1_keypair();

    Ok(public.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_input() {
        let input = [
            0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8, 13u8, 14u8, 15u8,
        ];
        let expected_output =
            "abandon amount liar amount expire adjust cage candy arch gather drum buyer";

        let output = bytes_to_input(&input);
        assert_eq!(output.unwrap(), expected_output);
    }

    #[test]
    fn generate_keypair_and_convert_to_input() {
        let (_public, private) = generate_secp256k1_keypair();

        let input = bytes_to_input(private.as_ref()).unwrap();

        let bytes = input_to_bytes(&input).unwrap();

        assert_eq!(bytes, private.as_ref());
        assert_eq!(input.split(" ").count(), 24);
    }

    #[test]
    fn test_input_to_bytes_gibberish() {
        let input = "this is not a valid BIP39 input phrase";

        let output = input_to_bytes(input);

        assert!(output.is_err());
    }

    #[test]
    fn test_signature_verification() {
        let (public, secret) = generate_secp256k1_keypair();

        let message = "hello world";

        let signature = sign(message, secret.as_ref()).unwrap();

        let result = verify_signature(
            &public.to_string(),
            message,
            &signature.serialize_der().to_string(),
        );

        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_signature_from_js() {
        let input = "rough brother whip arctic actress club initial gaze romance damp maple salute merry nation drastic gate define into draft earth furnace private ship mail";
        let private_hex = "02cb66e11449016b34588974eeb1fdcfde876dd1f8cd109fb84c47c39cd2c9cf83";
        let pubkey = "02cb66e11449016b34588974eeb1fdcfde876dd1f8cd109fb84c47c39cd2c9cf83";
        let message = "28004708";

        // This is a signature produced in javascript with given input and message
        let signature = "08a6db6a61c985d79f2055e86e387dec88a8e0013e868c26946ec35829bc85d946cb61f1314db7311df604d6e6d80d32711ceaef39c1191e0ad0ee02716687e3";

        let secret_key = secret_key_from_input(input).unwrap();

        let private_hex_generated = secret_key.secret_bytes();
        let pubkey_generated = secret_key
            .public_key(&secp256k1::Secp256k1::new())
            .to_string();

        assert_eq!(pubkey, &pubkey_generated);

        let sig = sign(message, secret_key.as_ref())
            .unwrap()
            .serialize_der()
            .to_string();

        assert_eq!(signature, &sig);
    }
}
