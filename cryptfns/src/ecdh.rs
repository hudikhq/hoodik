//! Hybrid file-key wrapping: X25519 ECDH combined with ML-KEM-768.
//!
//! A file key is wrapped to a recipient under *both* an X25519 shared secret
//! and an ML-KEM-768 encapsulation. An adversary who harvests a wrap today and
//! later builds a quantum computer still faces X25519; an adversary who breaks
//! ML-KEM still faces X25519. The wrap is only as weak as the stronger of the
//! two, which is the whole reason it is hybrid rather than pure post-quantum —
//! every Rust ML-KEM implementation is still pre-1.0 and unaudited.
//!
//! ```text
//! eph_sk, eph_pk = X25519 keygen                       (fresh per wrap)
//! ss_x           = X25519(eph_sk, recipient.x_pk)      (32 B)
//! mk_ct, ss_k    = ML-KEM-768.Encaps(recipient.mk_ek)  (ct 1088 B, ss 32 B)
//! ikm            = ss_k ‖ ss_x                          (ML-KEM first)
//! salt           = eph_pk ‖ mk_ct ‖ x_pk ‖ mk_ek
//! wrap_key       = HKDF-SHA256(ikm, salt, info = WRAP_INFO)
//! blob           = 0x02 ‖ eph_pk ‖ mk_ct ‖ nonce ‖ AEGIS-256(wrap_key ‖ nonce, file_key)
//! ```
//!
//! ML-KEM is not key-binding: one ciphertext can decapsulate under different
//! keys. Binding `mk_ct` and both long-term public keys into the HKDF salt ties
//! the derived key to this exact recipient, closing that gap. Decapsulation uses
//! implicit rejection and never fails — a wrong key yields a wrong `ss_k` and the
//! AEGIS tag, not the KEM, is what rejects it. The wrap AEAD is fixed to
//! AEGIS-256; the construction is versioned through [`WRAP_INFO`] and the leading
//! blob byte, never negotiated per wrap.

use crate::error::{CryptoResult, Error};

use hkdf::Hkdf;
use libcrux_ml_kem::mlkem768::{self, MlKem768Ciphertext, MlKem768PrivateKey, MlKem768PublicKey};
use libcrux_ml_kem::{KEY_GENERATION_SEED_SIZE, SHARED_SECRET_SIZE};
use pkcs8::der::pem::{decode_vec, encode_string, LineEnding};
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};
use zeroize::Zeroizing;

const X_PUBLIC_LEN: usize = 32;
const X_SECRET_LEN: usize = 32;
const MK_EK_LEN: usize = 1184;
const MK_DK_LEN: usize = 2400;
const MK_CT_LEN: usize = 1088;
/// Offset of the ML-KEM encapsulation key inside the decapsulation key. FIPS 203
/// stores `dk = dk_PKE ‖ ek ‖ H(ek) ‖ z`, and for ML-KEM-768 `dk_PKE` is 1152 B,
/// so the 1184-byte `ek` begins here. Recovering `ek` from `dk` lets the private
/// container omit it, keeping the sealed bundle to `x_sk ‖ mk_dk`.
const MK_DK_EK_OFFSET: usize = 1152;

const NONCE_LENGTH: usize = 32;
const TAG_LENGTH: usize = 16;

const CONTAINER_VERSION: u8 = 1;
/// Leading byte of a wrap blob. `0x01` is reserved for the X25519-only format
/// that never shipped and is rejected outright — there are no `0x01` blobs to be
/// compatible with, so accepting one could only ever be a downgrade.
const BLOB_VERSION: u8 = 0x02;

const PUBLIC_LABEL: &str = "HOODIK WRAPPING KEY";
const PRIVATE_LABEL: &str = "HOODIK WRAPPING PRIVATE KEY";

const PUBLIC_CONTAINER_LEN: usize = 1 + X_PUBLIC_LEN + MK_EK_LEN;
const PRIVATE_CONTAINER_LEN: usize = 1 + X_SECRET_LEN + MK_DK_LEN;

const WRAP_INFO: &[u8] = b"hoodik-file-key-wrap-v2";

