pub(crate) const CHUNKS: usize = 5;
pub(crate) const CHUNK_SIZE_BYTES: i32 = 1024 * 1024;

/// The bcrypt password a seeded legacy account is created with. Tests that log
/// the account in, or migrate it via OPAQUE, use this.
#[allow(dead_code)]
pub(crate) const LEGACY_PASSWORD: &str = "not-4-weak-password-for-god-sakes!";

/// A legacy (RSA + bcrypt) account inserted directly at the data layer.
///
/// Registration is Curve25519 + OPAQUE only, so a legacy account can no longer
/// be created through the register endpoint — but real deployments still hold
/// pre-migration RSA accounts, and the login-time migration path must keep
/// working. Tests that exercise that path seed the account here.
#[allow(dead_code)]
pub(crate) struct SeededLegacy {
    pub user_id: entity::Uuid,
    pub rsa_private: String,
    pub rsa_public: String,
    pub rsa_fingerprint: String,
}

/// Insert a legacy RSA account with a real bcrypt password and a real RSA
/// keypair (so signature and transition-certificate signing work). The
/// `encrypted_private_key` is an opaque stand-in — no test decrypts it; the
/// migration path re-wraps under the new key rather than reading the old one.
#[allow(dead_code)]
pub(crate) async fn seed_legacy_user<C: entity::ConnectionTrait>(
    db: &C,
    email: &str,
) -> SeededLegacy {
    use entity::{ActiveValue, EntityTrait, PaginatorTrait};

    let private = cryptfns::rsa::private::generate().unwrap();
    let rsa_private = cryptfns::rsa::private::to_string(&private).unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let rsa_public = cryptfns::rsa::public::to_string(&public).unwrap();
    let rsa_fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    // The register contract makes the first account the instance admin; mirror
    // that here so tests seeding their first user get the same identity the
    // real signup path would have handed them.
    let role = if entity::users::Entity::find().count(db).await.unwrap() == 0 {
        ActiveValue::Set(Some("admin".to_string()))
    } else {
        ActiveValue::NotSet
    };

    let user_id = entity::Uuid::new_v4();
    let now = chrono::Utc::now().timestamp();
    entity::users::Entity::insert(entity::users::ActiveModel {
        id: ActiveValue::Set(user_id),
        role,
        quota: ActiveValue::NotSet,
        email: ActiveValue::Set(email.to_string()),
        password: ActiveValue::Set(Some(util::password::hash(LEGACY_PASSWORD))),
        secret: ActiveValue::NotSet,
        pubkey: ActiveValue::Set(rsa_public.clone()),
        fingerprint: ActiveValue::Set(rsa_fingerprint.clone()),
        key_type: ActiveValue::Set("rsa".to_string()),
        wrapping_pubkey: ActiveValue::NotSet,
        security_version: ActiveValue::Set(0),
        opaque_password_file: ActiveValue::NotSet,
        encrypted_private_key: ActiveValue::Set(Some("legacy-encrypted-key".to_string())),
        email_verified_at: ActiveValue::Set(Some(now)),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
        share_notifications_enabled: ActiveValue::Set(true),
    })
    .exec_without_returning(db)
    .await
    .unwrap();

    SeededLegacy {
        user_id,
        rsa_private,
        rsa_public,
        rsa_fingerprint,
    }
}

