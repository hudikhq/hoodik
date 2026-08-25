//! `PUT /api/storage/{file_id}/content-tokens`.
//!
//! The route a client calls after restoring a note. A restore names a version
//! and carries no body, and the server holds only ciphertext, so it cannot
//! derive the tags for the text it just restored — it clears them and enrols
//! the file in the owner's sweep. This is how a client that has decrypted the
//! restored version puts the index back without waiting for that sweep.

#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use entity::{file_tokens, ColumnTrait, EntityTrait, QueryFilter};
use hoodik::server;
use storage::data::app_file::AppFile;
use storage::data::create_file::CreateFile;

fn create_payload(name_hash: &str) -> CreateFile {
    CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("note.md".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: Some(vec!["name11:1".to_string()]),
        search_tokens_file: Some(vec!["name22:1".to_string()]),
        content_tokens_root: Some(vec!["old11:1".to_string()]),
        content_tokens_file: Some(vec!["old22:1".to_string()]),
        name_hash: Some(name_hash.to_string()),
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
    }
}

async fn tags_by_source(context: &context::Context, file_id: entity::Uuid) -> Vec<(i32, String)> {
    file_tokens::Entity::find()
        .filter(file_tokens::Column::FileId.eq(file_id))
        .all(&context.db)
        .await
        .unwrap()
        .into_iter()
        .map(|t| (t.source, t.tag))
        .collect()
}

#[actix_web::test]
async fn test_content_tokens_replace_only_the_content_source() {
    let context =
        context::Context::mock_with_data_dir(Some("../data/test-content-tokens".to_string())).await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "content@test.com").await.jwt;

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create_payload("0123456789abcdef0123456789abcdef"))
        .to_request();
    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let name = i32::from(file_tokens::Source::Name);
    let content = i32::from(file_tokens::Source::Content);

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content-tokens", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(serde_json::json!({
            "content_tokens_root": ["new11:1"],
            "content_tokens_file": ["new22:1"],
        }))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);

    let tags = tags_by_source(&context, file.id).await;

    // The body the client just decrypted, in place of the one it replaced.
    assert!(
        tags.contains(&(content, "new11".to_string())),
        "the restored body must be indexed, got {:?}",
        tags
    );
    assert!(
        !tags.iter().any(|(s, t)| *s == content && t == "old11"),
        "the previous body's tags must not survive, got {:?}",
        tags
    );

    // The name did not change, and this route must not touch it. Writing the
    // body through a route that replaced everything would trade one stale
    // index for another.
    assert!(
        tags.contains(&(name, "name11".to_string())),
        "the name source is not this route's business, got {:?}",
        tags
    );
}

#[actix_web::test]
async fn test_content_tokens_are_searchable_immediately() {
    let context = context::Context::mock_with_data_dir(Some(
        "../data/test-content-tokens-search".to_string(),
    ))
    .await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "csearch@test.com").await.jwt;

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create_payload("fedcba9876543210fedcba9876543210"))
        .to_request();
    let file: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let req = test::TestRequest::put()
        .uri(format!("/api/storage/{}/content-tokens", file.id).as_str())
        .cookie(jwt.clone())
        .set_json(serde_json::json!({
            "content_tokens_root": ["fresh1:1"],
            "content_tokens_file": ["fresh2:1"],
        }))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), StatusCode::OK);

    // The whole point of the route: no sweep in between.
    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(jwt.clone())
        .set_json(serde_json::json!({ "root_tags": ["fresh1"] }))
        .to_request();
    let found: Vec<AppFile> =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    assert_eq!(found.len(), 1, "the restored body should be findable at once");
    assert_eq!(found[0].id, file.id);

    context.config.app.cleanup();
}