/// Wrap a file key for a recipient's hybrid wrapping public key (PEM container).
pub fn wrap(file_key: &[u8], recipient_public_key: &str) -> CryptoResult<String> {
    let container = public::from_str(recipient_public_key)?;
    let x_pk = &container[1..1 + X_PUBLIC_LEN];
    let mk_ek = &container[1 + X_PUBLIC_LEN..];
    let recipient_x = PublicKey::from(to_array::<X_PUBLIC_LEN>(x_pk)?);
    let recipient_mk =
        MlKem768PublicKey::try_from(mk_ek).map_err(|_| Error::InvalidLength("mk_ek length"))?;

    let ephemeral = EphemeralSecret::random_from_rng(rand::rngs::OsRng);
    let ephemeral_public = PublicKey::from(&ephemeral);
    let ss_x = ephemeral.diffie_hellman(&recipient_x);
    reject_non_contributory(&ss_x)?;

    let mut encaps_seed = Zeroizing::new([0u8; SHARED_SECRET_SIZE]);
    getrandom::getrandom(encaps_seed.as_mut_slice())?;
    let (mk_ct, ss_k) = mlkem768::encapsulate(&recipient_mk, *encaps_seed);
    let ss_k = Zeroizing::new(ss_k);

    let wrap_key = derive_wrap_key(
        ss_k.as_slice(),
        ss_x.as_bytes(),
        ephemeral_public.as_bytes(),
        mk_ct.as_ref(),
        x_pk,
        mk_ek,
    )?;

    let mut nonce = vec![0u8; NONCE_LENGTH];
    getrandom::getrandom(&mut nonce)?;

    let mut aead_key = Zeroizing::new(wrap_key.to_vec());
    aead_key.extend_from_slice(&nonce);
    let ciphertext = crate::aegis256::encrypt(aead_key.to_vec(), file_key.to_vec())?;

    let mut blob = Vec::with_capacity(1 + X_PUBLIC_LEN + MK_CT_LEN + NONCE_LENGTH + ciphertext.len());
    blob.push(BLOB_VERSION);
    blob.extend_from_slice(ephemeral_public.as_bytes());
    blob.extend_from_slice(mk_ct.as_ref());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);

    Ok(crate::base64::encode(blob))
}

/// Unwrap a file key with the recipient's hybrid wrapping private key (PEM).
pub fn unwrap(blob: &str, private_key: &str) -> CryptoResult<Vec<u8>> {
    let blob = crate::base64::decode(blob)?;
    if blob.len() < 1 + X_PUBLIC_LEN + MK_CT_LEN + NONCE_LENGTH + TAG_LENGTH {
        return Err(Error::InvalidLength("hybrid wrap blob too short"));
    }
    if blob[0] != BLOB_VERSION {
        return Err(Error::KeyEncoding(format!("unsupported wrap version {}", blob[0])));
    }

    let ct_offset = 1 + X_PUBLIC_LEN + MK_CT_LEN + NONCE_LENGTH;
    let eph_pk = to_array::<X_PUBLIC_LEN>(&blob[1..1 + X_PUBLIC_LEN])?;
    let mk_ct = MlKem768Ciphertext::try_from(&blob[1 + X_PUBLIC_LEN..1 + X_PUBLIC_LEN + MK_CT_LEN])
        .map_err(|_| Error::InvalidLength("mk_ct length"))?;
    let nonce = &blob[1 + X_PUBLIC_LEN + MK_CT_LEN..ct_offset];
    let ciphertext = &blob[ct_offset..];

    let secret = private::parse(private_key)?;
    let x_secret = StaticSecret::from(secret.x_secret());
    let own_x_pk = PublicKey::from(&x_secret);
    let eph_public = PublicKey::from(eph_pk);
    let ss_x = x_secret.diffie_hellman(&eph_public);
    reject_non_contributory(&ss_x)?;

    // libcrux's private-key type is a bare newtype with no Drop; the zeroized
    // master copy is `secret`, and this decaps key lives only for the call.
    let decaps_key = MlKem768PrivateKey::try_from(secret.mk_decaps())
        .map_err(|_| Error::InvalidLength("mk_dk length"))?;
    let ss_k = Zeroizing::new(mlkem768::decapsulate(&decaps_key, &mk_ct));

    let wrap_key = derive_wrap_key(
        ss_k.as_slice(),
        ss_x.as_bytes(),
        &eph_pk,
        mk_ct.as_ref(),
        own_x_pk.as_bytes(),
        secret.mk_encaps(),
    )?;

    let mut aead_key = Zeroizing::new(wrap_key.to_vec());
    aead_key.extend_from_slice(nonce);

    crate::aegis256::decrypt(aead_key.to_vec(), ciphertext.to_vec())
}