/// Bulk-seed `count` owned files for `user_id` directly at the data layer,
/// returning their ids. The migration regression tests need more files than the
/// pre-fix single-POST body could carry (~9.6k); creating them through the
/// storage route would take far too long, and migration only ever reads
/// `user_files.encrypted_key`, so realistic file rows are unnecessary.
#[allow(dead_code)]
pub(crate) async fn seed_owned_files<C: entity::ConnectionTrait>(
    db: &C,
    user_id: entity::Uuid,
    count: usize,
) -> Vec<entity::Uuid> {
    use entity::{files, user_files, ActiveValue, EntityTrait};

    let now = chrono::Utc::now().timestamp();
    let mut file_ids = Vec::with_capacity(count);
    let mut file_rows = Vec::with_capacity(count);
    let mut user_file_rows = Vec::with_capacity(count);

    for _ in 0..count {
        let file_id = entity::Uuid::new_v4();
        file_ids.push(file_id);

        file_rows.push(files::ActiveModel {
            id: ActiveValue::Set(file_id),
            name_hash: ActiveValue::Set("hash".to_string()),
            encrypted_name: ActiveValue::Set("name".to_string()),
            encrypted_thumbnail: ActiveValue::Set(None),
            mime: ActiveValue::Set("text/plain".to_string()),
            size: ActiveValue::Set(Some(1)),
            chunks: ActiveValue::Set(Some(1)),
            chunks_stored: ActiveValue::Set(Some(1)),
            file_id: ActiveValue::Set(None),
            md5: ActiveValue::Set(None),
            sha1: ActiveValue::Set(None),
            sha256: ActiveValue::Set(None),
            blake2b: ActiveValue::Set(None),
            cipher: ActiveValue::Set("aegis256".to_string()),
            editable: ActiveValue::Set(false),
            file_modified_at: ActiveValue::Set(now),
            created_at: ActiveValue::Set(now),
            finished_upload_at: ActiveValue::Set(Some(now)),
            active_version: ActiveValue::Set(1),
            pending_version: ActiveValue::Set(None),
            pending_chunks: ActiveValue::Set(None),
            pending_size: ActiveValue::Set(None),
            last_membership_change_at: ActiveValue::Set(None),
            members_list_signature: ActiveValue::Set(None),
            members_list_signed_at: ActiveValue::Set(None),
            members_list_signed_by_user_id: ActiveValue::Set(None),
        });

        user_file_rows.push(user_files::ActiveModel {
            id: ActiveValue::Set(entity::Uuid::new_v4()),
            file_id: ActiveValue::Set(file_id),
            user_id: ActiveValue::Set(user_id),
            encrypted_key: ActiveValue::Set("old-rsa-wrapped-key".to_string()),
            is_owner: ActiveValue::Set(true),
            created_at: ActiveValue::Set(now),
            expires_at: ActiveValue::Set(None),
            // Owner rows carry the highest non-owner tier; `permission()` returns
            // Owner from `is_owner` regardless, so the value is moot but must
            // satisfy the column's CHECK constraint.
            share_role: ActiveValue::Set("co-owner".to_string()),
            shared_at: ActiveValue::Set(None),
            shared_by_user_id: ActiveValue::Set(None),
            member_signature: ActiveValue::Set(None),
        });
    }

    // Chunked so the parameter count stays well under SQLite's bind limit.
    for chunk in file_rows.chunks(100) {
        files::Entity::insert_many(chunk.to_vec())
            .exec_without_returning(db)
            .await
            .unwrap();
    }
    for chunk in user_file_rows.chunks(100) {
        user_files::Entity::insert_many(chunk.to_vec())
            .exec_without_returning(db)
            .await
            .unwrap();
    }

    file_ids
}

/// The service produced by `test::init_service(server::app(..))`, so helpers
/// that drive the routes can take the app by reference.
#[allow(dead_code)]
pub(crate) trait TestApp:
    actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse<
        actix_web::body::EitherBody<actix_web::body::BoxBody>,
    >,
    Error = actix_web::Error,
>
{
}
impl<S> TestApp for S where
    S: actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<
            actix_web::body::EitherBody<actix_web::body::BoxBody>,
        >,
        Error = actix_web::Error,
    >
{
}

/// A Curve25519 + OPAQUE account created through the real register endpoint.
#[allow(dead_code)]
pub(crate) struct RegisteredCurve {
    pub user_id: entity::Uuid,
    pub jwt: actix_web::cookie::Cookie<'static>,
}

