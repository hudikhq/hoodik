#[path = "./helpers.rs"]
mod helpers;

use actix_web::test;
use hoodik::server;
use links::data::app_link::AppLink;
use serde_json::json;
use storage::data::app_file::AppFile;

use crate::helpers::{create_byte_chunks, CHUNK_SIZE_BYTES};

#[actix_web::test]
async fn test_creating_and_downloading_link() {
    let context = context::Context::mock_with_data_dir(Some("../data/test-links".to_string())).await;

    let app = test::init_service(server::app(context.clone())).await;

    // Public links are signed and their link key is RSA-wrapped against the
    // owner's account key, so the owner is a legacy RSA account.
    let owner = helpers::seed_legacy_user(&context.db, "john@doe.com").await;
    let public_string = owner.rsa_public.clone();
    let private_string = owner.rsa_private.clone();

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": "john@doe.com", "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let (data, size, checksum) = create_byte_chunks();
    assert_eq!(data.len(), size as usize / CHUNK_SIZE_BYTES as usize);

    let random_file = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        content_tokens_root: None,
        content_tokens_file: None,
        name_hash: Some(helpers::name_tag(&checksum)),
        mime: Some("text/plain".to_string()),
        size: Some(size),
        chunks: Some(data.len() as i64),
        file_id: None,
        // Date of the file creation from the disk, if not provided we set it to now
        file_modified_at: None,
        md5: Some("asd".to_string()),
        sha1: Some("asd".to_string()),
        sha256: Some("asd".to_string()),
        blake2b: Some("asd".to_string()),
        digest_tokens_root: None,
        digest_tokens_file: None,
        cipher: None,
        editable: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&random_file)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    // let string_body = String::from_utf8(body.to_vec()).unwrap();
    // println!("string_body: {}", string_body);

    let mut file: AppFile = serde_json::from_slice(&body).unwrap();
    let file_key = cryptfns::aes::generate_key().unwrap();
    let file_key_hex = cryptfns::hex::encode(file_key.clone());

    // println!("file: {:#?}", file);

    let mut uploaded = vec![];
    for (i, chunk) in data.into_iter().enumerate() {
        println!("chunk: {}", i);
        // println!("chunk: {}", i);
        let encrypted = cryptfns::aes::encrypt(file_key.clone(), chunk).unwrap();
        let checksum = cryptfns::sha256::digest(encrypted.as_slice());
        let uri = format!("/api/storage/{}?checksum={}&chunk={}", &file.id, checksum, i);

        let req = test::TestRequest::post()
            .uri(uri.as_str())
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(encrypted)
            .to_request();

        let body = test::call_and_read_body(&app, req).await;
        // let string_body = String::from_utf8(body.to_vec()).unwrap();
        // println!("string_body: {}", string_body);

        file = serde_json::from_slice(&body).unwrap();
        uploaded.push(i as i64);

        assert_eq!(file.uploaded_chunks.clone().unwrap(), uploaded);
        assert_eq!(file.chunks_stored.unwrap(), i as i64 + 1);
    }

    assert!(file.finished_upload_at.is_some());

    let link_key = cryptfns::aes::generate_key().unwrap();
    let link_key_hex = cryptfns::hex::encode(link_key.clone());
    let link_key_rsa_enc = cryptfns::rsa::public::encrypt(&link_key_hex, &public_string).unwrap();
    let signature =
        cryptfns::rsa::private::sign(file.id.to_string().as_str(), &private_string).unwrap();
    let encrypted_name = cryptfns::aes::encrypt(
        link_key.clone(),
        "random-file.txt".to_string().as_bytes().to_vec(),
    )
    .unwrap();
    let encrypted_name_hex = cryptfns::hex::encode(encrypted_name.clone());
    let file_key_hex_aes_enc =
        cryptfns::aes::encrypt(link_key.clone(), file_key_hex.clone().as_bytes().to_vec()).unwrap();
    let file_key_hex_aes_enc_hex = cryptfns::hex::encode(file_key_hex_aes_enc.clone());

    let create_link = links::data::create_link::CreateLink {
        file_id: Some(file.id.to_string()),
        signature: Some(signature),
        encrypted_name: Some(encrypted_name_hex),
        encrypted_link_key: Some(link_key_rsa_enc),
        encrypted_thumbnail: None,
        encrypted_file_key: Some(file_key_hex_aes_enc_hex),
        expires_at: None,
    };
    let req = test::TestRequest::post()
        .uri("/api/links")
        .cookie(jwt.clone())
        .set_json(create_link)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let link: AppLink = serde_json::from_slice(&body).unwrap();

    // E2EE closure: the server streams raw ciphertext and never reads a request
    // body. A stray `link_key` from an old client is ignored, and the returned
    // bytes are ciphertext (checksum differs from the known plaintext checksum).
    let uri = format!("/api/links/{}", link.id);
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_json(json!({ "link_key": "deadbeef" }))
        // .cookie(jwt.clone()) - no need for jwt, this should be public
        .to_request();

    let contents = test::call_and_read_body(&app, req).await.to_vec();

    let content_len = contents.len();
    let received_checksum = cryptfns::sha256::digest(contents.as_slice());

    // Ciphertext size is >= plaintext (AEAD overhead); exact depends on chunking.
    assert!(content_len >= size as usize);
    // The body must NOT be the plaintext (server never decrypts for public link content).
    assert_ne!(received_checksum, checksum, "public link content download must return ciphertext");

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/metadata", &file.id).as_str())
        .cookie(jwt)
        .set_json(&random_file)
        .to_request();

    let file =
        serde_json::from_slice::<AppFile>(&test::call_and_read_body(&app, req).await).unwrap();

    // println!("file: {:#?}", file);

    assert!(file.link.is_some());
    assert_eq!(file.link.unwrap().id, link.id);

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_link_download_decrypts_aegis256_file() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-links-256".to_string())).await;

    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::seed_legacy_user(&context.db, "jane@doe.com").await;
    let public_string = owner.rsa_public.clone();
    let private_string = owner.rsa_private.clone();

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": "jane@doe.com", "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let (data, size, checksum) = create_byte_chunks();

    let random_file = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        content_tokens_root: None,
        content_tokens_file: None,
        name_hash: Some(helpers::name_tag(&checksum)),
        mime: Some("text/plain".to_string()),
        size: Some(size),
        chunks: Some(data.len() as i64),
        file_id: None,
        file_modified_at: None,
        md5: Some("asd".to_string()),
        sha1: Some("asd".to_string()),
        sha256: Some("asd".to_string()),
        blake2b: Some("asd".to_string()),
        digest_tokens_root: None,
        digest_tokens_file: None,
        cipher: Some("aegis256".to_string()),
        editable: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&random_file)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let mut file: AppFile = serde_json::from_slice(&body).unwrap();

    // The client encrypts chunks before upload — the server only ever sees
    // AEGIS-256 ciphertext.
    let file_key = cryptfns::aegis256::generate_key().unwrap();
    let file_key_hex = cryptfns::hex::encode(file_key.clone());

    for (i, chunk) in data.iter().enumerate() {
        let encrypted = cryptfns::aegis256::encrypt(file_key.clone(), chunk.clone()).unwrap();
        let checksum = cryptfns::sha256::digest(encrypted.as_slice());
        let uri = format!(
            "/api/storage/{}?checksum={}&chunk={}",
            &file.id, checksum, i
        );

        let req = test::TestRequest::post()
            .uri(uri.as_str())
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(encrypted)
            .to_request();

        let body = test::call_and_read_body(&app, req).await;
        file = serde_json::from_slice(&body).unwrap();
    }

    assert!(file.finished_upload_at.is_some());

    let link_key = cryptfns::aes::generate_key().unwrap();
    let link_key_hex = cryptfns::hex::encode(link_key.clone());
    let link_key_rsa_enc = cryptfns::rsa::public::encrypt(&link_key_hex, &public_string).unwrap();
    let signature =
        cryptfns::rsa::private::sign(file.id.to_string().as_str(), &private_string).unwrap();
    let encrypted_name = cryptfns::aes::encrypt(
        link_key.clone(),
        "aegis256-file.txt".to_string().as_bytes().to_vec(),
    )
    .unwrap();
    let file_key_hex_enc =
        cryptfns::aes::encrypt(link_key.clone(), file_key_hex.as_bytes().to_vec()).unwrap();

    let create_link = links::data::create_link::CreateLink {
        file_id: Some(file.id.to_string()),
        signature: Some(signature),
        encrypted_name: Some(cryptfns::hex::encode(encrypted_name)),
        encrypted_link_key: Some(link_key_rsa_enc),
        encrypted_thumbnail: None,
        encrypted_file_key: Some(cryptfns::hex::encode(file_key_hex_enc)),
        expires_at: None,
    };
    let req = test::TestRequest::post()
        .uri("/api/links")
        .cookie(jwt.clone())
        .set_json(create_link)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let link: AppLink = serde_json::from_slice(&body).unwrap();

    // E2EE: server returns ciphertext for public link content.
    let req = test::TestRequest::post()
        .uri(&format!("/api/links/{}", link.id))
        .to_request();

    let contents = test::call_and_read_body(&app, req).await.to_vec();

    // Ciphertext size >= plaintext.
    assert!(contents.len() >= size as usize);
    let received = cryptfns::sha256::digest(contents.as_slice());
    assert_ne!(received, checksum, "public link content must be ciphertext only");

    // Per-chunk download must return exactly one chunk's ciphertext, and the
    // chunks concatenated in order must reproduce the whole-file stream — this
    // is what lets the recipient decrypt a multi-chunk file client-side.
    use entity::EntityTrait;
    let downloads_before = entity::links::Entity::find_by_id(link.id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap()
        .downloads;
    let mut per_chunk = Vec::new();
    for i in 0..data.len() {
        let req = test::TestRequest::post()
            .uri(&format!("/api/links/{}?chunk={}", link.id, i))
            .to_request();
        per_chunk.extend(test::call_and_read_body(&app, req).await.to_vec());
    }
    assert_eq!(per_chunk, contents, "chunked link download must match the full stream");

    // One recipient fetching every chunk is one download, not one per chunk:
    // only the final chunk request increments the owner-visible counter.
    let downloads_after = entity::links::Entity::find_by_id(link.id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap()
        .downloads;
    assert_eq!(
        downloads_after - downloads_before,
        1,
        "downloading all {} chunks increments the counter by one, not per chunk",
        data.len()
    );

    context.config.app.cleanup();
}

/// The anonymous link route streams too, so a chunk that was never written
/// used to reach the recipient under a 200 and get handed to the cipher.
/// Absence has to be a 404, and it must not consume the download counter.
///
/// The fixture declares a size spanning two chunks and then uploads only the
/// first, so the missing chunk is also the *last* one — the single index that
/// the route counts as a completed download. Any smaller declared size makes
/// the counter assertion below pass no matter where the increment happens.
#[actix_web::test]
async fn test_link_download_missing_chunk_returns_404() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-links-missing".to_string())).await;

    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::seed_legacy_user(&context.db, "gap@doe.com").await;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": "gap@doe.com", "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let create_file = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        content_tokens_root: None,
        content_tokens_file: None,
        name_hash: Some(helpers::name_tag("gap")),
        mime: Some("text/plain".to_string()),
        size: Some(fs::MAX_CHUNK_SIZE_BYTES as i64 + 1),
        chunks: Some(2),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        digest_tokens_root: None,
        digest_tokens_file: None,
        cipher: None,
        editable: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create_file)
        .to_request();
    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let payload = b"chunk-zero".to_vec();
    let uri = format!(
        "/api/storage/{}?checksum={}&chunk=0",
        &file.id,
        cryptfns::sha256::digest(payload.as_slice())
    );
    let req = test::TestRequest::post()
        .uri(uri.as_str())
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(payload)
        .to_request();
    test::call_service(&app, req).await;

    let link_key = cryptfns::aes::generate_key().unwrap();
    let link_key_hex = cryptfns::hex::encode(link_key.clone());
    let create_link = links::data::create_link::CreateLink {
        file_id: Some(file.id.to_string()),
        signature: Some(
            cryptfns::rsa::private::sign(file.id.to_string().as_str(), &owner.rsa_private).unwrap(),
        ),
        encrypted_name: Some(cryptfns::hex::encode(
            cryptfns::aes::encrypt(link_key.clone(), b"gap.txt".to_vec()).unwrap(),
        )),
        encrypted_link_key: Some(
            cryptfns::rsa::public::encrypt(&link_key_hex, &owner.rsa_public).unwrap(),
        ),
        encrypted_thumbnail: None,
        encrypted_file_key: Some(cryptfns::hex::encode(
            cryptfns::aes::encrypt(link_key, b"file-key".to_vec()).unwrap(),
        )),
        expires_at: None,
    };
    let req = test::TestRequest::post()
        .uri("/api/links")
        .cookie(jwt.clone())
        .set_json(create_link)
        .to_request();
    let link: AppLink = serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/links/{}?chunk=1", link.id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);

    let body = test::read_body(resp).await;
    assert!(
        !body.starts_with(b"<?xml"),
        "404 body must not be a storage-provider error document: {}",
        String::from_utf8_lossy(&body)
    );

    use entity::EntityTrait;
    let downloads = entity::links::Entity::find_by_id(link.id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap()
        .downloads;
    assert_eq!(
        downloads, 0,
        "the missing chunk is the last one, so counting it before the stream \
         resolved would have registered a download that never happened"
    );

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}

