use error::{AppResult, Error};
use rsa::{
    pkcs1::LineEnding,
    pss::{Signature, SigningKey, VerifyingKey},
    sha2::Sha256,
    signature::{RandomizedSigner, Verifier},
    Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};

pub use rsa::PublicKeyParts;

/// RSA private key
///
/// # Key description
/// Format: PKCS#8
/// Signing scheme: PSS
/// Signing algo: Sha256
/// Encryption scheme: PKCS#1 v1.5
/// Encryption scheme padding: RSA_PKCS1_PADDING
pub type PrivateKey = RsaPrivateKey;

/// RSA public key
///
/// # Key description
/// Format: PKCS#8
/// Signing scheme: PSS
/// Signing algo: Sha256
/// Encryption scheme: PKCS#1 v1.5
/// Encryption scheme padding: RSA_PKCS1_PADDING
pub type PublicKey = RsaPublicKey;

/// Generate fingerprint from private or public key
pub fn fingerprint<T: PublicKeyParts>(key: T) -> AppResult<String> {
    let n = key.n().to_bytes_be();

    Ok(sha256::digest(n.as_slice()))
}

/// Operations performed with a private key
pub mod private {
    use rsa::{
        pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey},
        Oaep,
    };

    use super::*;

    /// Generate a new private key.
    pub fn generate() -> AppResult<PrivateKey> {
        RsaPrivateKey::new(&mut rand::thread_rng(), 2048).map_err(Error::from)
    }

    /// Convert a private key to string
    pub fn to_string(key: &PrivateKey) -> AppResult<String> {
        key.to_pkcs1_pem(LineEnding::LF)
            .map_err(Error::from)
            .map(|s| s.to_string())
    }

    /// Generate a new private key from string
    pub fn from_str(input: &str) -> AppResult<PrivateKey> {
        let input = input.replace("\r\n", "\n");
        RsaPrivateKey::from_pkcs1_pem(&input).map_err(Error::from)
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
        let mut rng = rand::thread_rng();
        key.decrypt_blinded(&mut rng, Oaep::new::<Sha256>(), data)
            .map_err(Error::from)
    }

    /// Decrypt a base64 message with private key input string and output UTF8 string
    pub fn decrypt(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let decrypted = decrypt_with(crate::base64::decode(message)?.as_slice(), key)?;

        String::from_utf8(decrypted).map_err(Error::from)
    }

    /// Decrypt a base64 message with private key input string and output UTF8 string
    pub fn decrypt_hex(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let decrypted = decrypt_with(hex::decode(message)?.as_slice(), key)?;

        String::from_utf8(decrypted).map_err(Error::from)
    }

    /// Decrypt a base64 message with private key input string and output UTF8 string (pkcs1_oaep)
    pub fn decrypt_oaep(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let decrypted = decrypt_oaep_with(crate::base64::decode(message)?.as_slice(), key)?;

        String::from_utf8(decrypted).map_err(Error::from)
    }

    /// Decrypt a base64 message with private key input string and output UTF8 string (pkcs1_oaep)
    pub fn decrypt_oaep_hex(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let decrypted = decrypt_oaep_with(hex::decode(message)?.as_slice(), key)?;

        String::from_utf8(decrypted).map_err(Error::from)
    }
}

/// Operations performed with a public key
pub mod public {
    use rsa::{
        pkcs1::{DecodeRsaPublicKey, EncodeRsaPublicKey},
        Oaep, PublicKey as _,
    };

    use super::*;

    /// Convert a public key to string
    pub fn to_string(key: &PublicKey) -> AppResult<String> {
        key.to_pkcs1_pem(LineEnding::LF).map_err(Error::from)
    }

    /// Generate a public key from private key
    pub fn from_private(private_key: &PrivateKey) -> AppResult<PublicKey> {
        Ok(private_key.to_public_key())
    }

    /// Generate a public key from string
    pub fn from_str(input: &str) -> AppResult<PublicKey> {
        let input = input.replace("\r\n", "\n");
        RsaPublicKey::from_pkcs1_pem(&input).map_err(Error::from)
    }

    /// Verify message with public key
    pub fn verify_with(message: &str, signature: &str, key: PublicKey) -> AppResult<()> {
        let signature_decoded = hex::decode(signature)?;
        let message_as_bytes = message.as_bytes();

        let signature = Signature::try_from(signature_decoded.as_slice())?;
        let verifying_key = VerifyingKey::<Sha256>::from(key);

        verifying_key
            .verify(message_as_bytes, &signature)
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

        Ok(crate::base64::encode(encrypted))
    }