/// Build the JSON body of a v2 (Curve25519 + OPAQUE) `/api/auth/register`
/// request by running the OPAQUE registration handshake through the
/// unauthenticated start endpoint. Tests that POST it themselves — to inspect
/// the raw status, e.g. registration gating — use this instead of
/// [`register_curve25519`], which asserts success.
#[allow(dead_code)]
pub(crate) async fn build_curve25519_register_body(
    app: &impl TestApp,
    email: &str,
) -> serde_json::Value {
    use actix_web::test;

    let ed_private = cryptfns::ed25519::private::generate().unwrap();
    let ed_public = cryptfns::ed25519::public::from_private(&ed_private).unwrap();
    let fingerprint = cryptfns::spki::fingerprint(&ed_public).unwrap();
    let x_private = cryptfns::ecdh::private::generate().unwrap();
    let x_public = cryptfns::ecdh::public::from_private(&x_private).unwrap();

    let reg_start = cryptfns::opaque::client_registration_start(LEGACY_PASSWORD.as_bytes()).unwrap();
    let req = test::TestRequest::post()
        .uri("/api/auth/register/pake/start")
        .set_json(serde_json::json!({ "email": email, "registration_request": reg_start.message }))
        .to_request();
    let body: serde_json::Value = test::read_body_json(test::call_service(app, req).await).await;
    let reg_response = body["registration_response"].as_str().unwrap();
    let reg_finish = cryptfns::opaque::client_registration_finish(
        &reg_start.state,
        reg_response,
        LEGACY_PASSWORD.as_bytes(),
    )
    .unwrap();

    let export_key = cryptfns::base64::decode(&reg_finish.export_key).unwrap();
    let kek = cryptfns::envelope::derive_kek(&export_key).unwrap();
    let envelope = cryptfns::envelope::seal(
        &kek,
        format!("v1|ed:{ed_private}|x:{x_private}").as_bytes(),
    )
    .unwrap();

    serde_json::json!({
        "email": email,
        "pubkey": ed_public,
        "wrapping_pubkey": x_public,
        "fingerprint": fingerprint,
        "key_type": "curve25519",
        "encrypted_private_key": envelope,
        "opaque_registration_upload": reg_finish.message,
    })
}

/// Register a born-migrated Curve25519 account through the real OPAQUE
/// registration handshake — the only way to create an account now. Returns the
/// user id and session cookie for tests that just need a working account.
#[allow(dead_code)]
pub(crate) async fn register_curve25519(app: &impl TestApp, email: &str) -> RegisteredCurve {
    use actix_web::test;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&build_curve25519_register_body(app, email).await)
        .to_request();
    let resp = test::call_service(app, req).await;
    assert!(resp.status().is_success(), "register {email} failed: {:?}", resp.status());
    let (jwt, _) = extract_cookies(resp.headers());
    let jwt = jwt.expect("register response missing JWT cookie");
    let body: serde_json::Value = test::read_body_json(resp).await;
    let user_id = entity::Uuid::parse_str(body["user"]["id"].as_str().unwrap()).unwrap();

    RegisteredCurve { user_id, jwt }
}

/// Helper for testing to extract the cookies
#[allow(dead_code)]
pub(crate) fn extract_cookies(
    headers: &actix_web::http::header::HeaderMap,
) -> (
    Option<actix_web::cookie::Cookie<'static>>,
    Option<actix_web::cookie::Cookie<'static>>,
) {
    let mut cookies = headers
        .get_all("set-cookie")
        .clone()
        .map(|h| {
            let h = h.clone().to_str().unwrap().to_string();

            actix_web::cookie::Cookie::parse(h).unwrap()
        })
        .collect::<Vec<actix_web::cookie::Cookie<'static>>>()
        .into_iter();

    let jwt = cookies.clone().find(|c| c.name() == "hoodik_session");
    let refresh = cookies.find(|c| c.name() == "hoodik_refresh");

    (jwt, refresh)
}

/// Helper to create some mock file for uploading
#[allow(dead_code)]
pub(crate) fn create_byte_chunks() -> (Vec<Vec<u8>>, i64, String) {
    let one_chunk_size = CHUNK_SIZE_BYTES as usize;
    let mut byte_chunks = vec![];
    let mut body = vec![];

    while body.len() < (one_chunk_size * CHUNKS) {
        body.extend(b"a");
    }

    let checksum = cryptfns::sha256::digest(body.as_slice());

    for i in (0..body.len()).step_by(one_chunk_size) {
        let chunk = &body[i..(i + one_chunk_size)];
        byte_chunks.push(chunk.to_vec());
    }

    let total_len = byte_chunks.iter().map(|chunk| chunk.len()).sum::<usize>() as i64;

    (byte_chunks, total_len, checksum)
}

#[allow(dead_code)]
pub(crate) fn calculate_checksum(data: Vec<Vec<u8>>) -> String {
    let mut body = vec![];

    for chunk in data {
        body.extend(chunk);
    }

    cryptfns::sha256::digest(body.as_slice())
}