#[allow(clippy::too_many_arguments)]
fn derive_wrap_key(
    ss_k: &[u8],
    ss_x: &[u8; X_PUBLIC_LEN],
    ephemeral_public: &[u8],
    mk_ct: &[u8],
    recipient_x_pk: &[u8],
    recipient_mk_ek: &[u8],
) -> CryptoResult<Zeroizing<[u8; 32]>> {
    let mut ikm = Zeroizing::new(Vec::with_capacity(ss_k.len() + ss_x.len()));
    ikm.extend_from_slice(ss_k);
    ikm.extend_from_slice(ss_x);

    let mut salt =
        Vec::with_capacity(ephemeral_public.len() + mk_ct.len() + recipient_x_pk.len() + recipient_mk_ek.len());
    salt.extend_from_slice(ephemeral_public);
    salt.extend_from_slice(mk_ct);
    salt.extend_from_slice(recipient_x_pk);
    salt.extend_from_slice(recipient_mk_ek);

    let mut wrap_key = Zeroizing::new([0u8; 32]);
    Hkdf::<Sha256>::new(Some(&salt), &ikm)
        .expand(WRAP_INFO, wrap_key.as_mut_slice())
        .map_err(|e| Error::KeyEncoding(e.to_string()))?;

    Ok(wrap_key)
}

/// An all-zero X25519 output means a low-order or identity peer point — the DH
/// contributed nothing and the "secret" is attacker-known.
fn reject_non_contributory(shared: &x25519_dalek::SharedSecret) -> CryptoResult<()> {
    if shared.as_bytes().iter().all(|b| *b == 0) {
        return Err(Error::KeyEncoding("non-contributory x25519 public key".to_string()));
    }
    Ok(())
}

fn to_array<const N: usize>(bytes: &[u8]) -> CryptoResult<[u8; N]> {
    bytes.try_into().map_err(|_| Error::InvalidLength("unexpected key component length"))
}

fn encode_pem(label: &str, body: &[u8]) -> CryptoResult<String> {
    encode_string(label, LineEnding::LF, body).map_err(|e| Error::KeyEncoding(e.to_string()))
}

fn decode_pem(expected_label: &str, pem: &str) -> CryptoResult<Vec<u8>> {
    let (label, data) =
        decode_vec(pem.as_bytes()).map_err(|e| Error::KeyEncoding(e.to_string()))?;
    if label != expected_label {
        return Err(Error::KeyEncoding(format!("expected {expected_label} pem, got {label}")));
    }
    Ok(data)
}

pub mod private {
    use super::*;

    /// A parsed private wrapping key. The raw `version ‖ x_sk ‖ mk_dk` bytes stay
    /// in a [`Zeroizing`] buffer for the value's whole life; the accessors hand
    /// out short-lived views.
    pub(super) struct HybridSecret {
        body: Zeroizing<Vec<u8>>,
    }

    impl HybridSecret {
        pub(super) fn x_secret(&self) -> [u8; X_SECRET_LEN] {
            let mut out = [0u8; X_SECRET_LEN];
            out.copy_from_slice(&self.body[1..1 + X_SECRET_LEN]);
            out
        }

        pub(super) fn mk_decaps(&self) -> &[u8] {
            &self.body[1 + X_SECRET_LEN..]
        }

        pub(super) fn mk_encaps(&self) -> &[u8] {
            &self.mk_decaps()[MK_DK_EK_OFFSET..MK_DK_EK_OFFSET + MK_EK_LEN]
        }
    }

