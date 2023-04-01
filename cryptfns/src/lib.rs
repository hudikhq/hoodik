use error::{AppResult, Error};
use rsa::{
    pkcs8::LineEnding,
    pss::{Signature, SigningKey, VerifyingKey},
    sha2::Sha256,
    signature::{RandomizedSigner, Verifier},
    Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};

pub mod base64;

/// RSA private key
///
/// Format: PKCS#8
/// Signing scheme: PSS
/// Signing algo: Sha256
/// Decryption scheme: PKCS#1 v1.5
pub type PrivateKey = RsaPrivateKey;

/// RSA public key
///
/// Format: PKCS#8
/// Signing scheme: PSS
/// Signing algo: Sha256
/// Encryption scheme: PKCS#1 v1.5
pub type PublicKey = RsaPublicKey;

/// Operations performed with a private key
pub mod private {
    use rsa::{
        pkcs8::{DecodePrivateKey, EncodePrivateKey},
        Oaep,
    };

    use super::*;

    /// Generate a new private key.
    pub fn generate() -> AppResult<PrivateKey> {
        RsaPrivateKey::new(&mut rand::thread_rng(), 2048).map_err(Error::from)
    }

    /// Convert a private key to string
    pub fn to_string(key: &PrivateKey) -> AppResult<String> {
        key.to_pkcs8_pem(LineEnding::CRLF)
            .map_err(Error::from)
            .map(|s| s.to_string())
    }

    /// Generate a new private key from string
    pub fn from_str(input: &str) -> AppResult<PrivateKey> {
        RsaPrivateKey::from_pkcs8_pem(input).map_err(Error::from)
    }

    /// Sign a message with private key
    pub fn sign_with(message: &str, key: PrivateKey) -> AppResult<String> {
        let signing_key = SigningKey::<Sha256>::from(key);
        let mut rng = rand::thread_rng();
        let signature = signing_key.try_sign_with_rng(&mut rng, message.as_bytes())?;

        Ok(hex::encode(signature))
    }

    /// Sign a message with private key input string
    pub fn sign(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        sign_with(message, key)
    }

    /// Decrypt some data with private key
    pub fn decrypt_with(data: &[u8], key: PrivateKey) -> AppResult<Vec<u8>> {
        key.decrypt(Pkcs1v15Encrypt, data).map_err(Error::from)
    }

    /// Decrypt some data with private key (pkcs1_oaep)
    pub fn decrypt_oaep_with(data: &[u8], key: PrivateKey) -> AppResult<Vec<u8>> {
        key.decrypt(Oaep::new::<Sha256>(), data)
            .map_err(Error::from)
    }

    /// Decrypt a base64 message with private key input string and output UTF8 string
    pub fn decrypt(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let decrypted = decrypt_with(base64::decode(message)?.as_slice(), key)?;

        String::from_utf8(decrypted).map_err(Error::from)
    }

    /// Decrypt a base64 message with private key input string and output UTF8 string (pkcs1_oaep)
    pub fn decrypt_oaep(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let decrypted = decrypt_oaep_with(base64::decode(message)?.as_slice(), key)?;

        String::from_utf8(decrypted).map_err(Error::from)
    }
}

/// Operations performed with a public key
pub mod public {
    use rsa::pkcs8::{DecodePublicKey, EncodePublicKey};
    use rsa::{Oaep, PublicKey as InnerRsaPublicKey};

    use super::*;

    /// Convert a public key to string
    pub fn to_string(key: &PublicKey) -> AppResult<String> {
        key.to_public_key_pem(LineEnding::CRLF).map_err(Error::from)
    }

    /// Generate a public key from private key
    pub fn from_private(private_key: &PrivateKey) -> AppResult<PublicKey> {
        Ok(private_key.to_public_key())
    }

    /// Generate a public key from string
    pub fn from_str(input: &str) -> AppResult<PublicKey> {
        RsaPublicKey::from_public_key_pem(input).map_err(Error::from)
    }

