#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use hoodik::server;
use storage::data::app_file::AppFile;

/// `replaceContent` on a non-editable file is rejected with 400. The flag
/// is opt-in — callers have to set `editable = true` at create time (or
/// flip it later via `setEditable`) before the atomic-edit flow is allowed.
#[actix_web::test]
async fn test_replace_content_rejects_non_editable_file() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-replace-noeditable".to_string()))
            .await;

    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "noeditable@test.com")
        .await
        .jwt;

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("regular-file.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        name_hash: Some("hash".to_string()),
        mime: Some("text/plain".to_string()),
        size: Some(100),
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
        .set_json(&create)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(serde_json::json!({ "size": 10, "chunks": 1 }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    context.config.app.cleanup();
}

/// Directories are not editable content. `replaceContent` on a directory
/// is rejected with 400 before any pending-version bookkeeping happens.
#[actix_web::test]
async fn test_replace_content_rejects_directory() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-replace-dir".to_string())).await;

    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "dirtest@test.com")
        .await
        .jwt;

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("test-dir.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        name_hash: Some("dirhash".to_string()),
        mime: Some("dir".to_string()),
        size: None,
        chunks: None,
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
        .set_json(&create)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let dir: AppFile = serde_json::from_slice(&body).unwrap();

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", dir.id).as_str())
        .cookie(jwt.clone())
        .set_json(serde_json::json!({ "size": 10, "chunks": 1 }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    context.config.app.cleanup();
}

/// `size` and `chunks` must be positive. Validation runs before any
/// repository work, so a zero on either field surfaces as 422 with no
/// side effects on the file row.
#[actix_web::test]
async fn test_replace_content_validates_size_and_chunks() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-replace-validate".to_string()))
            .await;

    let app = test::init_service(server::app(context.clone())).await;

    let jwt = helpers::register_curve25519(&app, "validate@test.com")
        .await
        .jwt;

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("validate-note.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: None,
        search_tokens_file: None,
        name_hash: Some("valhash".to_string()),
        mime: Some("text/markdown".to_string()),
        size: Some(100),
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
        editable: Some(true),
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();

    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    for payload in [
        serde_json::json!({ "size": 0, "chunks": 1 }),
        serde_json::json!({ "size": 10, "chunks": 0 }),
    ] {
        let req = test::TestRequest::put()
            .uri(format!("/api/storage/{}/content", file.id).as_str())
            .cookie(jwt.clone())
            .set_json(&payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    context.config.app.cleanup();
}

/// A content save that carries no search tags leaves the file enrolled in the
/// owner's re-index sweep, with the content and digest tags it invalidated
/// cleared. Name tokens stay: they were never about the body.
///
/// This is the shape a client from before the keyed index sends: it has no
/// tags to offer, and `reindex` leaves a scope it was given nothing for
/// exactly as it was. Without the marker the note's text changes while its
/// index goes on describing the text it replaced — findable by words it no
/// longer contains, missing by the words it now does, and nothing anywhere
/// saying so. The empty `name_hash` is what the rekey migration uses to mean
/// "waiting for its owner's sweep", and the sweep rebuilds name and body
/// together.
#[actix_web::test]
async fn test_content_save_without_tags_enrols_the_file_for_reindex() {
    use entity::{file_tokens, files, ColumnTrait, EntityTrait, QueryFilter};

    let context =
        context::Context::mock_with_data_dir(Some("../data/test-replace-untagged".to_string()))
            .await;

    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "untagged@test.com")
        .await
        .jwt;

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: Some(vec!["aa11:1".to_string()]),
        search_tokens_file: Some(vec!["bb22:1".to_string()]),
        name_hash: Some("0123456789abcdef0123456789abcdef".to_string()),
        mime: Some("text/markdown".to_string()),
        size: Some(100),
        chunks: Some(1),
        file_id: None,
        file_modified_at: None,
        md5: None,
        sha1: None,
        sha256: None,
        blake2b: None,
        digest_tokens_root: Some(vec!["dd01:1".to_string()]),
        digest_tokens_file: Some(vec!["dd02:1".to_string()]),
        cipher: None,
        editable: Some(true),
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    let tags_before = file_tokens::Entity::find()
        .filter(file_tokens::Column::FileId.eq(file.id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(!tags_before.is_empty(), "the file starts out indexed");

    // The legacy shape: size and chunks, no tags.
    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(serde_json::json!({ "size": 20, "chunks": 1 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "the edit itself is not refused"
    );

    let row = files::Entity::find_by_id(file.id)
        .one(&context.db)
        .await
        .unwrap()
        .expect("the file still exists");
    assert_eq!(
        row.name_hash, "",
        "an empty name_hash is what enrols the file in the sweep"
    );

    // Content and digest tags that described the replaced text must not answer
    // for the new one. Name tokens stay: they were never about the body.
    let tags_after = file_tokens::Entity::find()
        .filter(file_tokens::Column::FileId.eq(file.id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(
        tags_after.iter().any(|t| t.tag == "aa11"),
        "name tokens must survive an untagged content save, got {:?}",
        tags_after
    );
    assert!(
        tags_after.iter().all(|t| {
            t.source == i32::from(entity::file_tokens::Source::Name)
                && t.tag != "dd01"
                && t.tag != "dd02"
        }),
        "content and digest tags that described the replaced text must not remain, got {:?}",
        tags_after
    );

    context.config.app.cleanup();
}

/// The same save with tags leaves the file indexed and out of the sweep —
/// the marker must not fire for an up-to-date client.
#[actix_web::test]
async fn test_content_save_with_tags_stays_indexed() {
    use entity::{file_tokens, files, ColumnTrait, EntityTrait, QueryFilter};

    let context =
        context::Context::mock_with_data_dir(Some("../data/test-replace-tagged".to_string())).await;

    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "tagged@test.com")
        .await
        .jwt;

    let create = storage::data::create_file::CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.enc".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: Some(vec!["aa11:1".to_string()]),
        search_tokens_file: Some(vec!["bb22:1".to_string()]),
        name_hash: Some("0123456789abcdef0123456789abcdef".to_string()),
        mime: Some("text/markdown".to_string()),
        size: Some(100),
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
        editable: Some(true),
    };

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create)
        .to_request();
    let body = test::call_and_read_body(&app, req).await;
    let file: AppFile = serde_json::from_slice(&body).unwrap();

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(serde_json::json!({
            "size": 20,
            "chunks": 1,
            "search_tokens_root": ["cc33:1"],
            "search_tokens_file": ["dd44:1"],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let row = files::Entity::find_by_id(file.id)
        .one(&context.db)
        .await
        .unwrap()
        .expect("the file still exists");
    assert_ne!(
        row.name_hash, "",
        "an up-to-date save stays out of the sweep"
    );

    let tags: Vec<file_tokens::Model> = file_tokens::Entity::find()
        .filter(file_tokens::Column::FileId.eq(file.id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(
        tags.iter()
            .any(|t| t.tag == "aa11" && t.source == i32::from(entity::file_tokens::Source::Name)),
        "name tokens must survive a tagged content save, got {:?}",
        tags
    );
    assert!(
        tags.iter().any(|t| t.tag == "cc33"
            && t.source == i32::from(entity::file_tokens::Source::Content)),
        "new content tags landed, got {:?}",
        tags
    );

    context.config.app.cleanup();
}
