#![allow(unexpected_cfgs)]
use crate::aes;
use crate::asn1;
use crate::chacha;
use crate::rsa;
use crate::rsa::PublicKeyParts;
use std::str::FromStr;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn crc16_digest(input: Vec<u8>) -> String {
    crate::utils::set_panic_hook();

    crate::crc::crc16_digest(input.as_slice())
}

#[wasm_bindgen]
pub fn sha256_digest(input: Vec<u8>) -> String {
    crate::utils::set_panic_hook();

    crate::sha256::digest(input.as_slice())
}

#[wasm_bindgen]
pub fn rsa_generate_private() -> Option<String> {
    crate::utils::set_panic_hook();

    let private = rsa::private::generate().ok()?;

    rsa::private::to_string(&private).ok()
}

#[wasm_bindgen]
pub fn rsa_private_key_size(private_key: String) -> Option<usize> {
    crate::utils::set_panic_hook();

    let private = rsa::private::from_str(&private_key).ok()?;

    Some(private.size() * 8)
}

#[wasm_bindgen]
pub fn rsa_public_key_size(public_key: String) -> Option<usize> {
    crate::utils::set_panic_hook();

    let public = rsa::public::from_str(&public_key).ok()?;

    Some(public.size() * 8)
}

#[wasm_bindgen]
pub fn rsa_public_from_private(private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    let private = rsa::private::from_str(&private_key).ok()?;
    let public = rsa::public::from_private(&private).ok()?;

    rsa::public::to_string(&public).ok()
}

#[wasm_bindgen]
pub fn rsa_sign(message: String, private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::private::sign(&message, &private_key).ok()
}

#[wasm_bindgen]
pub fn rsa_verify(message: String, signature: String, public_key: String) -> bool {
    crate::utils::set_panic_hook();

    rsa::public::verify(&message, &signature, &public_key).is_ok()
}

/// Sign raw bytes — sharing's signed payloads are DER blobs that are not
/// valid UTF-8 and cannot round-trip through `String`.
#[wasm_bindgen]
pub fn rsa_sign_bytes(message: Vec<u8>, private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::private::sign_bytes(&message, &private_key).ok()
}

/// Verify a signature over raw bytes.
#[wasm_bindgen]
pub fn rsa_verify_bytes(message: Vec<u8>, signature: String, public_key: String) -> bool {
    crate::utils::set_panic_hook();

    rsa::public::verify_bytes(&message, &signature, &public_key).is_ok()
}

#[wasm_bindgen]
pub fn rsa_encrypt(message: String, public_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::public::encrypt(&message, &public_key).ok()
}

#[wasm_bindgen]
pub fn rsa_decrypt(message: String, private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::private::decrypt(&message, &private_key).ok()
}

#[wasm_bindgen]
pub fn rsa_fingerprint_public(public_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::fingerprint(rsa::public::from_str(&public_key).ok()?).ok()
}

#[wasm_bindgen]
pub fn rsa_fingerprint_private(private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    rsa::fingerprint(rsa::private::from_str(&private_key).ok()?).ok()
}

#[wasm_bindgen]
pub fn x25519_generate_private() -> Option<String> {
    crate::utils::set_panic_hook();

    crate::ecdh::private::generate().ok()
}

#[wasm_bindgen]
pub fn x25519_public_from_private(private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    crate::ecdh::public::from_private(&private_key).ok()
}

/// Wrap a file key for an X25519 recipient; returns the base64 ECIES blob.
#[wasm_bindgen]
pub fn x25519_wrap(file_key: Vec<u8>, recipient_public_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    crate::ecdh::wrap(&file_key, &recipient_public_key).ok()
}

/// Unwrap a base64 ECIES blob with the recipient's X25519 private key.
#[wasm_bindgen]
pub fn x25519_unwrap(blob: String, private_key: String) -> Option<Vec<u8>> {
    crate::utils::set_panic_hook();

    crate::ecdh::unwrap(&blob, &private_key).ok()
}

#[wasm_bindgen]
pub fn ed25519_generate_private() -> Option<String> {
    crate::utils::set_panic_hook();

    crate::ed25519::private::generate().ok()
}

#[wasm_bindgen]
pub fn ed25519_public_from_private(private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    crate::ed25519::public::from_private(&private_key).ok()
}

#[wasm_bindgen]
pub fn ed25519_sign(message: String, private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    crate::ed25519::private::sign(&message, &private_key).ok()
}