    /// Verify message with public key
    pub fn verify_with(message: &str, signature: &str, key: PublicKey) -> AppResult<()> {
        let signature_decoded = hex::decode(signature)?;
        let signature = Signature::try_from(signature_decoded.as_slice())?;
        let verifying_key = VerifyingKey::<Sha256>::from(key);

        verifying_key
            .verify(message.as_bytes(), &signature)
            .map_err(Error::from)
    }

    /// Sign a message with public key input string
    pub fn verify(message: &str, signature: &str, key: &str) -> AppResult<()> {
        let key = from_str(key)?;
        verify_with(message, signature, key)
    }

    pub fn encrypt_with(data: &[u8], key: PublicKey) -> AppResult<Vec<u8>> {
        let mut rng = rand::thread_rng();
        key.encrypt(&mut rng, Pkcs1v15Encrypt, data)
            .map_err(Error::from)
    }

    /// Encrypt a message with public key (pksc1_oaep)
    pub fn encrypt_oaep_with(data: &[u8], key: PublicKey) -> AppResult<Vec<u8>> {
        let mut rng = rand::thread_rng();
        key.encrypt(&mut rng, Oaep::new::<Sha256>(), data)
            .map_err(Error::from)
    }

    /// Encrypt a message with public key
    pub fn encrypt(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let encrypted = encrypt_with(message.as_bytes(), key)?;

        Ok(base64::encode(encrypted))
    }

    /// Encrypt a message with public key (pksc1_oaep)
    pub fn encrypt_oaep(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let encrypted = encrypt_oaep_with(message.as_bytes(), key)?;

        Ok(base64::encode(encrypted))
    }
}

