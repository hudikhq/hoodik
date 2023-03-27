use bip39::{Language, Mnemonic};
use ed25519_dalek::{Keypair, PublicKey, SecretKey};
use rand::rngs::OsRng;

/// Generate the elliptic curve keypair
pub fn generate_ed25519_keypair() -> (PublicKey, SecretKey) {
    let mut csprng = OsRng {};
    let keypair: Keypair = Keypair::generate(&mut csprng);

    (keypair.public, keypair.secret)
}

/// Convert given bytes array to mnemonic
pub fn bytes_to_mnemonic(bytes: &[u8]) -> Option<String> {
    match Mnemonic::from_entropy_in(Language::English, bytes) {
        Ok(m) => Some(m.to_string()),
        Err(e) => {
            println!("Error converting bytes to mnemonic: {}", e);
            None
        }
    }
    // Mnemonic::from_entropy_in(Language::English, bytes)
    //     .ok()
    //     .map(|r| r.to_string())
}

/// Convert given mnemonic to bytes vector
pub fn mnemonic_to_bytes(mnemonic: &str) -> Option<Vec<u8>> {
    let parsed_mnemonic = match Mnemonic::parse_in(Language::English, mnemonic) {
        Ok(m) => m,
        Err(e) => {
            println!("Error converting mnemonic to bytes: {:#?}", e);
            return None;
        }
    };
    // let parsed_mnemonic = Mnemonic::parse_in(Language::English, mnemonic).ok()?;

    let (bytes, len) = parsed_mnemonic.to_entropy_array();

    Some(bytes[0..len].to_vec())
}

#[cfg(feature = "mock")]
pub fn get_pubkey_as_mnemonic() -> Option<String> {
    let (public, _) = generate_ed25519_keypair();

    bytes_to_mnemonic(&public.to_bytes())
}

#[cfg(test)]
mod tests {
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
}