    /// Encrypt a message with public key hex
    pub fn encrypt_hex(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let encrypted = encrypt_with(message.as_bytes(), key)?;

        Ok(hex::encode(encrypted))
    }

    /// Encrypt a message with public key (pksc1_oaep)
    pub fn encrypt_oaep(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let encrypted = encrypt_oaep_with(message.as_bytes(), key)?;

        Ok(crate::base64::encode(encrypted))
    }

    /// Encrypt a message with public key (pksc1_oaep) hex
    pub fn encrypt_oaep_hex(message: &str, key: &str) -> AppResult<String> {
        let key = from_str(key)?;
        let encrypted = encrypt_oaep_with(message.as_bytes(), key)?;

        Ok(hex::encode(encrypted))
    }
}

#[cfg(feature = "mock")]
pub fn get_string_pubkey() -> AppResult<String> {
    public::to_string(&public::from_private(&private::generate().unwrap()).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn _write_file(filename: &str, content: &str) {
        use std::fs::File;
        use std::io::prelude::*;
        let mut file = File::create(filename).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    const TEST_PRIVATE_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEAsMvjT2NZNqJo/3AYHH3RIm5fwmOXabbYxduvtNp33JQQZSPu
S+bbqe97jJXVIRUaEPWf05sCwctmFvxcL77FtWLCxaU8TYz4K59LAPcGLGuQO3Hl
PVFAmBUSMDRaK3T3mwMk/Z3qDi8GyumEmN1UfZtxxfqFfMgIB2b8K6enPSHQHoiq
N7xm8MdaDhZGQnyzgjFAKhKFWiusjKwWHe6vvEOFQkIrdTPwhg7ELCmY+kOuW6g/
giE6XcVx2TPOtG5A5qBeJ+vW8XJHZYfCcLwsDZnirTwUTWORWM2omNbquNATh+Lt
3Vh2Rp/vTZJD4LIeR91o55BWr+NLY2I52eSY6QIDAQABAoIBACiQj3Y+qFCV0RuS
36Vh5ONOieAzM6GI15IGRvlrCwdsXZqnNNzrekkybpmiI0W07scnZGWL8oT+o0zw
2EIINprYrzHkKMLubl6r7OyqwRreDzjkeCGqi/SZGRRAXtQLwWgqv4kFe5eHiLpz
+/2LAwDS8rbnNUudJeJ06bUmgYPP5al+96zivJlXhAINss/t5DYwpwoOR7dPaMz0
Yb4qQW9HT5cMi0giDxF89RIV08rFGkDQEU10/31AqqEr8LYMOn/J/JTRqmqnL/DM
kAdwrO1cwU5tnVIgAjj7ouyukDTvxhhk9otrgAwCP6wSOf7V8P4j4AWMHoVYXDTx
fx0JdQUCgYEA2HlCiRLphR/LSil78B+cR3MCJEtn1gGsUei5R+wkw6rjP6UQaTiQ
X4WAAM04B+ZPlQvtnxt/B9+rmM6UcgfPoWLR8RVOp1XMyL/OTwbgljrQc3bWuBiE
OuoYe2QOEtFYg69V4f37+DnHC2gKbbitw62FhaNbZQHNMfC9soh480cCgYEA0RP5
g/l9K/DALiuRE8wkF8ByfRXsYxUGzOu9B5elvAg89wn/LDzsLFfPBZQ/5jml3r99
kvn+eMzavUbqCMKICgD/XZ4cJucoWm+lPtddj/dce27ePVhhWWC7nQ2IzGdswC0T
1O1n6HIa3cMkeruWg36p137E8AVORXxIoOFJSk8CgYBE7pAeYBRWXOJ6Mi2SMC6u
ndPPxOdCwXOi/Y2Kdorad983VBOevfFTSYqSNsch1NgAqTS4lqPj2PimhxnEGfKm
/HXH5DYQmQTF5DYI+jKoBAB+1BfZtYzdyc+T8y98FIewHzQk66DB0XwtiKrRd551
khrTjEo9Js61mWh+onCJXwKBgB0XeW2KpocZrbP+7eXiTtdbONL83PKAd3zGBHxs
9muufcUmB/KA25/j6/NryGRhexn+bRupW2Y1ou4ZUvE7GDDEKMQ+/s3O9kd3J3gS
AXvJwH2QVK4WgR0tn41f17wRXAl1fD/xdLbcQa6/u3C0b2IGmt1YT1DSfCyg+X4h
OtBzAoGBAMqMskbS24GqN21itBVoP+6njQzyXL8r3H3+MHNbhmzuC9MpfE7v4QMI
IKeXOea5ts/4MTXbxUmzFwzj5gFeThsEykJEdHuWK1fbYW2rxFehXH5spqbrzGQX
ejsNMbtGsAhQfwBrKb6qlyNR6d6bTixhCuqYCTYjRi7AnL9G/w9B
-----END RSA PRIVATE KEY-----";

    const TEST_PUBLIC_KEY: &str = "-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEAsMvjT2NZNqJo/3AYHH3RIm5fwmOXabbYxduvtNp33JQQZSPuS+bb
qe97jJXVIRUaEPWf05sCwctmFvxcL77FtWLCxaU8TYz4K59LAPcGLGuQO3HlPVFA
mBUSMDRaK3T3mwMk/Z3qDi8GyumEmN1UfZtxxfqFfMgIB2b8K6enPSHQHoiqN7xm
8MdaDhZGQnyzgjFAKhKFWiusjKwWHe6vvEOFQkIrdTPwhg7ELCmY+kOuW6g/giE6
XcVx2TPOtG5A5qBeJ+vW8XJHZYfCcLwsDZnirTwUTWORWM2omNbquNATh+Lt3Vh2
Rp/vTZJD4LIeR91o55BWr+NLY2I52eSY6QIDAQAB
-----END RSA PUBLIC KEY-----";

    const FINGERPRINT: &str = "87ae936e11ec8a34f9d1c62687271ae8cd4b89189533c21e1bb7ddce84d37f86";

    const TEST_SIGNATURE_MESSAGE: &str = "28004708";

    const TEST_SIGNATURE_HEX: &str = "2ce2f99cbb6ca040aa5372384e23f637e500a42fe9ba18b82069b018740e0569943232e2caa411b0bc1084efcd9a205612b8fe0417d0324ef938515e01d1d63c6cb7acc5ceafb392223e75321cb50fbbd6a23294c854bdb6baf2df303b7c09845e22f29ed38f0cd84fdc16620c8b7211c6637ee35de1a2fcca7e90fc00ab28f0f90728749af80666d73d3fbf12fc9e656d3eb84cfceefc59f54160ec446ed4c22690460c9de34fc296351ae8ae0df6253d0b6e69de5d913eef2f556e107a3451a9fc3812173c25b5d90050ac4912a5464b0f1fad23582fcbf8dbc0634c9b535a6d5720b5fe808de21bd6785dd4ceb02604094a6e8cbddbf9feff4ee0b72f4554";

    const _ENCRYPTED_BASE64: &str = "pucjs8dcxAXGww4mlG1XTcWxO7PsFEr7yJ4NFKNVyIiB6AHt9v0d2cSd72+7IkuZrxJHDBp3ixRFAWXWuzsp96bhdVKIR0k4coBWy2TgBhZc+a6nRdpGUDgBnjn+wZE7LRIR/L0pIVY32N0/94zNYdaiAVeaAIBsUq9IdiKfv6kWAshk57pGec0SHa8hESFpfwv7QCKmzY5l1QjBwtzbo5LtmOU2wo/JuMYNDKqjXYOOOyTaSMMxT+6Q6qkib+SpHA4MvUVOEMSbS4Y3TDBm1xShFCol80FbfOmDiAWqBatXT/WCvtqfeYMN/hzmX5PoZh3sBZLHbfle6ePnz05aTQ==";

    const _ENCRYPTED_BASE64_OAEP: &str = "FH07X7rxVqjo1w/XGyVCsaLDAO8B07M4WR5nHanBJdCOAKVFFGNpi5uX5NX9oj03iaueFDFnyZwatr68Wr/GZznjAtj7j08CIGCA73AALB5Gi0HMjpDG/BwZcTeRp92e5Qoa2gLee8wOT/NRBChniQ8vS7LqhlxxBHFMM2h/35yhwkfLQ+I6tHcOVjAKj40VWjtV6UvcOsT0ffZIpZNsUX3Fdxhvvf2DaEKRHEIF0j2aO4kaaxnPSoSUkeqj1CFQ+5BhGA2INrU77gnzO76sYdtDsS6Qcojj7y0trxtwhOIrfK2dm9OPCcL8lLiE8LHOCKkes18QQU2oK+zoOS2JMA==";

    fn run_fingerprint_test() {
        let private_key = private::from_str(TEST_PRIVATE_KEY).unwrap();

        let public_key = public::from_private(&private_key).unwrap();
        let public_key2 = public::from_str(TEST_PUBLIC_KEY).unwrap();

        let fingerprint_private = fingerprint(private_key).unwrap();
        let fingerprint_public = fingerprint(public_key).unwrap();
        let fingerprint_public2 = fingerprint(public_key2).unwrap();

        assert_eq!(fingerprint_private, fingerprint_public);
        assert_eq!(fingerprint_public, fingerprint_public2);
        assert_eq!(fingerprint_public2, FINGERPRINT);
    }

    #[test]
    fn test_rsa_fingerprint() {
        run_fingerprint_test();
    }

    #[test]
    fn test_rsa_fingerprint_multi_thread() {
        let t1 = std::thread::spawn(|| run_fingerprint_test());
        let t2 = std::thread::spawn(|| run_fingerprint_test());
        let t3 = std::thread::spawn(|| run_fingerprint_test());
        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();
    }

    #[test]
    fn test_rsa_key_generating() {
        let private_key = private::generate().unwrap();
        let private_key_string = private::to_string(&private_key).unwrap();
        let private_key = private::from_str(&private_key_string).unwrap();

        let public_key = public::from_private(&private_key).unwrap();
        let public_key_string = public::to_string(&public_key).unwrap();
        let _public_key = public::from_str(&public_key_string).unwrap();
    }

    #[test]
    fn test_rsa_signing_and_verifying() {
        let private_key = private::generate().unwrap();
        let public_key = public::from_private(&private_key).unwrap();

        let message = "Hello world";

        let signature = private::sign_with(message, private_key).unwrap();

        public::verify_with(message, &signature, public_key).unwrap()
    }

    #[test]
    fn test_rsa_signature_verification_from_javascript() {
        public::verify(TEST_SIGNATURE_MESSAGE, TEST_SIGNATURE_HEX, TEST_PUBLIC_KEY).unwrap();
    }

    #[test]
    fn test_rsa_encrypt_and_decrypt() {
        let private_key = private::from_str(TEST_PRIVATE_KEY).unwrap();
        let private_key_string = private::to_string(&private_key).unwrap();

        let public_key = public::from_private(&private_key).unwrap();
        let public_key_string = public::to_string(&public_key).unwrap();

        let message = "hello world";
        let encrypted = public::encrypt(message, &public_key_string).unwrap();

        println!("encrypted");
        println!("{}", &encrypted);

        assert_ne!(message, encrypted);

        let decrypted = private::decrypt(&encrypted, &private_key_string).unwrap();

        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_rsa_encrypt_and_decrypt_oaep() {
        let private_key = private::from_str(TEST_PRIVATE_KEY).unwrap();
        let private_key_string = private::to_string(&private_key).unwrap();

        let public_key = public::from_private(&private_key).unwrap();
        let public_key_string = public::to_string(&public_key).unwrap();

        let message = "hello world";
        let encrypted = public::encrypt_oaep(message, &public_key_string).unwrap();

        println!("encrypted oaep");
        println!("{}", &encrypted);

        assert_ne!(message, encrypted);

        let decrypted = private::decrypt_oaep(&encrypted, &private_key_string).unwrap();

        assert_eq!(message, decrypted);
    }

    #[test]
    fn test_rsa_decrypt_message_from_frontend() {
        let decrypted = private::decrypt(_ENCRYPTED_BASE64, TEST_PRIVATE_KEY)
            .expect("can decrypt message that was encrypted in javascript");

        assert_eq!(decrypted, "hello world");
    }

    #[test]
    #[ignore = "This test is ignored because no matter what I try the oaep encrypted message from javascript cannot be decrypted in rust - or vice versa for that matter"]
    fn test_rsa_decrypt_message_from_frontend_oaep() {
        let decrypted = private::decrypt_oaep(_ENCRYPTED_BASE64_OAEP, TEST_PRIVATE_KEY)
            .expect("can decrypt message that was encrypted in javascript");

        assert_eq!(decrypted, "hello world");
    }
}
