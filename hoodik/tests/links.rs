#[path = "./helpers.rs"]
mod helpers;

use actix_web::test;
use auth::data::create_user::CreateUser;
use hoodik::server;
use links::data::app_link::AppLink;
use storage::data::app_file::AppFile;

use crate::helpers::{create_byte_chunks, CHUNK_SIZE_BYTES};

#[actix_web::test]
async fn test_creating_and_downloading_link() {
    let context = context::Context::mock_with_data_dir(Some("../data-test".to_string())).await;

    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let public_string = cryptfns::rsa::public::to_string(&public).unwrap();
    let private_string = cryptfns::rsa::private::to_string(&private).unwrap();
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
            invitation_id: None,
        })
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    let (jwt, _) = helpers::extract_cookies(&resp.headers());
    let jwt = jwt.unwrap();

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
    let file_key = cryptfns::aes::generate_key().unwrap();
    let file_key_hex = cryptfns::hex::encode(file_key.clone());

    // println!("file: {:#?}", file);

    let mut uploaded = vec![];
    for (i, chunk) in data.into_iter().enumerate() {
        println!("chunk: {}", i);
        // println!("chunk: {}", i);
        let checksum = cryptfns::sha256::digest(chunk.as_slice());
        let uri = format!(
            "/api/storage/{}?checksum={}&chunk={}&key_hex={}",
            &file.id, checksum, i, &file_key_hex
        );

        let req = test::TestRequest::post()
            .uri(uri.as_str())
            .cookie(jwt.clone())
            .append_header(("Content-Type", "application/octet-stream"))
            .set_payload(chunk)
            .to_request();

        let body = test::call_and_read_body(&mut app, req).await;
        let string_body = String::from_utf8(body.to_vec()).unwrap();
        println!("string_body: {}", string_body);
        file = serde_json::from_slice(&body).unwrap();
        uploaded.push(i as i32);

        assert_eq!(file.uploaded_chunks.clone().unwrap(), uploaded);
        assert_eq!(file.chunks_stored.unwrap(), i as i32 + 1);
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

    let body = test::call_and_read_body(&mut app, req).await;
    let link: AppLink = serde_json::from_slice(&body).unwrap();

    let download_linked_file = links::data::download::Download {
        link_key: Some(link_key_hex),
    };
    let uri = format!("/api/links/{}", link.id);
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_json(download_linked_file)
        // .cookie(jwt.clone()) - no need for jwt, this should be public
        .to_request();

    let contents = test::call_and_read_body(&mut app, req).await.to_vec();
    // let string_body = String::from_utf8(contents.to_vec()).unwrap();
    // println!("string_body: {}", string_body);

    let content_len = contents.len();
    let file_checksum = cryptfns::sha256::digest(contents.as_slice());

    assert_eq!(content_len, size as usize);
    assert_eq!(file_checksum, checksum);

    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/metadata", &file.id).as_str())
        .cookie(jwt)
        .set_json(&random_file)
        .to_request();

    let file =
        serde_json::from_slice::<AppFile>(&test::call_and_read_body(&mut app, req).await).unwrap();

    // println!("file: {:#?}", file);

    assert!(file.link.is_some());
    assert_eq!(file.link.unwrap().id, link.id);

    context.config.app.cleanup();
}