#[wasm_bindgen]
pub fn ed25519_verify(message: String, signature: String, public_key: String) -> bool {
    crate::utils::set_panic_hook();

    crate::ed25519::public::verify(&message, &signature, &public_key).is_ok()
}

/// Sign raw bytes — signed payloads are DER blobs that are not valid UTF-8.
#[wasm_bindgen]
pub fn ed25519_sign_bytes(message: Vec<u8>, private_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    crate::ed25519::private::sign_bytes(&message, &private_key).ok()
}

/// Verify an Ed25519 signature over raw bytes.
#[wasm_bindgen]
pub fn ed25519_verify_bytes(message: Vec<u8>, signature: String, public_key: String) -> bool {
    crate::utils::set_panic_hook();

    crate::ed25519::public::verify_bytes(&message, &signature, &public_key).is_ok()
}

/// Key-type-agnostic fingerprint of any SPKI PEM public key.
#[wasm_bindgen]
pub fn spki_fingerprint(public_key: String) -> Option<String> {
    crate::utils::set_panic_hook();

    crate::spki::fingerprint(&public_key).ok()
}

/// OPAQUE client registration step 1. Returns JSON `{state, message}`.
#[wasm_bindgen]
pub fn opaque_client_registration_start(password: String) -> Option<String> {
    crate::utils::set_panic_hook();

    let result = crate::opaque::client_registration_start(password.as_bytes()).ok()?;
    serde_json::to_string(&result).ok()
}

/// OPAQUE client registration step 2. Returns JSON `{message, export_key}`.
#[wasm_bindgen]
pub fn opaque_client_registration_finish(
    registration_state: String,
    registration_response: String,
    password: String,
) -> Option<String> {
    crate::utils::set_panic_hook();

    let result = crate::opaque::client_registration_finish(
        &registration_state,
        &registration_response,
        password.as_bytes(),
    )
    .ok()?;
    serde_json::to_string(&result).ok()
}

/// OPAQUE client login step 1. Returns JSON `{state, message}`.
#[wasm_bindgen]
pub fn opaque_client_login_start(password: String) -> Option<String> {
    crate::utils::set_panic_hook();

    let result = crate::opaque::client_login_start(password.as_bytes()).ok()?;
    serde_json::to_string(&result).ok()
}

/// OPAQUE client login step 2. Returns JSON `{finalization, session_key, export_key}`.
#[wasm_bindgen]
pub fn opaque_client_login_finish(
    login_state: String,
    credential_response: String,
    password: String,
) -> Option<String> {
    crate::utils::set_panic_hook();

    let result = crate::opaque::client_login_finish(
        &login_state,
        &credential_response,
        password.as_bytes(),
    )
    .ok()?;
    serde_json::to_string(&result).ok()
}

/// Derive the private-key envelope KEK from an OPAQUE `export_key`.
#[wasm_bindgen]
pub fn envelope_derive_kek(export_key: Vec<u8>) -> Option<Vec<u8>> {
    crate::utils::set_panic_hook();

    crate::envelope::derive_kek(&export_key).ok().map(|k| k.to_vec())
}

/// Seal a private-key bundle under `kek`; returns the base64 envelope.
#[wasm_bindgen]
pub fn envelope_seal(kek: Vec<u8>, bundle: Vec<u8>) -> Option<String> {
    crate::utils::set_panic_hook();

    crate::envelope::seal(&to_kek(&kek)?, &bundle).ok()
}

/// Open a base64 envelope with `kek`.
#[wasm_bindgen]
pub fn envelope_open(kek: Vec<u8>, envelope: String) -> Option<Vec<u8>> {
    crate::utils::set_panic_hook();

    crate::envelope::open(&to_kek(&kek)?, &envelope).ok()
}

/// Re-wrap an envelope's data key from `old_kek` to `new_kek` — the cheap
/// half of a password change.
#[wasm_bindgen]
pub fn envelope_rewrap(old_kek: Vec<u8>, new_kek: Vec<u8>, envelope: String) -> Option<String> {
    crate::utils::set_panic_hook();

    crate::envelope::rewrap(&to_kek(&old_kek)?, &to_kek(&new_kek)?, &envelope).ok()
}

fn to_kek(bytes: &[u8]) -> Option<[u8; 32]> {
    bytes.try_into().ok()
}

