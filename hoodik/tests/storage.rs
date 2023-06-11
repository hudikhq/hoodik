#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use auth::data::create_user::CreateUser;
use fs::IntoFilename;
use hoodik::server;
use storage::data::app_file::AppFile;

use crate::helpers::{create_byte_chunks, CHUNKS, CHUNK_SIZE_BYTES};

#[actix_web::test]
async fn test_creating_file_and_uploading_chunks() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;

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

    let resp = test::call_service(&mut app, req).await;
    let (jwt, _) = helpers::extract_cookies(&resp.headers());
    let jwt = jwt.unwrap();

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

    let resp = test::call_service(&mut app, req).await;
    let (second_jwt, _) = helpers::extract_cookies(&resp.headers());
    let second_jwt = second_jwt.unwrap();

    let (data, size, checksum) = create_byte_chunks();
    assert_eq!(data.len(), size as usize / CHUNK_SIZE_BYTES as usize);

    let random_file = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: None,
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
        .cookie(jwt.clone())
        .set_json(&random_file)
        .to_request();

    let body = test::call_and_read_body(&mut app, req).await;
    // let string_body = String::from_utf8(body.to_vec()).unwrap();
    // println!("string_body: {}", string_body);

    let mut file: AppFile = serde_json::from_slice(&body).unwrap();

    // println!("file: {:#?}", file);

    let mut uploaded = vec![];
    for (i, chunk) in data.into_iter().enumerate() {
        println!("chunk: {}", i);
        // println!("chunk: {}", i);
        let checksum = cryptfns::sha256::digest(chunk.as_slice());
        let uri = format!(
            "/api/storage/{}?checksum={}&chunk={}",
            &file.id, checksum, i
        );

        let req = test::TestRequest::post()
            .uri(uri.as_str())
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
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

    let filename = file.filename().unwrap();

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}", &file.id).as_str())
        .cookie(jwt.clone())
        .to_request();

    let contents = test::call_and_read_body(&mut app, req).await.to_vec();

    let content_len = contents.len();
    let file_checksum = cryptfns::sha256::digest(contents.as_slice());

    for i in 0..CHUNKS {
        let f = format!(
            "{}/{}",
            context.config.app.data_dir,
            filename.clone().with_chunk(i as i32)
        );

        println!("removing file: {}", f);
        assert!(std::fs::remove_file(f).is_ok());
    }

    assert_eq!(content_len, size as usize);
    assert_eq!(file_checksum, checksum);

    // Other user cannot see the file metadata
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/metadata", &file.id).as_str())
        .cookie(second_jwt)
        .set_json(&random_file)
        .to_request();

    let response = test::call_service(&mut app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Owner can see it
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/metadata", &file.id).as_str())
        .cookie(jwt.clone())
        .set_json(&random_file)
        .to_request();

    let response = test::call_service(&mut app, req).await;
    assert_eq!(response.status(), StatusCode::OK);

    context.config.app.cleanup();
}