    pub(super) fn parse(pem: &str) -> CryptoResult<HybridSecret> {
        let body = Zeroizing::new(decode_pem(PRIVATE_LABEL, pem)?);
        if body.len() != PRIVATE_CONTAINER_LEN {
            return Err(Error::InvalidLength("wrapping private key wrong length"));
        }
        if body[0] != CONTAINER_VERSION {
            return Err(Error::KeyEncoding(format!(
                "unsupported wrapping key version {}",
                body[0]
            )));
        }
        Ok(HybridSecret { body })
    }

    /// Generate a hybrid wrapping private key as a PEM container.
    pub fn generate() -> CryptoResult<String> {
        let x_secret = StaticSecret::random_from_rng(rand::rngs::OsRng);
        let x_bytes = Zeroizing::new(x_secret.to_bytes());

        let mut seed = Zeroizing::new([0u8; KEY_GENERATION_SEED_SIZE]);
        getrandom::getrandom(seed.as_mut_slice())?;
        let keypair = mlkem768::generate_key_pair(*seed);

        let mut body = Zeroizing::new(Vec::with_capacity(PRIVATE_CONTAINER_LEN));
        body.push(CONTAINER_VERSION);
        body.extend_from_slice(x_bytes.as_slice());
        body.extend_from_slice(keypair.private_key().as_slice());

        encode_pem(PRIVATE_LABEL, &body)
    }
}

pub mod public {
    use super::*;

    /// Parse and validate a hybrid wrapping public key PEM, returning the raw
    /// `version ‖ x_pk ‖ mk_ek` container bytes. This is the canonical the
    /// key-transition certificate commits to, and the validation registration
    /// runs — a bare X25519 SPKI key fails here on its label.
    pub fn from_str(pem: &str) -> CryptoResult<Vec<u8>> {
        let body = decode_pem(PUBLIC_LABEL, pem)?;
        if body.len() != PUBLIC_CONTAINER_LEN {
            return Err(Error::InvalidLength("wrapping public key wrong length"));
        }
        if body[0] != CONTAINER_VERSION {
            return Err(Error::KeyEncoding(format!(
                "unsupported wrapping key version {}",
                body[0]
            )));
        }
        Ok(body)
    }