/// Sign a key-transition certificate with the old and new identity keys.
/// Returns JSON `{old_signature, new_signature}`.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn transition_sign(
    user_id: Vec<u8>,
    old_key_type: String,
    old_key_pem: String,
    old_fingerprint: String,
    new_identity_key_pem: String,
    new_wrapping_key_pem: String,
    new_fingerprint: String,
    issued_at: i64,
    old_private_key: String,
    new_identity_private_key: String,
) -> Option<String> {
    crate::utils::set_panic_hook();

    let signatures = crate::transition::sign_certificate(
        &user_id,
        &old_key_type,
        &old_key_pem,
        &old_fingerprint,
        &new_identity_key_pem,
        &new_wrapping_key_pem,
        &new_fingerprint,
        issued_at,
        &old_private_key,
        &new_identity_private_key,
    )
    .ok()?;
    serde_json::to_string(&signatures).ok()
}

#[wasm_bindgen]
pub fn aes_generate_key() -> Option<Vec<u8>> {
    aes::generate_key().ok()
}

#[wasm_bindgen]
pub fn aes_encrypt(key: Vec<u8>, plaintext: Vec<u8>) -> Option<Vec<u8>> {
    aes::encrypt(key, plaintext).ok()
}

#[wasm_bindgen]
pub fn aes_decrypt(key: Vec<u8>, ciphertext: Vec<u8>) -> Option<Vec<u8>> {
    aes::decrypt(key, ciphertext).ok()
}

#[wasm_bindgen]
pub fn chacha_generate_key() -> Option<Vec<u8>> {
    chacha::generate_key().ok()
}

#[wasm_bindgen]
pub fn chacha_encrypt(key: Vec<u8>, plaintext: Vec<u8>) -> Option<Vec<u8>> {
    chacha::encrypt(key, plaintext).ok()
}

#[wasm_bindgen]
pub fn chacha_decrypt(key: Vec<u8>, ciphertext: Vec<u8>) -> Option<Vec<u8>> {
    chacha::decrypt(key, ciphertext).ok()
}

/// Generate a key for the given cipher identifier (e.g. `"ascon128a"`, `"chacha20poly1305"`).
#[wasm_bindgen]
pub fn cipher_generate_key(cipher: &str) -> Option<Vec<u8>> {
    crate::cipher::Cipher::from_str(cipher).ok()?.generate_key().ok()
}

/// Encrypt `plaintext` with `key` using the named cipher.
#[wasm_bindgen]
pub fn cipher_encrypt(cipher: &str, key: Vec<u8>, plaintext: Vec<u8>) -> Option<Vec<u8>> {
    crate::cipher::Cipher::from_str(cipher).ok()?.encrypt(key, plaintext).ok()
}

/// Decrypt `ciphertext` with `key` using the named cipher.
#[wasm_bindgen]
pub fn cipher_decrypt(cipher: &str, key: Vec<u8>, ciphertext: Vec<u8>) -> Option<Vec<u8>> {
    crate::cipher::Cipher::from_str(cipher).ok()?.decrypt(key, ciphertext).ok()
}

#[cfg(feature = "tokenizer")]
#[wasm_bindgen]
pub fn text_into_tokens(input: &str) -> Option<String> {
    crate::tokenizer::into_tokens(input)
        .ok()
        .map(crate::tokenizer::into_string)
}

#[cfg(feature = "tokenizer")]
#[wasm_bindgen]
pub fn text_into_hashed_tokens(input: &str) -> Option<String> {
    crate::tokenizer::into_hashed_tokens(input)
        .ok()
        .map(crate::tokenizer::into_string)
}

fn share_role_from_u8(v: u8) -> Option<asn1::ShareRoleEnum> {
    match v {
        0 => Some(asn1::ShareRoleEnum::Reader),
        1 => Some(asn1::ShareRoleEnum::Editor),
        2 => Some(asn1::ShareRoleEnum::CoOwner),
        _ => None,
    }
}

fn bytes_to_array_wasm<const N: usize>(input: &[u8]) -> Option<[u8; N]> {
    if input.len() != N {
        return None;
    }
    let mut out = [0u8; N];
    out.copy_from_slice(input);
    Some(out)
}