#[cfg(feature = "mock")]
pub fn get_string_pubkey() -> AppResult<String> {
    public::to_string(&public::from_private(&private::generate().unwrap()).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PRIVATE_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDFzqMJ5KNQwDKy
YocJ8aolIgiAaft4JNgOfb837/V/o/XJYyIs7yXV9X3VL2wA7DDODwk3iN8ag6GM
9msEp7inqg9qfZzHIee9555t2WNfuJdH1C+99QIWLMHXlPvFqDtPHOj3j//MfAMU
3rs88fThCdgU5iw3fhMw/g4D8PTuwWR6a3tqqqecpzlSFvfWCK/5HJldAQralas6
+jkYqqqERwqli62s0kT2OS64Qq7jo0FOB93pp9HPHe4A1grb+eIoC4R5LVlnPCRk
9PMV536AhTbEYekhwSoSTSRiKXLKQ1kPsrCO1QZO1NxE49UGB42sc0k/TB+mWL28
18WJPt2lAgMBAAECggEAIvXdjP8S+k+t5idR1KkYuE1mkUOqBVcFtLH23O0VR8Tz
yO8zeBugZUtpPQePoC4ehhzUNTOEswv2vpJC4eS+1ytQZDLlRbCxY7gPIT0duipG
2pQfCATIpKCuderIAOw150qlxjN2M27roIGpOCFPdYKm5TK1N+2ZeLw+P+YTdCr8
XipNlLk5msTv132zqDPhQmnwtw5ThQ96hsRZrMcEbidsJWdiJahYWhrCuj5NbEPl
sqLQexceuoiRehJkC8h3vCCxCkJxdJuh+bIHiN/Tk2TG/KOMoPGSUger1kAS8OxJ
y8A4q8QK2h+h+U9WQyBfcQVhQNDd8uIS7KXh9RC9AQKBgQDwbUCBZIqeNYCj/1j8
Erc3K+Gq/UPE7ygOGJZatiIJQnJcNAS9cDwWKcF4DHBvtz3B3nk80pGNnDJMiNSP
612tbF6EIK+NGhshNMrCS9WvxHamo5oGM9qkAyhAE57d9k3sbMpfiZrSFCLr1JyX
0Qu8fx3BoZCn6HFTFUY3uih2wQKBgQDSnqsalCFxp7gKeRkapALH5VxHfJpRcrBC
Loxv4BOCnvJyAAZif/DR/DQ96Max9I+2UuFfyGSTVFTEQ6aNeqTWD1Hq/zD1O1uG
IaTkaL/S3Nj1dBz4guztXqSTBQ5pZhwhSr6rD8aqpQk8xybevEvVY8YQVK9F4v1S
xOa9xSBj5QKBgEHgXY1WpBinZkEJRTOEWUk3r9SvInOCaAI8wG3Ie9j3qOgUpLvX
Vc9oz4b6OZCSr8xADg4ZUCJyCuInl757aiaLi/Y+EnviDE7z7R6BsuI/PZd5Okm6
yYypBM1R0vTUeRNv15+Hz7ECLXNaxTFf6QxT9C5K+5zWNr7iFGROkKnBAoGBAKkD
uMzAWEIrU+3blcCiIrUkojOfkvqPLVA+qGXSi/WC9Y1z5au/fZIUcBvKI0CEv5qQ
0diaJ9NulgNVQl9ALuy0KImKtU/ljSGK+BZu1Jgyr0vxHJpz/grRqwFryk/cJ/Cz
WWROaZ9ghpQmQFP3CGe6BCPwwSI08BIuffeFK+PdAoGAbZE4yvCdiP/LPP5By9w9
e4oeigb96PbCjaH/81HaegUd1A2vWEdbgvIBx/ccdQgQM05EGbM0T0VaG0TCjMaZ
anp/AJ8u1eIDracvCmRatY2Z2Wk03ZPxyotzskk2kfLI6VDzGItu1geRqZR6w7qu
I4AaX1PngjIdZmeTgirqEb4=
-----END PRIVATE KEY-----";

    const TEST_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAxc6jCeSjUMAysmKHCfGq
JSIIgGn7eCTYDn2/N+/1f6P1yWMiLO8l1fV91S9sAOwwzg8JN4jfGoOhjPZrBKe4
p6oPan2cxyHnveeebdljX7iXR9QvvfUCFizB15T7xag7Txzo94//zHwDFN67PPH0
4QnYFOYsN34TMP4OA/D07sFkemt7aqqnnKc5Uhb31giv+RyZXQEK2pWrOvo5GKqq
hEcKpYutrNJE9jkuuEKu46NBTgfd6afRzx3uANYK2/niKAuEeS1ZZzwkZPTzFed+
gIU2xGHpIcEqEk0kYilyykNZD7KwjtUGTtTcROPVBgeNrHNJP0wfpli9vNfFiT7d
pQIDAQAB
-----END PUBLIC KEY-----";

    const TEST_SIGNATURE_MESSAGE: &str = "28004708";

    const TEST_SIGNATURE_HEX: &str = "3c6ccb8f3ca39074d283d698ae9f82ec7d2d3fe27157bbe54f3e30ef453417c8678a1ef74a76ee5dc7059309a367ebf86abb811482528a5ffd91deb80d328e9ab13bf83ae17b5c7eadf56706f14a071bdff5d08bb79d638f7e631fa6fc765355320b635d83b6a275d865517f22f090aa6a411a251a2a78ebec9512594a2e9e1218ddd892ed12b5af95e8e74a88a2cc6ce4ba59b00d3a403ebd14633e01753aff64b0c92c3611811f184017a2a70923946ddc59a8a923039aed128e125df98f65f304ae9b3b0644102945d839ae1977059b329fb06a688f4aeac44ddc0e5654aba12e49b1f43f5af001de01142194ed4fb7ab728446e9985e9b2495da29a8a6dd";

    const ENCRYPTED_BASE64: &str = "rfo5APfDE8Ia3cfPGI9Z3WGyTWGHz9Py5+MXNs+u+0kPTU9iAX0Gg/77ka0Dysxw3FIEVq/PxuM8Iyy2CYLLICCYuA2ymTR+4W+dqFhVs2tXh/PgFuMs9Kkj2oj4/EWPNG+h7b1pAOMwosAKFTlNYJU/m38Td+v42NSzDSSPKGRze8v1iQCbx4VkCjglts4hTzOJcmch5cf4XSnRPV+3YcuGK+wjBNAztpOAjn8rVN2+C4vgSGpsg/qmOAUo4b8DW4q+oZSEgl2druLxvpc7sTtbYcWheKnkfEN51N0FtRPd4rOIV0JpMaUvp+e2IkIvIvFGdPtSl3gdf39ujphLbQ==";

    const ENCRYPTED_BASE64_OAEP: &str = "QzlPs+Pr+Vw7Rq55qjlPogB5e24EO+B89dCjkyR/95bBuhugWRIrYI1lM+3ke0NOF5AAo9tW1GiZCaUmECUIluMR354HywkQVYewVHKiNJrLRsjVnR6flEptDFWeswbnqnS4XCWwvZUkQh+uGb6E6eX7uBvDYNYZ5wdq2AbHmBgSXtAOG1a5PLKLJ+ZoyAAuKV+cvbhox8SoCARKBHrfKVTn70xvahnS+EOerdqzXjPlT0nWiUkXILR64KIQvv4Fqt6Lx7EdaMbdnZ1I9UlO41kkfHNy2uxIlUCH1vDEYC0NkwhImJ8gQ/ZOyk0F+aOnV7wRa9eDqp/UNuNKqblmqg==";

    #[test]
    fn test_key_generating() {
        let private_key = private::generate().unwrap();
        let private_key_string = private::to_string(&private_key).unwrap();
        let private_key = private::from_str(&private_key_string).unwrap();

        let public_key = public::from_private(&private_key).unwrap();
        let public_key_string = public::to_string(&public_key).unwrap();
        let _public_key = public::from_str(&public_key_string).unwrap();
    }

    #[test]
    fn test_signing_and_verifying() {
        let private_key = private::generate().unwrap();
        let public_key = public::from_private(&private_key).unwrap();

        let message = "Hello world";

        let signature = private::sign_with(message, private_key).unwrap();

        public::verify_with(message, &signature, public_key).unwrap()
    }

    #[test]
    fn test_signature_verification_from_javascript() {
        let _private_key = private::from_str(TEST_PRIVATE_KEY).unwrap();
        let public_key = public::from_str(TEST_PUBLIC_KEY).unwrap();

        public::verify_with(TEST_SIGNATURE_MESSAGE, TEST_SIGNATURE_HEX, public_key).unwrap();
    }

    #[test]
    fn test_encrypt_and_decrypt() {
        let private_key = private::generate().unwrap();
        let private_key_string = private::to_string(&private_key).unwrap();

        let public_key = public::from_private(&private_key).unwrap();
        let public_key_string = public::to_string(&public_key).unwrap();

        let message = "hello world";
        let encrypted = public::encrypt(message, &public_key_string).unwrap();

        println!("encrypted: {}", encrypted);

        assert_ne!(message, encrypted);

        let decrypted = private::decrypt(&encrypted, &private_key_string).unwrap();

        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_encrypt_and_decrypt_oaep() {
        let private_key = private::generate().unwrap();
        let private_key_string = private::to_string(&private_key).unwrap();

        let public_key = public::from_private(&private_key).unwrap();
        let public_key_string = public::to_string(&public_key).unwrap();

        let message = "hello world";
        let encrypted = public::encrypt_oaep(message, &public_key_string).unwrap();

        println!("encrypted oaep: {}", encrypted);

        assert_ne!(message, encrypted);

        let decrypted = private::decrypt_oaep(&encrypted, &private_key_string).unwrap();

        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_decrypt_message_from_frontend() {
        let decrypted = private::decrypt(ENCRYPTED_BASE64, TEST_PRIVATE_KEY).unwrap();

        assert_eq!(decrypted, "hello world");
    }

    #[test]
    #[ignore = "This test is ignored because no matter what I try the oaep encrypted message from javascript cannot be decrypted in rust."]
    fn test_decrypt_message_from_frontend_oaep() {
        let decrypted = private::decrypt_oaep(ENCRYPTED_BASE64_OAEP, TEST_PRIVATE_KEY).unwrap();

        assert_eq!(decrypted, "hello world");
    }
}