/// The link manifest hands out URLs straight to the bucket, so it is the only
/// gate on that path — and it is deliberately unauthenticated, because the
/// link *is* the credential. What it must still refuse: a link whose time is
/// up, and a link that does not exist. Both are checked before any URL work,
/// and neither may move the download counter.
#[actix_web::test]
async fn test_link_chunk_urls_refuse_an_expired_or_unknown_link() {
    use entity::EntityTrait;

    let context =
        context::Context::mock_with_data_dir(Some("../data/test-links-manifest".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::seed_legacy_user(&context.db, "manifest@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": "manifest@doe.com", "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let create_file = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        content_tokens_root: None,
        content_tokens_file: None,
        name_hash: Some(helpers::name_tag("manifest")),
        mime: Some("text/plain".to_string()),
        size: Some(8),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        digest_tokens_root: None,
        digest_tokens_file: None,
        cipher: None,
        editable: None,
    };
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create_file)
        .to_request();
    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let payload = b"ciphertxt".to_vec();
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/storage/{}?checksum={}&chunk=0",
            file.id,
            cryptfns::sha256::digest(payload.as_slice())
        ))
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(payload)
        .to_request();
    test::call_service(&app, req).await;

    let link_key = cryptfns::aes::generate_key().unwrap();
    let link_key_hex = cryptfns::hex::encode(link_key.clone());
    let create_link = links::data::create_link::CreateLink {
        file_id: Some(file.id.to_string()),
        signature: Some(
            cryptfns::rsa::private::sign(file.id.to_string().as_str(), &owner.rsa_private).unwrap(),
        ),
        encrypted_name: Some(cryptfns::hex::encode(
            cryptfns::aes::encrypt(link_key.clone(), b"manifest.txt".to_vec()).unwrap(),
        )),
        encrypted_link_key: Some(
            cryptfns::rsa::public::encrypt(&link_key_hex, &owner.rsa_public).unwrap(),
        ),
        encrypted_thumbnail: None,
        encrypted_file_key: Some(cryptfns::hex::encode(
            cryptfns::aes::encrypt(link_key, b"file-key".to_vec()).unwrap(),
        )),
        // Already past — the route has to read the row before it decides.
        expires_at: Some(chrono::Utc::now().timestamp() - 60),
    };
    let req = test::TestRequest::post()
        .uri("/api/links")
        .cookie(jwt.clone())
        .set_json(create_link)
        .to_request();
    let link: AppLink =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/links/{}/chunk-urls", link.id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        actix_web::http::StatusCode::UNAUTHORIZED,
        "an expired link must not be handed URLs that outlive it by days"
    );

    let downloads = entity::links::Entity::find_by_id(link.id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap()
        .downloads;
    assert_eq!(
        downloads, 0,
        "a refused manifest must not count as a download"
    );

    let req = test::TestRequest::post()
        .uri(&format!("/api/links/{}/chunk-urls", entity::Uuid::new_v4()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::NOT_FOUND);

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}

/// A live link on a deployment with nothing to hand out is told so plainly.
/// The client then reads through the relaying route, which is what every
/// local-filesystem deployment has always done.
#[actix_web::test]
async fn test_link_chunk_urls_refuse_when_the_provider_has_no_urls() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-links-manifest-off".to_string()))
            .await;
    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::seed_legacy_user(&context.db, "manifest-off@doe.com").await;
    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": "manifest-off@doe.com", "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let create_file = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        content_tokens_root: None,
        content_tokens_file: None,
        name_hash: Some(helpers::name_tag("manifest-off")),
        mime: Some("text/plain".to_string()),
        size: Some(8),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        digest_tokens_root: None,
        digest_tokens_file: None,
        cipher: None,
        editable: None,
    };
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create_file)
        .to_request();
    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let payload = b"ciphertxt".to_vec();
    let req = test::TestRequest::post()
        .uri(&format!(
            "/api/storage/{}?checksum={}&chunk=0",
            file.id,
            cryptfns::sha256::digest(payload.as_slice())
        ))
        .cookie(jwt.clone())
        .append_header(("Content-Type", "application/octet-stream"))
        .set_payload(payload)
        .to_request();
    test::call_service(&app, req).await;

    let link_key = cryptfns::aes::generate_key().unwrap();
    let link_key_hex = cryptfns::hex::encode(link_key.clone());
    let create_link = links::data::create_link::CreateLink {
        file_id: Some(file.id.to_string()),
        signature: Some(
            cryptfns::rsa::private::sign(file.id.to_string().as_str(), &owner.rsa_private).unwrap(),
        ),
        encrypted_name: Some(cryptfns::hex::encode(
            cryptfns::aes::encrypt(link_key.clone(), b"live.txt".to_vec()).unwrap(),
        )),
        encrypted_link_key: Some(
            cryptfns::rsa::public::encrypt(&link_key_hex, &owner.rsa_public).unwrap(),
        ),
        encrypted_thumbnail: None,
        encrypted_file_key: Some(cryptfns::hex::encode(
            cryptfns::aes::encrypt(link_key, b"file-key".to_vec()).unwrap(),
        )),
        expires_at: None,
    };
    let req = test::TestRequest::post()
        .uri("/api/links")
        .cookie(jwt.clone())
        .set_json(create_link)
        .to_request();
    let link: AppLink =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/links/{}/chunk-urls", link.id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), actix_web::http::StatusCode::BAD_REQUEST);

    use fs::prelude::{Fs, FsProviderContract};
    Fs::new(&context.config).purge_all(&file).await.unwrap();
    context.config.app.cleanup();
}