/// DER-encode `ShareRequestPayloadV1`. Returns `None` on invalid input
/// (wrong array length, unknown share-role discriminant, …).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)] // wasm-bindgen surface — flat primitives only
pub fn share_payload_encode_v1(
    sender_id: Vec<u8>,
    recipient_id: Vec<u8>,
    recipient_pubkey_fingerprint: Vec<u8>,
    share_role: u8,
    root_file_id: Vec<u8>,
    entries_hash: Vec<u8>,
    timestamp: i64,
    nonce: Vec<u8>,
) -> Option<Vec<u8>> {
    crate::utils::set_panic_hook();

    let payload = asn1::ShareRequestPayloadV1 {
        sender_id: bytes_to_array_wasm::<16>(&sender_id)?,
        recipient_id: bytes_to_array_wasm::<16>(&recipient_id)?,
        recipient_pubkey_fingerprint: bytes_to_array_wasm::<32>(&recipient_pubkey_fingerprint)?,
        share_role: share_role_from_u8(share_role)?,
        root_file_id: bytes_to_array_wasm::<16>(&root_file_id)?,
        entries_hash: bytes_to_array_wasm::<32>(&entries_hash)?,
        timestamp,
        nonce: bytes_to_array_wasm::<16>(&nonce)?,
    };
    asn1::encode_share_request_v1(&payload).ok()
}

/// DER-encode `MemberSigPayloadV1`.
#[wasm_bindgen]
pub fn member_sig_encode_v1(
    user_id: Vec<u8>,
    pubkey_der: Vec<u8>,
    fingerprint: Vec<u8>,
    share_role: u8,
    signed_at: i64,
) -> Option<Vec<u8>> {
    crate::utils::set_panic_hook();

    let payload = asn1::MemberSigPayloadV1 {
        user_id: bytes_to_array_wasm::<16>(&user_id)?,
        pubkey_der,
        fingerprint: bytes_to_array_wasm::<32>(&fingerprint)?,
        share_role: share_role_from_u8(share_role)?,
        signed_at,
    };
    asn1::encode_member_sig_v1(&payload).ok()
}

/// DER-encode `AuditEventRowV1`. `share_role` is the wire byte (0–2) or
/// `255` to encode the row without a role (e.g. revoke events).
#[wasm_bindgen]
pub fn audit_event_encode_v1(
    sender_id: Vec<u8>,
    recipient_id: Vec<u8>,
    file_id: Vec<u8>,
    action: String,
    share_role: u8,
    created_at: i64,
) -> Option<Vec<u8>> {
    crate::utils::set_panic_hook();

    let share_role = match share_role {
        255 => None,
        v => Some(share_role_from_u8(v)?),
    };
    let row = asn1::AuditEventRowV1 {
        sender_id: bytes_to_array_wasm::<16>(&sender_id)?,
        recipient_id: bytes_to_array_wasm::<16>(&recipient_id)?,
        file_id: bytes_to_array_wasm::<16>(&file_id)?,
        action,
        share_role,
        created_at,
    };
    asn1::encode_audit_event_v1(&row).ok()
}

fn audit_action_from_u8(v: u8) -> Option<asn1::AuditEventActionEnum> {
    use asn1::AuditEventActionEnum::*;
    match v {
        0 => Some(Grant),
        1 => Some(Revoke),
        2 => Some(RoleChange),
        3 => Some(SharedFolderUpload),
        4 => Some(Fork),
        5 => Some(SharedByCoOwner),
        6 => Some(SharedFolderEdit),
        7 => Some(SharedFolderRestore),
        8 => Some(SharedFolderEvict),
        9 => Some(SharedFolderMoveOut),
        _ => None,
    }
}

/// DER-encode `AuditEventSigInputV1`. The two role fields and the
/// `recipient_id` accept the wire byte `255` as a sentinel for "absent",
/// matching `audit_event_encode_v1`'s convention. The `recipient_id`
/// sentinel is an empty `Vec` because `255` is a legal byte value inside
/// a UUID.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn audit_event_sig_input_encode_v1(
    sender_id: Vec<u8>,
    recipient_id: Vec<u8>,
    file_id: Vec<u8>,
    action: u8,
    share_role_before: u8,
    share_role_after: u8,
    timestamp: i64,
) -> Option<Vec<u8>> {
    crate::utils::set_panic_hook();

    let recipient_id = if recipient_id.is_empty() {
        None
    } else {
        Some(bytes_to_array_wasm::<16>(&recipient_id)?)
    };
    let share_role_before = match share_role_before {
        255 => None,
        v => Some(share_role_from_u8(v)?),
    };
    let share_role_after = match share_role_after {
        255 => None,
        v => Some(share_role_from_u8(v)?),
    };

    let payload = asn1::AuditEventSigInputV1 {
        sender_id: bytes_to_array_wasm::<16>(&sender_id)?,
        recipient_id,
        file_id: bytes_to_array_wasm::<16>(&file_id)?,
        action: audit_action_from_u8(action)?,
        share_role_before,
        share_role_after,
        timestamp,
    };
    asn1::encode_audit_event_sig_input_v1(&payload).ok()
}