    /// Derive the public wrapping key PEM from a private wrapping key PEM.
    pub fn from_private(private_key: &str) -> CryptoResult<String> {
        let secret = private::parse(private_key)?;
        let x_public = PublicKey::from(&StaticSecret::from(secret.x_secret()));

        let mut body = Vec::with_capacity(PUBLIC_CONTAINER_LEN);
        body.push(CONTAINER_VERSION);
        body.extend_from_slice(x_public.as_bytes());
        body.extend_from_slice(secret.mk_encaps());

        encode_pem(PUBLIC_LABEL, &body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keypair() -> (String, String) {
        let private = private::generate().unwrap();
        let public = public::from_private(&private).unwrap();
        (private, public)
    }

    #[test]
    fn component_lengths_match_libcrux() {
        assert_eq!(MK_EK_LEN, MlKem768PublicKey::len());
        assert_eq!(MK_DK_LEN, MlKem768PrivateKey::len());
        assert_eq!(MK_CT_LEN, MlKem768Ciphertext::len());
        assert_eq!(SHARED_SECRET_SIZE, 32);
    }

    #[test]
    fn extracted_encaps_key_matches_keypair() {
        // The FIPS-203 offset must pick out exactly the encapsulation key the
        // keypair carries, or a wrap to the derived public key would not unwrap.
        let seed = [4u8; KEY_GENERATION_SEED_SIZE];
        let keypair = mlkem768::generate_key_pair(seed);
        let dk = keypair.private_key().as_slice();
        assert_eq!(
            &dk[MK_DK_EK_OFFSET..MK_DK_EK_OFFSET + MK_EK_LEN],
            keypair.public_key().as_slice()
        );
    }

    #[test]
    fn container_lengths() {
        let (private, public) = keypair();
        let priv_body = decode_pem(PRIVATE_LABEL, &private).unwrap();
        let pub_body = decode_pem(PUBLIC_LABEL, &public).unwrap();
        assert_eq!(priv_body.len(), 1 + 32 + 2400);
        assert_eq!(pub_body.len(), 1 + 32 + 1184);
    }

    #[test]
    fn pem_labels_and_reparse() {
        let (private, public) = keypair();
        assert!(private.contains("BEGIN HOODIK WRAPPING PRIVATE KEY"));
        assert!(public.contains("BEGIN HOODIK WRAPPING KEY"));
        private::parse(&private).unwrap();
        public::from_str(&public).unwrap();
    }

    #[test]
    fn from_str_rejects_bare_x25519_and_identity_keys() {
        // A bare X25519 SPKI key — the never-shipped wrapping format — is
        // refused, so registration and the transition certificate only ever
        // accept the hybrid container.
        const X25519_SPKI: &str = "-----BEGIN PUBLIC KEY-----\n\
            MCowBQYDK2VuAyEAMj7hOamJF96N+WAmBu691xekmKrEAA5XBhHjQAWgi24=\n\
            -----END PUBLIC KEY-----\n";
        assert!(public::from_str(X25519_SPKI).is_err());

        let ed = crate::ed25519::public::from_private(&crate::ed25519::private::generate().unwrap())
            .unwrap();
        assert!(public::from_str(&ed).is_err());
    }

    #[test]
    fn from_str_returns_container_canonical() {
        // The bytes the transition certificate commits to are the container's
        // `version ‖ x_pk ‖ mk_ek`, and they round-trip the PEM exactly.
        let (_, public) = keypair();
        let canonical = public::from_str(&public).unwrap();
        assert_eq!(canonical.len(), PUBLIC_CONTAINER_LEN);
        assert_eq!(canonical[0], CONTAINER_VERSION);
        assert_eq!(canonical, decode_pem(PUBLIC_LABEL, &public).unwrap());
    }

    #[test]
    fn wrap_unwrap_round_trip() {
        let (private, public) = keypair();
        let file_key = crate::aegis256::generate_key().unwrap();

        let blob = wrap(&file_key, &public).unwrap();
        assert_eq!(unwrap(&blob, &private).unwrap(), file_key);
    }

    #[test]
    fn empty_and_large_and_multibyte_payloads() {
        let (private, public) = keypair();
        for payload in [
            Vec::new(),
            vec![0u8; 1024 * 64],
            "ključ-датотеке-鍵".as_bytes().to_vec(),
        ] {
            let blob = wrap(&payload, &public).unwrap();
            assert_eq!(unwrap(&blob, &private).unwrap(), payload);
        }
    }

    #[test]
    fn wrong_x25519_key_fails() {
        // Same ML-KEM key, different X25519 half: the hybrid must still reject.
        let (private, public) = keypair();
        let file_key = crate::aegis256::generate_key().unwrap();
        let blob = wrap(&file_key, &public).unwrap();

        let mut priv_body = decode_pem(PRIVATE_LABEL, &private).unwrap();
        let other = private::generate().unwrap();
        let other_body = decode_pem(PRIVATE_LABEL, &other).unwrap();
        priv_body[1..1 + 32].copy_from_slice(&other_body[1..1 + 32]);
        let mangled = encode_pem(PRIVATE_LABEL, &priv_body).unwrap();

        assert!(unwrap(&blob, &mangled).is_err());
    }

    #[test]
    fn wrong_ml_kem_key_fails() {
        // Same X25519 key, different ML-KEM half: rejected via the AEGIS tag.
        let (private, public) = keypair();
        let file_key = crate::aegis256::generate_key().unwrap();
        let blob = wrap(&file_key, &public).unwrap();

        let mut priv_body = decode_pem(PRIVATE_LABEL, &private).unwrap();
        let other = private::generate().unwrap();
        let other_body = decode_pem(PRIVATE_LABEL, &other).unwrap();
        priv_body[1 + 32..].copy_from_slice(&other_body[1 + 32..]);
        let mangled = encode_pem(PRIVATE_LABEL, &priv_body).unwrap();

        assert!(unwrap(&blob, &mangled).is_err());
    }

    #[test]
    fn swapped_ml_kem_ciphertext_fails() {
        // The binding property: ML-KEM is not key-binding, so a second, valid
        // ciphertext must not open the wrap once substituted for the real one.
        // Without `mk_ct` in the HKDF salt this test would pass a forgery.
        let (private, public) = keypair();
        let file_key = crate::aegis256::generate_key().unwrap();

        let blob = wrap(&file_key, &public).unwrap();
        let mut bytes = crate::base64::decode(&blob).unwrap();

        let container = public::from_str(&public).unwrap();
        let recipient_mk =
            MlKem768PublicKey::try_from(&container[1 + 32..]).unwrap();
        let (other_ct, _) = mlkem768::encapsulate(&recipient_mk, [9u8; SHARED_SECRET_SIZE]);

        let ct_start = 1 + 32;
        bytes[ct_start..ct_start + MK_CT_LEN].copy_from_slice(other_ct.as_ref());
        assert!(unwrap(&crate::base64::encode(bytes), &private).is_err());
    }

    #[test]
    fn decapsulation_never_panics_on_garbage() {
        // Implicit rejection: decaps returns a pseudorandom secret for a random
        // ciphertext instead of failing, so unwrap must surface a clean Err
        // (the AEGIS tag), never a panic.
        let (private, public) = keypair();
        let blob = wrap(b"file-key", &public).unwrap();
        let mut bytes = crate::base64::decode(&blob).unwrap();
        for b in bytes[1 + 32..1 + 32 + MK_CT_LEN].iter_mut() {
            *b ^= 0xff;
        }
        assert!(unwrap(&crate::base64::encode(bytes), &private).is_err());
    }

    #[test]
    fn ephemeral_is_unique_per_wrap() {
        let (_, public) = keypair();
        let a = crate::base64::decode(&wrap(b"same-key", &public).unwrap()).unwrap();
        let b = crate::base64::decode(&wrap(b"same-key", &public).unwrap()).unwrap();
        assert_ne!(a, b);
        // The ephemeral public key (bytes 1..33) differs every wrap.
        assert_ne!(a[1..1 + 32], b[1..1 + 32]);
    }

    #[test]
    fn tampered_blob_fails() {
        let (private, public) = keypair();
        let blob = wrap(b"file-key", &public).unwrap();
        let mut bytes = crate::base64::decode(&blob).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        assert!(unwrap(&crate::base64::encode(bytes), &private).is_err());
    }

    #[test]
    fn truncated_blob_fails_cleanly() {
        let (private, _) = keypair();
        let short = crate::base64::encode(vec![BLOB_VERSION; 64]);
        assert!(unwrap(&short, &private).is_err());
    }

    #[test]
    fn version_one_blob_rejected() {
        // The X25519-only format that never shipped. A `0x01` lead byte must be
        // refused, not decoded — accepting it could only be a downgrade.
        let (private, public) = keypair();
        let blob = wrap(b"file-key", &public).unwrap();
        let mut bytes = crate::base64::decode(&blob).unwrap();
        bytes[0] = 0x01;
        let err = unwrap(&crate::base64::encode(bytes), &private).unwrap_err();
        assert!(matches!(err, Error::KeyEncoding(_)));
    }

    #[test]
    fn cross_build_golden_vector() {
        // A wrap produced by one build must unwrap in another. Generating a live
        // WASM↔native exchange in a unit test is impractical, so this pins a
        // native-produced blob: the same source compiles to the portable WASM
        // path, and the suite is run once with NEON and once with the SIMD paths
        // disabled (LIBCRUX_DISABLE_SIMD128=1) to prove portable ≡ NEON on the
        // one value that would diverge if any half computed differently.
        let recovered = unwrap(GOLDEN_BLOB, GOLDEN_PRIVATE).unwrap();
        assert_eq!(recovered, b"golden-file-key-0123456789abcdef");
    }

    // A native-produced wrap of `b"golden-file-key-0123456789abcdef"`, pinned so
    // a build with different codegen (WASM's portable path, or NEON vs. the
    // SIMD-off portable path) is caught if it fails to reproduce the recovery.
    const GOLDEN_PRIVATE: &str = "-----BEGIN HOODIK WRAPPING PRIVATE KEY-----
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
-----END HOODIK WRAPPING PRIVATE KEY-----
";
    const GOLDEN_BLOB: &str = "Ak7I6/O7a2LHsF5eQANcI69/N2jr4W7j9OoacwOQCt8vHFB89MAbe4q9Da9+kQ1hQYnL0in09uTK6Qi769p8jaNsnuxGfeR8n2wps0YrcdxJF/bX6+9luWsn1OBO1vcVQx8g9uRt5etWyi9BBI70APYXA2asHxShd+aINVmHdfxOo0Bt/P4T+rozTuQuQ/l3U/xKSC8TPK2b/T3xNz0xM3yUKMlTCUJpBKnNsc5FCU5bD6y3i77jkP5g2Ot+nKu3CWWgxGU/7xIPB7e5y+YZharoMwAo6PQj+d6k4OV1BA/kZN9QDG4i4IIkL1plQJRZ5LmA6nPoOglPNo2MsPtDqS3JzjrORcoPMs/du7IrqVXl7eJf9z5re44OmEiEtFae2x7QXI38Q9+ud7er/spmPCi+JQtWwRsU0WGBw/3TAZ6/NJy4dIbt446zus3yKIyksmHsktOYiC4/gUot9OYZ4S3p3E+OFI0LdczPLzPz7QF+W2bBsqYcnyIEs8pv3ofBI84I0sO1uO61Bos72A4oRTvtz8FZhY6CygrbvRLFbXgEMakhHmrF9TEnMF3iLRmM436BGQ8jrim/Mv/C+BhA1ODoOjfTr8342VfBBg6YSL6ZCoIxhJPHe1GYtzwUwrV/CGk0icT2yX4OpyWWJWLCxZcNIfOpIAfwWQyI/zctJIo4mvbcfzrP86uQa0rnZvyCZnzEwikO5aSdYsrU+N/lSvzN48HJo2FblV9uaUkkYv+Vc7LRthAbSrQmNUKz5K+tLOqpARlv3kSZ84pFaRyZomabJbczwQ9E3Dt2BZCOswcBdi7bbM3TlirazBWmUCPGmbwXIpP387mlFqzy252V+vXJDm5I0u8iobqme1yQey+DAkIQQFovCJo4SH6TI+jUFtYI7MLN2gZR2vsUPNUfRDypfu8Wpkbpqs+6v/mVVCMv1RzGTJF30xsb/xu24dLHWrJZ5zl5NeSHoofIFxnlJOJ/MZOxl5vdi817jojhmA7W4mM2NbCI0AnirbzXQiXh9AnBCuvtjtdfVQod7bxugAGnOM16N0wgytPrYDwhjdYcix5FaCmtFWQR6ZJBD1NudVyVL/0UVHxOzqr4yzUotWq1wwb10RrgLRF6YDBIl12wCCGLnMlKpP3ZWOw8dryv4mfQd+iMPC8FKcdR1sqSHDlT9YiHdU5OminpOSuOOQ0kedhXClKRq98dkk0nQKjyZnvwHgi9CyiGtCFGe9DMatNsixyM90I2tr/pXWXixsN6ViCUjOmRZ5cBwZuBmYAk3o/w0M5wxgXKS8CQ+oWGzyp9jt7zXdkI/hCoB6pFH41JlfhAyEK4cQWMa9eiMRocfOK3hrT/lz8w8QYiz49C87OBQQxKXgmztm5as8tO2C9mztqOlafGchwcP/3B5SnrQL4WUcLlA3ZRNTc/99fvsmPBYaZ4cjQeEV0DEQ9aPnixFl7FJ66KGSpIovb1Hwlr/x2M9su4LT3JIwRcln3gH5Qc9BsPoDyLN68Gadm18YB8CDaEzAl4JVbBOlcl9rP9jAUqofVgF9unb6P9zER/AfMcAWivDPl1b+TJZbcS70od6VBqW/8SGL6ruj6HG7uI7w==";
}
