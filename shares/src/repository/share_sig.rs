//! Signature + role-string helpers for the share path (`share.rs`).
//!
//! Every signed canonical here is re-encoded from authoritative state and
//! verified against the *signer's* pubkey — the wire never supplies the
//! bytes that get verified.

use cryptfns::asn1::{
    encode_member_sig_v1, MemberSigPayloadV1, ShareRoleEnum, MEMBER_SIG_V1_PREFIX,
};
use error::{AppResult, Error};

pub(crate) fn role_enum_to_str(role: ShareRoleEnum) -> &'static str {
    match role {
        ShareRoleEnum::Reader => "reader",
        ShareRoleEnum::Editor => "editor",
        ShareRoleEnum::CoOwner => "co-owner",
    }
}

pub(crate) fn role_str_to_enum(role: &str) -> Option<ShareRoleEnum> {
    match role {
        "reader" => Some(ShareRoleEnum::Reader),
        "editor" => Some(ShareRoleEnum::Editor),
        "co-owner" => Some(ShareRoleEnum::CoOwner),
        _ => None,
    }
}

/// `&'static str` variant for the audit-log columns, which want a borrow
/// with the program's lifetime rather than an owned string.
pub(crate) fn static_role_str(role: &str) -> Option<&'static str> {
    match role {
        "reader" => Some("reader"),
        "editor" => Some("editor"),
        "co-owner" => Some("co-owner"),
        _ => None,
    }
}

/// Decode a 64-hex-char fingerprint column into its 32 raw bytes. The
/// value comes from the trusted `users.fingerprint` column, so a decode
/// failure is an internal-consistency error, not a client mistake.
pub(crate) fn fingerprint_bytes(fingerprint: &str) -> AppResult<[u8; 32]> {
    let bytes = cryptfns::hex::decode(fingerprint)
        .map_err(|_| Error::InternalError("fingerprint_not_hex".to_string()))?;
    if bytes.len() != 32 {
        return Err(Error::InternalError("fingerprint_wrong_length".to_string()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Re-encode the `MemberSigPayloadV1` canonical from the recipient's
/// authoritative pubkey/fingerprint/role plus the supplied `signed_at`,
/// then RSA-verify `signature_b64` against the granter's pubkey. Returns
/// the decoded raw signature bytes for persistence in
/// `user_files.member_signature`.
pub(crate) fn verify_member_signature(
    granter_pubkey: &str,
    signature_b64: &str,
    recipient_id: entity::Uuid,
    recipient_pubkey: &str,
    recipient_fingerprint: [u8; 32],
    share_role: ShareRoleEnum,
    signed_at: i64,
) -> AppResult<Vec<u8>> {
    let pubkey_der = cryptfns::rsa::public::to_pkcs1_der(recipient_pubkey)
        .map_err(|_| Error::BadRequest("recipient_pubkey_invalid".to_string()))?;
    let payload = MemberSigPayloadV1 {
        user_id: recipient_id.into_bytes(),
        pubkey_der,
        fingerprint: recipient_fingerprint,
        share_role,
        signed_at,
    };
    let der = encode_member_sig_v1(&payload).map_err(|e| Error::CryptoError(Box::new(e)))?;
    let mut signing_input = Vec::with_capacity(MEMBER_SIG_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(MEMBER_SIG_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    cryptfns::rsa::public::verify_bytes(&signing_input, signature_b64, granter_pubkey)
        .map_err(|_| Error::BadRequest("member_signature_invalid".to_string()))?;
    cryptfns::base64::decode(signature_b64)
        .map_err(|_| Error::BadRequest("member_signature_invalid_base64".to_string()))
}