/// DER-encode a v1 folder member list. Members travel as flat parallel
/// arrays: `user_ids` and `signed_by_user_ids` concatenate every 16-byte
/// UUID, `pubkey_fingerprints` concatenates every 32-byte SHA-256, and
/// `share_roles` / `is_owner_flags` carry one byte per member. All
/// arrays must contain the same number of entries; the encoder sorts
/// on `user_id` so callers can pass members in any order.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn folder_member_list_encode_v1(
    folder_id: Vec<u8>,
    folder_owner_id: Vec<u8>,
    user_ids: Vec<u8>,
    pubkey_fingerprints: Vec<u8>,
    share_roles: Vec<u8>,
    is_owner_flags: Vec<u8>,
    signed_by_user_ids: Vec<u8>,
    members_signed_at: i64,
) -> Option<Vec<u8>> {
    crate::utils::set_panic_hook();

    let count = share_roles.len();
    if is_owner_flags.len() != count
        || user_ids.len() != count * 16
        || signed_by_user_ids.len() != count * 16
        || pubkey_fingerprints.len() != count * 32
    {
        return None;
    }

    let mut members = Vec::with_capacity(count);
    for i in 0..count {
        let user_id = bytes_to_array_wasm::<16>(&user_ids[i * 16..(i + 1) * 16])?;
        let signed_by_user_id =
            bytes_to_array_wasm::<16>(&signed_by_user_ids[i * 16..(i + 1) * 16])?;
        let pubkey_fingerprint =
            bytes_to_array_wasm::<32>(&pubkey_fingerprints[i * 32..(i + 1) * 32])?;
        members.push(asn1::FolderListMember {
            user_id,
            pubkey_fingerprint,
            share_role: share_role_from_u8(share_roles[i])?,
            is_owner: is_owner_flags[i] != 0,
            signed_by_user_id,
        });
    }

    let payload = asn1::FolderMemberListV1 {
        folder_id: bytes_to_array_wasm::<16>(&folder_id)?,
        folder_owner_id: bytes_to_array_wasm::<16>(&folder_owner_id)?,
        members,
        members_signed_at,
    };
    asn1::encode_folder_member_list_v1(&payload).ok()
}

/// DER-encode the canonical entries list that `entries_hash` commits to.
/// Inputs are flat parallel arrays — `file_ids` is the concatenation of
/// every 16-byte file UUID, `encrypted_keys_flat` is the concatenation of
/// every per-entry ciphertext, and `encrypted_key_lengths` carries the
/// length of each ciphertext so the host can slice the flat buffer.
///
/// The encoder sorts by `file_id` exactly like the Rust side, so callers
/// hash `sha256(output)` to derive `entries_hash`.
#[wasm_bindgen]
pub fn entries_encode_v1(
    file_ids: Vec<u8>,
    encrypted_keys_flat: Vec<u8>,
    encrypted_key_lengths: Vec<u32>,
) -> Option<Vec<u8>> {
    crate::utils::set_panic_hook();

    let count = encrypted_key_lengths.len();
    if file_ids.len() != count * 16 {
        return None;
    }

    let expected_total: usize = encrypted_key_lengths.iter().map(|&n| n as usize).sum();
    if encrypted_keys_flat.len() != expected_total {
        return None;
    }

    let mut entries = Vec::with_capacity(count);
    let mut key_cursor = 0usize;
    for (i, &len) in encrypted_key_lengths.iter().enumerate() {
        let len = len as usize;
        let file_id = bytes_to_array_wasm::<16>(&file_ids[i * 16..(i + 1) * 16])?;
        let encrypted_key = encrypted_keys_flat[key_cursor..key_cursor + len].to_vec();
        key_cursor += len;
        entries.push(asn1::ShareEntry {
            file_id,
            encrypted_key,
        });
    }

    asn1::encode_entries_v1(&entries).ok()
}
