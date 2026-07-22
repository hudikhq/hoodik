//! Listings carry full rows by default so older clients keep working.
//! A client that passes `compact=true` gets rows without the thumbnail
//! blob — the column is never read off the page, and a SQL-computed
//! `has_thumbnail` tells the client to fetch it lazily instead, per file
//! (thumbnail route) or per link (metadata route). These tests pin the
//! default, the compact projection, and the lazy routes.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use hoodik::server;
use links::data::app_link::AppLink;
use serde_json::json;
use storage::data::{app_file::AppFile, response::Response};

const THUMBNAIL: &str = "encrypted-thumbnail-gibberish";

fn create_file_with_thumbnail(name_hash: &str) -> storage::data::create_file::CreateFile {
    storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-gibberish".to_string()),
        encrypted_name: Some("name".to_string()),
        encrypted_thumbnail: Some(THUMBNAIL.to_string()),
        search_tokens_hashed: None,
        name_hash: Some(name_hash.to_string()),
        mime: Some("image/jpeg".to_string()),
        size: Some(100),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        cipher: None,
        editable: None,
    }
}

#[actix_web::test]
async fn test_storage_compact_listing_and_thumbnail_route() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-thumbnails".to_string())).await;

    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "john@doe.com").await.jwt;
    let second_jwt = helpers::register_curve25519(&app, "jane@doe.com").await.jwt;

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(create_file_with_thumbnail("hash-1"))
        .to_request();

    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    assert!(file.has_thumbnail);

    // Without `compact` the listing ships full rows — the compatible
    // default for clients that predate the parameter.
    let req = test::TestRequest::get()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .to_request();
    let listing: Response =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    let listed = listing
        .children
        .iter()
        .find(|child| child.id == file.id)
        .unwrap();
    assert_eq!(listed.encrypted_thumbnail.as_deref(), Some(THUMBNAIL));
    assert!(listed.has_thumbnail);

    // `compact` withholds the blob and reports the SQL-computed flag,
    // leaving every other field in place.
    let req = test::TestRequest::get()
        .uri("/api/storage?compact=true")
        .cookie(jwt.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    assert!(!String::from_utf8(body.to_vec())
        .unwrap()
        .contains(THUMBNAIL));

    let listing: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let listed = listing["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["id"] == json!(file.id))
        .unwrap();
    assert_eq!(listed["has_thumbnail"], json!(true));
    assert!(listed.get("encrypted_thumbnail").is_none());
    assert_eq!(listed["encrypted_name"], json!("name"));
    assert_eq!(listed["mime"], json!("image/jpeg"));

    // A file without a thumbnail reports the flag as false.
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(storage::data::create_file::CreateFile {
            encrypted_thumbnail: None,
            name_hash: Some("hash-no-thumb".to_string()),
            ..create_file_with_thumbnail("unused")
        })
        .to_request();
    let plain: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    assert!(!plain.has_thumbnail);

    let req = test::TestRequest::get()
        .uri("/api/storage?compact=true")
        .cookie(jwt.clone())
        .to_request();
    let listing: serde_json::Value =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    let listed = listing["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|child| child["id"] == json!(plain.id))
        .unwrap();
    assert_eq!(listed["has_thumbnail"], json!(false));

    // The thumbnail route serves the blob to anyone with a user_files row.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/thumbnail", &file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let thumbnail: serde_json::Value =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    assert_eq!(thumbnail["encrypted_thumbnail"], json!(THUMBNAIL));

    // ... and stays hidden from users without one.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/thumbnail", &file.id).as_str())
        .cookie(second_jwt)
        .to_request();
    let response = test::call_service(&app, req).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Single-file metadata keeps the blob inline.
    let req = test::TestRequest::get()
        .uri(format!("/api/storage/{}/metadata", &file.id).as_str())
        .cookie(jwt.clone())
        .to_request();
    let metadata: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    assert_eq!(metadata.encrypted_thumbnail.as_deref(), Some(THUMBNAIL));

    context.config.app.cleanup();
}

#[actix_web::test]
async fn test_links_compact_listing_and_metadata_blob() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-thumbnails-links".to_string()))
            .await;

    let app = test::init_service(server::app(context.clone())).await;

    let owner = helpers::seed_legacy_user(&context.db, "john@doe.com").await;

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(json!({ "email": "john@doe.com", "password": helpers::LEGACY_PASSWORD }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let (jwt, _) = helpers::extract_cookies(resp.headers());
    let jwt = jwt.unwrap();

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(create_file_with_thumbnail("hash-2"))
        .to_request();
    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let signature =
        cryptfns::rsa::private::sign(file.id.to_string().as_str(), &owner.rsa_private).unwrap();
    let link_key_rsa_enc =
        cryptfns::rsa::public::encrypt("link-key-hex", &owner.rsa_public).unwrap();

    let create_link = links::data::create_link::CreateLink {
        file_id: Some(file.id.to_string()),
        signature: Some(signature),
        encrypted_name: Some("encrypted-link-name".to_string()),
        encrypted_link_key: Some(link_key_rsa_enc),
        encrypted_thumbnail: Some(THUMBNAIL.to_string()),
        encrypted_file_key: Some("encrypted-file-key".to_string()),
        expires_at: None,
    };
    let req = test::TestRequest::post()
        .uri("/api/links")
        .cookie(jwt.clone())
        .set_json(create_link)
        .to_request();
    let link: AppLink =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    assert!(link.has_thumbnail);

    // Without `compact` the owner's listing ships full rows.
    let req = test::TestRequest::get()
        .uri("/api/links")
        .cookie(jwt.clone())
        .to_request();
    let listing: Vec<AppLink> =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    let listed = listing.iter().find(|row| row.id == link.id).unwrap();
    assert_eq!(listed.encrypted_thumbnail.as_deref(), Some(THUMBNAIL));
    assert!(listed.has_thumbnail);

    // `compact` withholds the blob and reports the SQL-computed flag.
    let req = test::TestRequest::get()
        .uri("/api/links?compact=true")
        .cookie(jwt.clone())
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    assert!(!String::from_utf8(body.to_vec())
        .unwrap()
        .contains(THUMBNAIL));

    let listing: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let listed = listing
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == json!(link.id))
        .unwrap();
    assert_eq!(listed["has_thumbnail"], json!(true));
    assert!(listed.get("encrypted_thumbnail").is_none());

    // Public single-link metadata keeps the blob inline.
    let req = test::TestRequest::get()
        .uri(format!("/api/links/{}/metadata", &link.id).as_str())
        .to_request();
    let metadata: AppLink =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    assert_eq!(metadata.encrypted_thumbnail.as_deref(), Some(THUMBNAIL));

    context.config.app.cleanup();
}

/// `compact` on the search route. Search reaches the projection through
/// a different query builder than the listing — it joins the token index
/// and aggregates — so the flag needs its own coverage. Drives the route
/// the way current clients do: hashed tokens in, no plaintext term.
#[actix_web::test]
async fn test_search_compact_withholds_thumbnail_blob() {
    let context =
        context::Context::mock_with_data_dir(Some("../data-test-thumbnails-search".to_string()))
            .await;

    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "john@doe.com").await.jwt;

    let hashed: Vec<String> = cryptfns::tokenizer::into_hashed_tokens("octopus")
        .expect("tokenize search word")
        .into_iter()
        .map(|t| format!("{}:{}", t.token, t.weight))
        .collect();

    let mut payload = create_file_with_thumbnail("searchable-thumb");
    payload.search_tokens_hashed = Some(hashed.clone());

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&payload)
        .to_request();
    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    assert!(file.has_thumbnail);

    // Default: the blob rides along, as it did before the flag existed.
    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(jwt.clone())
        .set_json(json!({ "search_tokens_hashed": hashed }))
        .to_request();
    let hits: Vec<AppFile> =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();
    let hit = hits.iter().find(|f| f.id == file.id).unwrap();
    assert_eq!(hit.encrypted_thumbnail.as_deref(), Some(THUMBNAIL));

    // `compact`: same hit, no blob, flag still true.
    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(jwt.clone())
        .set_json(json!({ "search_tokens_hashed": hashed, "compact": true }))
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    assert!(!String::from_utf8(body.to_vec())
        .unwrap()
        .contains(THUMBNAIL));

    let hits: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let hit = hits
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == serde_json::json!(file.id))
        .expect("compact search should still return the file");
    assert_eq!(hit["has_thumbnail"], json!(true));
    assert!(hit["encrypted_thumbnail"].is_null());
}
