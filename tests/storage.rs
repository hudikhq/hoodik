use actix_web::{http::StatusCode, test};
use auth::data::{authenticated::AuthenticatedJwt, create_user::CreateUser};
use hoodik::server;
use storage::data::app_file::AppFile;

fn create_byte_chunks() -> (Vec<Vec<u8>>, i64, String) {
    let one_chunk_size = storage::CHUNK_SIZE_BYTES as usize;
    let mut byte_chunks = vec![];
    let mut body = vec![];

    while body.len() < (one_chunk_size * 5) {
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

#[actix_web::test]
async fn test_creating_file_and_uploading_chunks() {
    let context = context::Context::mock_sqlite().await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let _private_string = cryptfns::rsa::private::to_string(&private).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let encrypted_secret = "some-random-encrypted-secret".to_string();

    let mut app = test::init_service(server::app(context.clone())).await;

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("john@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
        })
        .to_request();

    let body = test::call_and_read_body(&mut app, req).await;
    let authenticated_jwt: AuthenticatedJwt = serde_json::from_slice(&body).unwrap();
    let jwt = format!("Bearer {}", authenticated_jwt.jwt.clone());
    let csrf = authenticated_jwt.authenticated.session.csrf.clone();

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&CreateUser {
            email: Some("john2@doe.com".to_string()),
            password: Some("not-4-weak-password-for-god-sakes!".to_string()),
            secret: None,
            token: None,
            pubkey: Some(public_string.clone()),
            fingerprint: Some(fingerprint.clone()),
            encrypted_private_key: Some(encrypted_secret.clone()),
        })
        .to_request();

    let body = test::call_and_read_body(&mut app, req).await;
    let authenticated_jwt: AuthenticatedJwt = serde_json::from_slice(&body).unwrap();
    let jwt1 = format!("Bearer {}", authenticated_jwt.jwt.clone());
    let csrf1 = authenticated_jwt.authenticated.session.csrf.clone();

    let (data, size, checksum) = create_byte_chunks();
    assert_eq!(
        data.len(),
        size as usize / storage::CHUNK_SIZE_BYTES as usize
    );

    let random_file = storage::data::create_file::CreateFile {
        encrypted_metadata: Some("encrypted-gibberish".to_string()),
        search_tokens_hashed: None,
        name_hash: Some(checksum.clone()),
        mime: Some("text/plain".to_string()),
        size: Some(size),
        chunks: Some(data.len() as i32),
        file_id: None,
        /// Date of the file creation from the disk, if not provided we set it to now
        file_created_at: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .append_header(("Authorization", jwt.clone()))
        .append_header(("X-CSRF-Token", csrf.clone()))
        .set_json(&random_file)
        .to_request();

    let body = test::call_and_read_body(&mut app, req).await;
    // let string_body = String::from_utf8(body.to_vec()).unwrap();
    // println!("string_body: {}", string_body);

    let mut file: AppFile = serde_json::from_slice(&body).unwrap();

    // println!("file: {:#?}", file);

    let mut uploaded = vec![];
    for (i, chunk) in data.into_iter().enumerate() {
        let checksum = cryptfns::sha256::digest(chunk.as_slice());
        let req = test::TestRequest::post()
            .uri(
                format!(
                    "/api/storage/{}?chunk={}&checksum={}",
                    &file.id, i, checksum
                )
                .as_str(),
            )
            .append_header(("Content-Type", "application/octet-stream"))
            .append_header(("Authorization", jwt.clone()))
            .append_header(("X-CSRF-Token", csrf.clone()))
            .set_payload(chunk)
            .to_request();

        let body = test::call_and_read_body(&mut app, req).await;
        // let string_body = String::from_utf8(body.to_vec()).unwrap();
        // println!("string_body: {}", string_body);
        file = serde_json::from_slice(&body).unwrap();
        uploaded.push(i as i32);

        assert_eq!(file.uploaded_chunks.clone().unwrap(), uploaded);
        assert_eq!(file.chunks_stored.unwrap(), i as i32 + 1);
    }

    assert!(file.finished_upload_at.is_some());

    let filename = file.get_filename().unwrap();

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", &file.id).as_str())
        .append_header(("Authorization", jwt.clone()))
        .append_header(("X-CSRF-Token", csrf.clone()))
        .to_request();

    let contents = test::call_and_read_body(&mut app, req).await.to_vec();

    let content_len = contents.len();
    let file_checksum = cryptfns::sha256::digest(contents.as_slice());

    assert!(std::fs::remove_file(format!("{}/{}", context.config.data_dir, filename)).is_ok());

    assert!(
        std::fs::remove_file(format!("{}/{}.0.part", context.config.data_dir, filename)).is_err()
    );
    assert!(
        std::fs::remove_file(format!("{}/{}.1.part", context.config.data_dir, filename)).is_err()
    );
    assert!(
        std::fs::remove_file(format!("{}/{}.2.part", context.config.data_dir, filename)).is_err()
    );
    assert!(
        std::fs::remove_file(format!("{}/{}.3.part", context.config.data_dir, filename)).is_err()
    );
    assert!(
        std::fs::remove_file(format!("{}/{}.4.part", context.config.data_dir, filename)).is_err()
    );

    assert_eq!(content_len, size as usize);
    assert_eq!(file_checksum, checksum);

    // Other user cannot see the file metadata
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/metadata", &file.id).as_str())
        .append_header(("Authorization", jwt1.clone()))
        .append_header(("X-CSRF-Token", csrf1.clone()))
        .set_json(&random_file)
        .to_request();

    let response = test::call_service(&mut app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Owner can see it
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/metadata", &file.id).as_str())
        .append_header(("Authorization", jwt.clone()))
        .append_header(("X-CSRF-Token", csrf.clone()))
        .set_json(&random_file)
        .to_request();

    let response = test::call_service(&mut app, req).await;
    assert_eq!(response.status(), StatusCode::OK);
}
