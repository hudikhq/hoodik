//! Shared helpers for the four `shares_*.rs` integration suites. Provides:
//!
//! - DTOs/structs for the registered test user + envelope building
//! - `register_account!` / `create_file!` / `post_share!` /
//!   `delete_share!` macros that drive the real HTTP routes
//!
//! The macros avoid the awkward generic trait-bound dance `init_service`
//! requires when an async helper takes the app by reference.
#![allow(dead_code)]

use actix_web::dev::{Service, ServiceResponse};
use actix_web::test;
use auth::data::create_user::CreateUser;
use cryptfns::asn1::{
    encode_audit_event_sig_input_v1, encode_entries_v1, encode_folder_member_list_v1,
    encode_member_sig_v1, encode_share_request_v1, AuditEventActionEnum, AuditEventSigInputV1,
    FolderListMember, FolderMemberListV1, MemberSigPayloadV1, ShareEntry, ShareRequestPayloadV1,
    ShareRoleEnum, AUDIT_EVENT_SIG_V1_PREFIX, FOLDER_LIST_V1_PREFIX, MEMBER_SIG_V1_PREFIX,
    SHARE_REQUEST_V1_PREFIX,
};
use cryptfns::identity::KeyType;
use entity::Uuid;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[path = "./helpers.rs"]
mod helpers;

pub fn extract_cookies(
    headers: &actix_web::http::header::HeaderMap,
) -> (
    Option<actix_web::cookie::Cookie<'static>>,
    Option<actix_web::cookie::Cookie<'static>>,
) {
    helpers::extract_cookies(headers)
}

pub struct TestUser {
    pub email: String,
    pub user_id: Uuid,
    pub private_pem: String,
    pub public_pem: String,
    pub fingerprint: String,
    pub key_type: KeyType,
    /// X25519 wrapping keypair — curve25519 accounts only.
    pub wrapping_private_pem: Option<String>,
    pub wrapping_public_pem: Option<String>,
    pub jwt: actix_web::cookie::Cookie<'static>,
}

impl TestUser {
    pub fn fingerprint_bytes(&self) -> [u8; 32] {
        let bytes = cryptfns::hex::decode(&self.fingerprint).expect("fingerprint hex");
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        out
    }

    /// Sign with the account's identity key, mirroring the client:
    /// RSA for legacy accounts, Ed25519 for curve25519 accounts.
    pub fn sign_bytes(&self, message: &[u8]) -> String {
        match self.key_type {
            KeyType::Rsa => cryptfns::rsa::private::sign_bytes(message, &self.private_pem),
            KeyType::Curve25519 => cryptfns::ed25519::private::sign_bytes(message, &self.private_pem),
        }
        .expect("sign with identity key")
    }

    /// Wrap a file key under the account's own key material: RSA encrypt
    /// for legacy accounts, hybrid X25519 + ML-KEM wrap for curve25519 accounts.
    pub fn wrap_key(&self, key: &str) -> String {
        match self.key_type {
            KeyType::Rsa => cryptfns::rsa::public::encrypt(key, &self.public_pem),
            KeyType::Curve25519 => cryptfns::ecdh::wrap(
                key.as_bytes(),
                self.wrapping_public_pem
                    .as_ref()
                    .expect("curve25519 fixture carries a wrapping key"),
            ),
        }
        .expect("wrap file key")
    }
}

pub trait TestApp:
    Service<
    actix_http::Request,
    Response = ServiceResponse<Self::Body>,
    Error = actix_web::Error,
>
{
    type Body: actix_web::body::MessageBody;
}
impl<S, B> TestApp for S
where
    B: actix_web::body::MessageBody,
    S: Service<
        actix_http::Request,
        Response = ServiceResponse<B>,
        Error = actix_web::Error,
    >,
{
    type Body = B;
}

const CURVE_TEST_PASSWORD: &[u8] = b"not-4-weak-password-for-god-sakes!";

/// Register a born-migrated curve25519 account through the real OPAQUE
/// registration handshake, returning the fixture the shares suites drive.
pub async fn register_curve25519(app: &impl TestApp, email: &str) -> TestUser {
    let (private_pem, public_pem, fingerprint, wrapping_private_pem, wrapping_public_pem) =
        generate_curve25519_keypair();

    let reg_start = cryptfns::opaque::client_registration_start(CURVE_TEST_PASSWORD).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/register/pake/start")
        .set_json(serde_json::json!({ "email": email, "registration_request": reg_start.message }))
        .to_request();
    let body: Value = test::read_body_json(test::call_service(app, req).await).await;
    let reg_response = body["registration_response"].as_str().unwrap();
    let reg_finish = cryptfns::opaque::client_registration_finish(
        &reg_start.state,
        reg_response,
        CURVE_TEST_PASSWORD,
    )
    .unwrap();

    let export_key = cryptfns::base64::decode(&reg_finish.export_key).unwrap();
    let kek = cryptfns::envelope::derive_kek(&export_key).unwrap();
    let envelope = cryptfns::envelope::seal(
        &kek,
        format!("v1|ed:{private_pem}|x:{wrapping_private_pem}").as_bytes(),
    )
    .unwrap();

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(serde_json::json!({
            "email": email,
            "pubkey": public_pem,
            "wrapping_pubkey": wrapping_public_pem,
            "fingerprint": fingerprint,
            "key_type": "curve25519",
            "encrypted_private_key": envelope,
            "opaque_registration_upload": reg_finish.message,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert!(resp.status().is_success(), "register {email} failed: {:?}", resp.status());
    let (jwt, _) = extract_cookies(resp.headers());
    let jwt = jwt.expect("register response missing JWT cookie");
    let body = test::read_body(resp).await;
    let authenticated: auth::data::authenticated::Authenticated =
        serde_json::from_slice(&body).expect("authenticated json");

    TestUser {
        email: email.to_string(),
        user_id: authenticated.user.id,
        private_pem,
        public_pem,
        fingerprint,
        key_type: KeyType::Curve25519,
        wrapping_private_pem: Some(wrapping_private_pem),
        wrapping_public_pem: Some(wrapping_public_pem),
        jwt,
    }
}

/// Seed a legacy RSA `TestUser`: insert the account at the data layer, then log
/// it in through the credentials endpoint for a session cookie. The RSA private
/// key is retained so the fixture can sign and wrap exactly as a pre-migration
/// client would.
pub async fn seed_legacy_test_user(app: &impl TestApp, db: &entity::DbConn, email: &str) -> TestUser {
    let seeded = helpers::seed_legacy_user(db, email).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(serde_json::json!({ "email": email, "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert!(resp.status().is_success(), "login {email} failed: {:?}", resp.status());
    let (jwt, _) = extract_cookies(resp.headers());
    let jwt = jwt.expect("login response missing JWT cookie");

    TestUser {
        email: email.to_string(),
        user_id: seeded.user_id,
        private_pem: seeded.rsa_private,
        public_pem: seeded.rsa_public,
        fingerprint: seeded.rsa_fingerprint,
        key_type: KeyType::Rsa,
        wrapping_private_pem: None,
        wrapping_public_pem: None,
        jwt,
    }
}

pub fn make_create_user(email: &str, public_pem: &str, fingerprint: &str) -> CreateUser {
    CreateUser {
        email: Some(email.to_string()),
        password: Some("not-4-weak-password-for-god-sakes!".to_string()),
        secret: None,
        token: None,
        pubkey: Some(public_pem.to_string()),
        fingerprint: Some(fingerprint.to_string()),
        encrypted_private_key: Some("encrypted-private-key-blob".to_string()),
        key_type: None,
        wrapping_pubkey: None,
        opaque_registration_upload: None,
        invitation_id: None,
    }
}

pub fn make_create_curve25519_user(
    email: &str,
    public_pem: &str,
    fingerprint: &str,
    wrapping_pubkey: &str,
) -> CreateUser {
    let mut user = make_create_user(email, public_pem, fingerprint);
    user.key_type = Some("curve25519".to_string());
    user.wrapping_pubkey = Some(wrapping_pubkey.to_string());
    user
}

pub fn make_create_file(public_pem: &str, name_hash: &str) -> storage::data::create_file::CreateFile {
    make_create_file_with_key(
        cryptfns::rsa::public::encrypt("deadbeef", public_pem).unwrap(),
        name_hash,
    )
}

pub fn make_create_file_for(user: &TestUser, name_hash: &str) -> storage::data::create_file::CreateFile {
    make_create_file_with_key(user.wrap_key("deadbeef"), name_hash)
}

fn make_create_file_with_key(
    encrypted_key: String,
    name_hash: &str,
) -> storage::data::create_file::CreateFile {
    storage::data::create_file::CreateFile {
        encrypted_key: Some(encrypted_key),
        encrypted_name: Some("encrypted-name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(name_hash.to_string()),
        mime: Some("text/plain".to_string()),
        size: Some(1024),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: Some("md5".to_string()),
        sha1: Some("sha1".to_string()),
        sha256: Some("sha256".to_string()),
        blake2b: Some("b2b".to_string()),
        cipher: None,
        editable: None,
    }
}

pub fn make_create_folder(
    user: &TestUser,
    name_hash: &str,
    parent_id: Option<Uuid>,
) -> storage::data::create_file::CreateFile {
    storage::data::create_file::CreateFile {
        encrypted_key: Some(user.wrap_key("deadbeef")),
        encrypted_name: Some("encrypted-name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: None,
        name_hash: Some(name_hash.to_string()),
        mime: Some("dir".to_string()),
        size: None,
        chunks: None,
        file_id: parent_id.map(|u| u.to_string()),
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        cipher: None,
        editable: None,
    }
}

pub fn make_create_child_file(
    user: &TestUser,
    name_hash: &str,
    parent_id: Uuid,
) -> storage::data::create_file::CreateFile {
    let mut file = make_create_file_for(user, name_hash);
    file.file_id = Some(parent_id.to_string());
    file
}

pub struct ShareEnvelopeInputs<'a> {
    pub sender: &'a TestUser,
    pub recipient: &'a TestUser,
    pub role: ShareRoleEnum,
    pub root_file_id: Uuid,
    pub entries: Vec<(Uuid, Vec<u8>)>,
    pub nonce: [u8; 16],
    pub timestamp: i64,
}

/// One prospective member entry, ready to roll into a
/// `FolderMemberListV1`. Tests build these directly so the helper can
/// stay generic across folder-share / revoke / role-change shapes.
pub struct FolderListMemberSpec<'a> {
    pub user: &'a TestUser,
    pub share_role: ShareRoleEnum,
    pub is_owner: bool,
    pub signed_by: &'a TestUser,
}

/// Sign a `FolderMemberListV1` over the supplied roster. Mirrors the
/// canonical encoder the server uses to reconstruct the bytes, so a
/// helper that builds a "post-mutation" projection and a server that
/// reads the live `user_files` table both end up at the same DER.
pub fn sign_folder_member_list(
    folder_id: Uuid,
    folder_owner_id: Uuid,
    members: &[FolderListMemberSpec<'_>],
    signer: &TestUser,
    signed_at: i64,
) -> Value {
    let payload = FolderMemberListV1 {
        folder_id: folder_id.into_bytes(),
        folder_owner_id: folder_owner_id.into_bytes(),
        members: members
            .iter()
            .map(|m| FolderListMember {
                user_id: m.user.user_id.into_bytes(),
                pubkey_fingerprint: m.user.fingerprint_bytes(),
                share_role: m.share_role,
                is_owner: m.is_owner,
                signed_by_user_id: m.signed_by.user_id.into_bytes(),
            })
            .collect(),
        members_signed_at: signed_at,
    };
    let der = encode_folder_member_list_v1(&payload).expect("encode folder member list");
    let mut signing_input = Vec::with_capacity(FOLDER_LIST_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(FOLDER_LIST_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    let signature = signer.sign_bytes(&signing_input);
    serde_json::json!({
        "signature": signature,
        "signed_at": signed_at,
        "signed_by_user_id": signer.user_id.to_string(),
    })
}

pub fn build_share_envelope(
    sender: &TestUser,
    recipient: &TestUser,
    role: ShareRoleEnum,
    root_file_id: Uuid,
    entries: Vec<(Uuid, Vec<u8>)>,
    nonce: [u8; 16],
    timestamp: i64,
) -> Value {
    build_envelope_for_action(
        ShareEnvelopeInputs {
            sender,
            recipient,
            role,
            root_file_id,
            entries,
            nonce,
            timestamp,
        },
        AuditEventActionEnum::Grant,
        None,
    )
}

/// Build an envelope whose `event_signature` covers a `role_change`
/// canonical input — the recipient already holds `previous_role` on
/// `root_file_id` and the caller is upgrading or downgrading them to
/// `new_role`. Matches what the SPA signs once it detects that the
/// recipient has an existing row on the root.
#[allow(clippy::too_many_arguments)]
pub fn build_role_change_envelope(
    sender: &TestUser,
    recipient: &TestUser,
    previous_role: ShareRoleEnum,
    new_role: ShareRoleEnum,
    root_file_id: Uuid,
    entries: Vec<(Uuid, Vec<u8>)>,
    nonce: [u8; 16],
    timestamp: i64,
) -> Value {
    build_envelope_for_action(
        ShareEnvelopeInputs {
            sender,
            recipient,
            role: new_role,
            root_file_id,
            entries,
            nonce,
            timestamp,
        },
        AuditEventActionEnum::RoleChange,
        Some(previous_role),
    )
}

/// Folder-share variant of `build_share_envelope` that carries a fresh
/// `members_list_signature`. Most editable-folder tests use this — the
/// caller passes the projected post-share roster (owner + sender's
/// existing position + recipient added/upgraded) and the signer (folder
/// owner on initial share, Co-owner on re-share).
#[allow(clippy::too_many_arguments)]
pub fn build_folder_share_envelope(
    sender: &TestUser,
    recipient: &TestUser,
    role: ShareRoleEnum,
    folder_id: Uuid,
    folder_owner_id: Uuid,
    nonce: [u8; 16],
    timestamp: i64,
    members_after: &[FolderListMemberSpec<'_>],
    list_signer: &TestUser,
) -> Value {
    build_folder_share_envelope_with_entries(
        sender,
        recipient,
        role,
        folder_id,
        folder_owner_id,
        vec![(folder_id, b"wrap-folder".to_vec())],
        nonce,
        timestamp,
        members_after,
        list_signer,
    )
}

/// Same as `build_folder_share_envelope` but lets the caller supply a
/// full entries list — needed for folder shares that recurse over
/// children (the server validates entries against the subtree).
#[allow(clippy::too_many_arguments)]
pub fn build_folder_share_envelope_with_entries(
    sender: &TestUser,
    recipient: &TestUser,
    role: ShareRoleEnum,
    folder_id: Uuid,
    folder_owner_id: Uuid,
    entries: Vec<(Uuid, Vec<u8>)>,
    nonce: [u8; 16],
    timestamp: i64,
    members_after: &[FolderListMemberSpec<'_>],
    list_signer: &TestUser,
) -> Value {
    let envelope = build_share_envelope(sender, recipient, role, folder_id, entries, nonce, timestamp);
    inject_list_signature(envelope, folder_id, folder_owner_id, members_after, list_signer, timestamp)
}

/// Folder variant of `build_role_change_envelope`. Use when the recipient
/// already holds a role on the folder root and the caller is changing it —
/// the server detects the existing row, requires a `role_change`-signed
/// audit event AND a fresh `members_list_signature` over the post-mutation
/// roster.
#[allow(clippy::too_many_arguments)]
pub fn build_folder_role_change_envelope(
    sender: &TestUser,
    recipient: &TestUser,
    previous_role: ShareRoleEnum,
    new_role: ShareRoleEnum,
    folder_id: Uuid,
    folder_owner_id: Uuid,
    nonce: [u8; 16],
    timestamp: i64,
    members_after: &[FolderListMemberSpec<'_>],
    list_signer: &TestUser,
) -> Value {
    let envelope = build_role_change_envelope(
        sender,
        recipient,
        previous_role,
        new_role,
        folder_id,
        vec![(folder_id, b"wrap-folder".to_vec())],
        nonce,
        timestamp,
    );
    inject_list_signature(envelope, folder_id, folder_owner_id, members_after, list_signer, timestamp)
}

#[allow(clippy::too_many_arguments)]
pub fn build_co_owner_folder_share_envelope(
    sender: &TestUser,
    recipient: &TestUser,
    role: ShareRoleEnum,
    folder_id: Uuid,
    folder_owner_id: Uuid,
    nonce: [u8; 16],
    timestamp: i64,
    members_after: &[FolderListMemberSpec<'_>],
    list_signer: &TestUser,
) -> Value {
    let envelope = build_co_owner_share_envelope(
        sender,
        recipient,
        role,
        folder_id,
        vec![(folder_id, b"wrap-folder".to_vec())],
        nonce,
        timestamp,
    );
    inject_list_signature(envelope, folder_id, folder_owner_id, members_after, list_signer, timestamp)
}

fn inject_list_signature(
    mut envelope: Value,
    folder_id: Uuid,
    folder_owner_id: Uuid,
    members_after: &[FolderListMemberSpec<'_>],
    list_signer: &TestUser,
    timestamp: i64,
) -> Value {
    let list_sig = sign_folder_member_list(
        folder_id,
        folder_owner_id,
        members_after,
        list_signer,
        timestamp,
    );
    let map = envelope
        .as_object_mut()
        .expect("envelope object");
    map.insert("members_list_signature".to_string(), list_sig);
    envelope
}

pub fn build_co_owner_share_envelope(
    sender: &TestUser,
    recipient: &TestUser,
    role: ShareRoleEnum,
    root_file_id: Uuid,
    entries: Vec<(Uuid, Vec<u8>)>,
    nonce: [u8; 16],
    timestamp: i64,
) -> Value {
    build_envelope_for_action(
        ShareEnvelopeInputs {
            sender,
            recipient,
            role,
            root_file_id,
            entries,
            nonce,
            timestamp,
        },
        AuditEventActionEnum::SharedByCoOwner,
        None,
    )
}

fn build_envelope_for_action(
    inputs: ShareEnvelopeInputs<'_>,
    audit_action: AuditEventActionEnum,
    share_role_before: Option<ShareRoleEnum>,
) -> Value {
    let ShareEnvelopeInputs {
        sender,
        recipient,
        role,
        root_file_id,
        entries,
        nonce,
        timestamp,
    } = inputs;
    let entries_for_hash: Vec<ShareEntry> = entries
        .iter()
        .map(|(file_id, encrypted_key)| ShareEntry {
            file_id: file_id.into_bytes(),
            encrypted_key: encrypted_key.clone(),
        })
        .collect();
    let entries_der = encode_entries_v1(&entries_for_hash).expect("encode entries");
    let mut hasher = Sha256::new();
    hasher.update(&entries_der);
    let entries_hash: [u8; 32] = hasher.finalize().into();

    let payload = ShareRequestPayloadV1 {
        sender_id: sender.user_id.into_bytes(),
        recipient_id: recipient.user_id.into_bytes(),
        recipient_pubkey_fingerprint: recipient.fingerprint_bytes(),
        share_role: role,
        root_file_id: root_file_id.into_bytes(),
        entries_hash,
        timestamp,
        nonce,
    };
    let payload_der = encode_share_request_v1(&payload).expect("encode payload");
    let mut signing_input = Vec::with_capacity(SHARE_REQUEST_V1_PREFIX.len() + payload_der.len());
    signing_input.extend_from_slice(SHARE_REQUEST_V1_PREFIX);
    signing_input.extend_from_slice(&payload_der);
    let signature = sender.sign_bytes(&signing_input);

    let event_signature = sign_audit_event(
        sender,
        recipient,
        root_file_id,
        audit_action,
        share_role_before,
        Some(role),
        timestamp,
    );

    serde_json::json!({
        "payload_der": cryptfns::base64::encode(&payload_der),
        "signature": signature,
        "event_signature": event_signature,
        "entries": entries
            .iter()
            .map(|(file_id, encrypted_key)| serde_json::json!({
                "file_id": file_id.to_string(),
                "encrypted_key": cryptfns::base64::encode(encrypted_key),
            }))
            .collect::<Vec<_>>(),
    })
}

/// Sign a `MemberSigPayloadV1` over the recipient's (pubkey, fingerprint,
/// role, signed_at) using the named signer's privkey. Returns the
/// base64 wire form. Mirrors `shareCrypto.signMember` on the SPA so
/// envelopes built here exercise the same producer the SPA ships.
pub fn sign_member_payload(
    signer: &TestUser,
    recipient: &TestUser,
    share_role: ShareRoleEnum,
    signed_at: i64,
) -> String {
    let pubkey_der = recipient
        .key_type
        .member_pubkey_der(&recipient.public_pem)
        .expect("recipient pubkey to DER");
    let payload = MemberSigPayloadV1 {
        user_id: recipient.user_id.into_bytes(),
        pubkey_der,
        fingerprint: recipient.fingerprint_bytes(),
        share_role,
        signed_at,
    };
    let der = encode_member_sig_v1(&payload).expect("encode MemberSigPayloadV1");
    let mut signing_input = Vec::with_capacity(MEMBER_SIG_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(MEMBER_SIG_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    signer.sign_bytes(&signing_input)
}

/// Inject `member_signature` + `member_signed_at` into an envelope built
/// by `build_share_envelope` / `build_co_owner_share_envelope`. Test
/// callers use this when they want the produced rows to land with a
/// verified σ rather than the legacy-NULL fallback.
pub fn inject_member_signature(
    mut envelope: Value,
    signer: &TestUser,
    recipient: &TestUser,
    share_role: ShareRoleEnum,
    signed_at: i64,
) -> Value {
    let sig = sign_member_payload(signer, recipient, share_role, signed_at);
    let map = envelope.as_object_mut().expect("envelope object");
    map.insert(
        "member_signature".to_string(),
        Value::String(sig),
    );
    map.insert(
        "member_signed_at".to_string(),
        Value::Number(signed_at.into()),
    );
    envelope
}

pub fn sign_audit_event(
    actor: &TestUser,
    target: &TestUser,
    file_id: Uuid,
    action: AuditEventActionEnum,
    share_role_before: Option<ShareRoleEnum>,
    share_role_after: Option<ShareRoleEnum>,
    timestamp: i64,
) -> String {
    let input = AuditEventSigInputV1 {
        sender_id: actor.user_id.into_bytes(),
        recipient_id: Some(target.user_id.into_bytes()),
        file_id: file_id.into_bytes(),
        action,
        share_role_before,
        share_role_after,
        timestamp,
    };
    let der = encode_audit_event_sig_input_v1(&input).expect("encode audit sig input");
    let mut signing_input = Vec::with_capacity(AUDIT_EVENT_SIG_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(AUDIT_EVENT_SIG_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    actor.sign_bytes(&signing_input)
}

pub fn build_revoke_body(
    caller: &TestUser,
    target: &TestUser,
    file_id: Uuid,
    role_before: ShareRoleEnum,
    timestamp: i64,
) -> Value {
    let event_signature = sign_audit_event(
        caller,
        target,
        file_id,
        AuditEventActionEnum::Revoke,
        Some(role_before),
        None,
        timestamp,
    );
    serde_json::json!({ "event_signature": event_signature, "timestamp": timestamp })
}

/// Folder-revoke variant of `build_revoke_body`. Caller passes the
/// projected post-revoke roster (current set minus the revoked
/// recipient, and minus any cascade-affected Co-owner grants when the
/// revoked recipient is a Co-owner).
#[allow(clippy::too_many_arguments)]
pub fn build_folder_revoke_body(
    caller: &TestUser,
    target: &TestUser,
    folder_id: Uuid,
    folder_owner_id: Uuid,
    role_before: ShareRoleEnum,
    timestamp: i64,
    members_after: &[FolderListMemberSpec<'_>],
    list_signer: &TestUser,
) -> Value {
    let event_signature = sign_audit_event(
        caller,
        target,
        folder_id,
        AuditEventActionEnum::Revoke,
        Some(role_before),
        None,
        timestamp,
    );
    let list_sig = sign_folder_member_list(
        folder_id,
        folder_owner_id,
        members_after,
        list_signer,
        timestamp,
    );
    serde_json::json!({
        "event_signature": event_signature,
        "timestamp": timestamp,
        "members_list_signature": list_sig,
    })
}

/// Build a signed `event_signature` covering an action that has only
/// a file_id and a timestamp (no recipient). Used by `upload-multikey`
/// and `move-into-shared`.
pub fn sign_no_recipient_event(
    actor: &TestUser,
    file_id: Uuid,
    action: AuditEventActionEnum,
    timestamp: i64,
) -> String {
    let input = AuditEventSigInputV1 {
        sender_id: actor.user_id.into_bytes(),
        recipient_id: None,
        file_id: file_id.into_bytes(),
        action,
        share_role_before: None,
        share_role_after: None,
        timestamp,
    };
    let der = encode_audit_event_sig_input_v1(&input).expect("encode no-recipient audit sig input");
    let mut signing_input = Vec::with_capacity(AUDIT_EVENT_SIG_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(AUDIT_EVENT_SIG_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    actor.sign_bytes(&signing_input)
}

#[allow(clippy::too_many_arguments)]
pub fn build_upload_multikey_body(
    new_file_id: Uuid,
    parent_id: Uuid,
    name_hash: &str,
    member_keys: Vec<(Uuid, &str, bool)>,
    members_signed_at: i64,
    members_list_signature: Option<String>,
    event_signature: String,
    timestamp: i64,
) -> Value {
    let keys: Vec<Value> = member_keys
        .into_iter()
        .map(|(uid, key, is_owner)| serde_json::json!({
            "user_id": uid.to_string(),
            "encrypted_key": key,
            "is_owner_of_file": is_owner,
        }))
        .collect();
    serde_json::json!({
        "new_file_id": new_file_id.to_string(),
        "parent_file_id": parent_id.to_string(),
        "name_hash": name_hash,
        "encrypted_name": "encrypted-name",
        "mime": "text/plain",
        "size": 1024,
        "chunks": 1,
        "sha256": "sha256",
        "member_keys": keys,
        "members_list_snapshot": {
            "members_signed_at": members_signed_at,
            "members_list_signature": members_list_signature,
        },
        "event_signature": event_signature,
        "timestamp": timestamp,
    })
}

pub fn build_move_into_shared_body(
    file_id: Uuid,
    destination_folder_id: Uuid,
    member_keys: Vec<(Uuid, &str)>,
    members_signed_at: i64,
    members_list_signature: Option<String>,
    event_signature: String,
    timestamp: i64,
) -> Value {
    let keys: Vec<Value> = member_keys
        .into_iter()
        .map(|(uid, key)| serde_json::json!({
            "user_id": uid.to_string(),
            "encrypted_key": key,
        }))
        .collect();
    serde_json::json!({
        "file_id": file_id.to_string(),
        "destination_folder_id": destination_folder_id.to_string(),
        "member_keys": keys,
        "members_list_snapshot": {
            "members_signed_at": members_signed_at,
            "members_list_signature": members_list_signature,
        },
        "event_signature": event_signature,
        "timestamp": timestamp,
    })
}

/// Folder-cascade variant of `build_move_into_shared_body`. `entries`
/// carries one `(node_id, member_keys)` pair per node in the moved subtree
/// (root + every descendant); the flat `member_keys` field is omitted so
/// the server takes the cascade branch.
#[allow(clippy::type_complexity)]
pub fn build_move_into_shared_cascade_body(
    root_id: Uuid,
    destination_folder_id: Uuid,
    entries: Vec<(Uuid, Vec<(Uuid, &str)>)>,
    members_signed_at: i64,
    members_list_signature: Option<String>,
    event_signature: String,
    timestamp: i64,
) -> Value {
    let entries_json: Vec<Value> = entries
        .into_iter()
        .map(|(node_id, keys)| {
            let keys_json: Vec<Value> = keys
                .into_iter()
                .map(|(uid, key)| {
                    serde_json::json!({
                        "user_id": uid.to_string(),
                        "encrypted_key": key,
                    })
                })
                .collect();
            serde_json::json!({
                "file_id": node_id.to_string(),
                "member_keys": keys_json,
            })
        })
        .collect();
    serde_json::json!({
        "file_id": root_id.to_string(),
        "destination_folder_id": destination_folder_id.to_string(),
        "entries": entries_json,
        "members_list_snapshot": {
            "members_signed_at": members_signed_at,
            "members_list_signature": members_list_signature,
        },
        "event_signature": event_signature,
        "timestamp": timestamp,
    })
}

/// Body for `POST /api/storage/move-out-of-shared`. `destination_folder_id`
/// is `None` for the owner's drive root.
pub fn build_move_out_of_shared_body(
    file_id: Uuid,
    destination_folder_id: Option<Uuid>,
    event_signature: String,
    timestamp: i64,
) -> Value {
    serde_json::json!({
        "file_id": file_id.to_string(),
        "destination_folder_id": destination_folder_id.map(|id| id.to_string()),
        "event_signature": event_signature,
        "timestamp": timestamp,
    })
}

/// Sign one cascade file's `grant` audit canonical for a group member-add.
/// The signed input mirrors the single-file grant in `share.rs`:
/// recipient = new member, `file_id` = the cascade target, role_after =
/// the add's role. Returns the base64 signature for that file.
pub fn sign_add_group_member_event(
    actor: &TestUser,
    new_member: &TestUser,
    file_id: Uuid,
    role: ShareRoleEnum,
    timestamp: i64,
) -> String {
    let input = AuditEventSigInputV1 {
        sender_id: actor.user_id.into_bytes(),
        recipient_id: Some(new_member.user_id.into_bytes()),
        file_id: file_id.into_bytes(),
        action: AuditEventActionEnum::Grant,
        share_role_before: None,
        share_role_after: Some(role),
        timestamp,
    };
    let der = encode_audit_event_sig_input_v1(&input).expect("encode group-add audit sig input");
    let mut signing_input = Vec::with_capacity(AUDIT_EVENT_SIG_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(AUDIT_EVENT_SIG_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    actor.sign_bytes(&signing_input)
}

/// Build the `event_signatures` map (file_id → base64 sig) the group
/// member-add route expects: one per-file signature for every cascade
/// target.
pub fn sign_add_group_member_events(
    actor: &TestUser,
    new_member: &TestUser,
    file_ids: &[Uuid],
    role: ShareRoleEnum,
    timestamp: i64,
) -> std::collections::HashMap<String, String> {
    file_ids
        .iter()
        .map(|file_id| {
            (
                file_id.to_string(),
                sign_add_group_member_event(actor, new_member, *file_id, role, timestamp),
            )
        })
        .collect()
}

/// Build the signed `event_signature` for `POST /api/shares/{file_id}/
/// fork`. Fork has no recipient — the audit row credits the source
/// file id via the row's `file_id` column, so the
/// owner of the source file sees who saved a copy.
pub fn sign_fork_event(actor: &TestUser, source_file_id: Uuid, timestamp: i64) -> String {
    sign_no_recipient_event(actor, source_file_id, AuditEventActionEnum::Fork, timestamp)
}

pub fn build_evict_body(actor: &TestUser, target: &TestUser, file_id: Uuid, timestamp: i64) -> Value {
    let event_signature = sign_audit_event(
        actor,
        target,
        file_id,
        AuditEventActionEnum::SharedFolderEvict,
        None,
        None,
        timestamp,
    );
    serde_json::json!({
        "event_signature": event_signature,
        "timestamp": timestamp,
    })
}

pub fn random_nonce() -> [u8; 16] {
    use rand::RngCore;
    let mut nonce = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce);
    nonce
}

pub fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

pub fn generate_keypair() -> (String, String, String) {
    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_pem = cryptfns::rsa::public::to_string(&public).unwrap();
    let private_pem = cryptfns::rsa::private::to_string(&private).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();
    (private_pem, public_pem, fingerprint)
}

/// Key material for a curve25519 account: Ed25519 identity keypair plus hybrid
/// X25519 + ML-KEM wrapping keypair. Returns `(private_pem, public_pem,
/// fingerprint, wrapping_private_pem, wrapping_public_pem)`.
pub fn generate_curve25519_keypair() -> (String, String, String, String, String) {
    let private_pem = cryptfns::ed25519::private::generate().unwrap();
    let public_pem = cryptfns::ed25519::public::from_private(&private_pem).unwrap();
    let fingerprint = cryptfns::spki::fingerprint(&public_pem).unwrap();
    let wrapping_private_pem = cryptfns::ecdh::private::generate().unwrap();
    let wrapping_public_pem = cryptfns::ecdh::public::from_private(&wrapping_private_pem).unwrap();
    (private_pem, public_pem, fingerprint, wrapping_private_pem, wrapping_public_pem)
}

/// Seed a legacy RSA account at the data layer and log it in. Registration no
/// longer creates RSA accounts, but the sharing routes must keep working for
/// the pre-migration accounts real deployments still hold — and the RSA↔curve
/// interop and chain-resolution suites need genuine RSA participants. Expands
/// to a `let $user = TestUser { ... };` binding.
#[macro_export]
macro_rules! register_user {
    ($app:expr, $context:expr, $user:ident, $email:expr) => {
        let $user = $crate::shares_common::seed_legacy_test_user(&$app, &$context.db, $email).await;
    };
}

/// Curve25519 sibling of `register_user!` — registers a born-migrated v2
/// account through the real OPAQUE registration handshake: unauthenticated
/// `register/pake/start`, then `register` with the resulting upload. Mirrors
/// the client's ceremony so the fixture matches a real curve25519 signup.
#[macro_export]
macro_rules! register_curve25519_user {
    ($app:expr, $user:ident, $email:expr) => {
        let $user =
            $crate::shares_common::register_curve25519(&$app, $email).await;
    };
}

#[macro_export]
macro_rules! create_file {
    ($app:expr, $user:expr, $name_hash:expr) => {{
        let payload = $crate::shares_common::make_create_file_for(&$user, $name_hash);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/storage")
            .cookie($user.jwt.clone())
            .set_json(&payload)
            .to_request();
        let body = actix_web::test::call_and_read_body(&$app, req).await;
        serde_json::from_slice::<storage::data::app_file::AppFile>(&body).expect("create_file json")
    }};
}

#[macro_export]
macro_rules! create_folder {
    ($app:expr, $user:expr, $name_hash:expr) => {{
        let payload = $crate::shares_common::make_create_folder(&$user, $name_hash, None);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/storage")
            .cookie($user.jwt.clone())
            .set_json(&payload)
            .to_request();
        let body = actix_web::test::call_and_read_body(&$app, req).await;
        serde_json::from_slice::<storage::data::app_file::AppFile>(&body)
            .expect("create_folder json")
    }};
    ($app:expr, $user:expr, $name_hash:expr, $parent_id:expr) => {{
        let payload = $crate::shares_common::make_create_folder(&$user, $name_hash, Some($parent_id));
        let req = actix_web::test::TestRequest::post()
            .uri("/api/storage")
            .cookie($user.jwt.clone())
            .set_json(&payload)
            .to_request();
        let body = actix_web::test::call_and_read_body(&$app, req).await;
        serde_json::from_slice::<storage::data::app_file::AppFile>(&body)
            .expect("create_folder json")
    }};
}

#[macro_export]
macro_rules! create_child_file {
    ($app:expr, $user:expr, $name_hash:expr, $parent_id:expr) => {{
        let payload =
            $crate::shares_common::make_create_child_file(&$user, $name_hash, $parent_id);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/storage")
            .cookie($user.jwt.clone())
            .set_json(&payload)
            .to_request();
        let body = actix_web::test::call_and_read_body(&$app, req).await;
        serde_json::from_slice::<storage::data::app_file::AppFile>(&body)
            .expect("create_child_file json")
    }};
}

#[macro_export]
macro_rules! post_share {
    ($app:expr, $caller:expr, $envelope:expr) => {{
        let req = actix_web::test::TestRequest::post()
            .uri("/api/shares")
            .cookie($caller.jwt.clone())
            .set_json(&$envelope)
            .to_request();
        actix_web::test::call_service(&$app, req).await
    }};
}

#[macro_export]
macro_rules! delete_share {
    ($app:expr, $caller:expr, $file_id:expr, $recipient_id:expr, $body:expr) => {{
        let req = actix_web::test::TestRequest::delete()
            .uri(&format!("/api/shares/{}/{}", $file_id, $recipient_id))
            .cookie($caller.jwt.clone())
            .set_json(&$body)
            .to_request();
        actix_web::test::call_service(&$app, req).await
    }};
}

#[macro_export]
macro_rules! grant {
    ($app:expr, $sender:expr, $recipient:expr, $role:expr, $file_id:expr) => {{
        let envelope = $crate::shares_common::build_share_envelope(
            &$sender,
            &$recipient,
            $role,
            $file_id,
            vec![($file_id, b"wrapped".to_vec())],
            $crate::shares_common::random_nonce(),
            $crate::shares_common::now_secs(),
        );
        let resp = post_share!($app, $sender, envelope);
        assert!(
            resp.status().is_success(),
            "grant from {} to {} failed: {:?}",
            $sender.email,
            $recipient.email,
            resp.status()
        );
    }};
}

/// Folder-share grant — same as `grant!` but stamps the post-share
/// `members_list_signature` so the server's hard requirement is met.
/// `members_after` is the projected roster the helper signs over.
#[macro_export]
macro_rules! grant_folder {
    ($app:expr, $sender:expr, $recipient:expr, $role:expr, $folder_id:expr, $folder_owner:expr, $members_after:expr, $list_signer:expr) => {{
        let timestamp = $crate::shares_common::now_secs();
        let envelope = $crate::shares_common::build_folder_share_envelope(
            &$sender,
            &$recipient,
            $role,
            $folder_id,
            $folder_owner.user_id,
            $crate::shares_common::random_nonce(),
            timestamp,
            $members_after,
            &$list_signer,
        );
        let resp = post_share!($app, $sender, envelope);
        assert!(
            resp.status().is_success(),
            "folder grant from {} to {} failed: {:?}",
            $sender.email,
            $recipient.email,
            resp.status()
        );
    }};
}
