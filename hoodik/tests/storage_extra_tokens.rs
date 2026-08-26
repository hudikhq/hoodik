#[path = "./helpers.rs"]
mod helpers;

use actix_web::{http::StatusCode, test};
use entity::{
    file_tokens::{self, Source},
    users, ColumnTrait, EntityTrait, QueryFilter,
};
use hoodik::server;
use storage::data::app_file::AppFile;
use storage::data::create_file::CreateFile;
use storage::data::extra_tokens::MAX_EXTRA_TOKENS;

fn create_payload(name_hash: &str, name_tag: &str) -> CreateFile {
    CreateFile {
        encrypted_key: Some("encrypted-key".to_string()),
        encrypted_name: Some("invoice.pdf".to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: Some(vec![format!("{name_tag}:1")]),
        search_tokens_file: Some(vec![format!("{name_tag}:1")]),
        content_tokens_root: None,
        content_tokens_file: None,
        name_hash: Some(name_hash.to_string()),
        mime: Some("application/pdf".to_string()),
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

async fn create_file(
    app: &impl helpers::TestApp,
    jwt: &actix_web::cookie::Cookie<'static>,
    name_hash: &str,
    name_tag: &str,
) -> AppFile {
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&create_payload(name_hash, name_tag))
        .to_request();
    let body = test::call_and_read_body(app, req).await;
    serde_json::from_slice(&body).unwrap()
}

async fn search(
    app: &impl helpers::TestApp,
    jwt: &actix_web::cookie::Cookie<'static>,
    tag: &str,
) -> Vec<AppFile> {
    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(jwt.clone())
        .set_json(serde_json::json!({ "root_tags": [tag] }))
        .to_request();
    let body = test::call_and_read_body(app, req).await;
    serde_json::from_slice(&body).unwrap()
}

fn ids(hits: &[AppFile]) -> Vec<entity::Uuid> {
    hits.iter().map(|f| f.id).collect()
}

#[actix_web::test]
async fn test_extra_tokens_are_additive_and_survive_rename() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "extra@test.com")
        .await
        .jwt;

    let name_tag = "aa11aa11aa11aa11aa11aa11aa11aa11";
    let extra_tag = "bb22bb22bb22bb22bb22bb22bb22bb22";
    let new_name_tag = "cc33cc33cc33cc33cc33cc33cc33cc33";
    let content_tag = "dd44dd44dd44dd44dd44dd44dd44dd44";

    let file = create_file(&app, &jwt, &helpers::name_tag("invoice.pdf"), name_tag).await;

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/extra-tokens", file.id))
        .cookie(jwt.clone())
        .set_json(serde_json::json!({
            "search_tokens_root": [format!("{extra_tag}:1")],
            "search_tokens_file": [format!("{extra_tag}:1")],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let hits = search(&app, &jwt, extra_tag).await;
    assert!(
        ids(&hits).contains(&file.id),
        "extra tag must find the file"
    );

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}", file.id))
        .cookie(jwt.clone())
        .set_json(serde_json::json!({
            "name_hash": helpers::name_tag("renamed.pdf"),
            "encrypted_name": "renamed.pdf",
            "search_tokens_root": [format!("{new_name_tag}:1")],
            "search_tokens_file": [format!("{new_name_tag}:1")],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    assert!(
        ids(&search(&app, &jwt, extra_tag).await).contains(&file.id),
        "extra tags must survive rename"
    );
    assert!(
        !ids(&search(&app, &jwt, name_tag).await).contains(&file.id),
        "old name tags must not survive rename"
    );
    assert!(
        ids(&search(&app, &jwt, new_name_tag).await).contains(&file.id),
        "new name tags must match"
    );

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/content", file.id))
        .cookie(jwt.clone())
        .set_json(serde_json::json!({
            "size": 20,
            "chunks": 1,
            "search_tokens_root": [format!("{content_tag}:1")],
            "search_tokens_file": [format!("{content_tag}:1")],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    assert!(
        ids(&search(&app, &jwt, extra_tag).await).contains(&file.id),
        "extra tags must survive a note save"
    );
    assert!(
        ids(&search(&app, &jwt, new_name_tag).await).contains(&file.id),
        "name tags must survive a note save"
    );

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/extra-tokens", file.id))
        .cookie(jwt.clone())
        .set_json(serde_json::json!({
            "search_tokens_root": [],
            "search_tokens_file": [],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    assert!(
        !ids(&search(&app, &jwt, extra_tag).await).contains(&file.id),
        "empty extra-tokens must clear extra"
    );
    assert!(
        ids(&search(&app, &jwt, new_name_tag).await).contains(&file.id),
        "clearing extra must leave the filename searchable"
    );
}

#[actix_web::test]
async fn test_create_content_tokens_survive_a_name_only_rename() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "content-create@test.com")
        .await
        .jwt;

    let name_tag = "aa11aa11aa11aa11aa11aa11aa11aa11";
    let content_tag = "bb22bb22bb22bb22bb22bb22bb22bb22";
    let new_name_tag = "cc33cc33cc33cc33cc33cc33cc33cc33";

    let mut payload = create_payload(&helpers::name_tag("note.md"), name_tag);
    payload.mime = Some("text/markdown".to_string());
    payload.editable = Some(true);
    payload.content_tokens_root = Some(vec![format!("{content_tag}:1")]);
    payload.content_tokens_file = Some(vec![format!("{content_tag}:1")]);

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&payload)
        .to_request();
    let file: AppFile = serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let rows = file_tokens::Entity::find()
        .filter(file_tokens::Column::FileId.eq(file.id))
        .all(&context.db)
        .await
        .unwrap();
    let source_of = |tag: &str| {
        rows.iter()
            .find(|r| r.tag == tag)
            .map(|r| r.source)
            .expect(tag)
    };
    assert_eq!(source_of(name_tag), i32::from(Source::Name));
    assert_eq!(source_of(content_tag), i32::from(Source::Content));

    assert!(ids(&search(&app, &jwt, name_tag).await).contains(&file.id));
    assert!(ids(&search(&app, &jwt, content_tag).await).contains(&file.id));

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}", file.id))
        .cookie(jwt.clone())
        .set_json(serde_json::json!({
            "name_hash": helpers::name_tag("renamed.md"),
            "encrypted_name": "renamed.md",
            "search_tokens_root": [format!("{new_name_tag}:1")],
            "search_tokens_file": [format!("{new_name_tag}:1")],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    assert!(
        ids(&search(&app, &jwt, content_tag).await).contains(&file.id),
        "body tags must survive a rename that only sends the new name"
    );
    assert!(!ids(&search(&app, &jwt, name_tag).await).contains(&file.id));
    assert!(ids(&search(&app, &jwt, new_name_tag).await).contains(&file.id));
}

#[actix_web::test]
async fn test_reindex_writes_content_tokens_to_the_content_source() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;
    let registered = helpers::register_curve25519(&app, "content-reindex@test.com").await;
    let jwt = registered.jwt;

    let name_tag = "aa11aa11aa11aa11aa11aa11aa11aa11";
    let old_content = "bb22bb22bb22bb22bb22bb22bb22bb22";
    let new_name = "cc33cc33cc33cc33cc33cc33cc33cc33";
    let new_content = "dd44dd44dd44dd44dd44dd44dd44dd44";

    let mut payload = create_payload(&helpers::name_tag("note.md"), name_tag);
    payload.mime = Some("text/markdown".to_string());
    payload.editable = Some(true);
    payload.content_tokens_root = Some(vec![format!("{old_content}:1")]);
    payload.content_tokens_file = Some(vec![format!("{old_content}:1")]);

    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(jwt.clone())
        .set_json(&payload)
        .to_request();
    let file: AppFile = serde_json::from_slice(&test::call_and_read_body(&app, req).await).unwrap();

    let fingerprint = users::Entity::find_by_id(registered.user_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap()
        .fingerprint;

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/reindex", file.id))
        .cookie(jwt.clone())
        .set_json(serde_json::json!({
            "name_hash": helpers::name_tag("note.md"),
            "fingerprint": fingerprint,
            "search_tokens_root": [format!("{new_name}:1")],
            "search_tokens_file": [format!("{new_name}:1")],
            "content_tokens_root": [format!("{new_content}:1")],
            "content_tokens_file": [format!("{new_content}:1")],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let rows = file_tokens::Entity::find()
        .filter(file_tokens::Column::FileId.eq(file.id))
        .all(&context.db)
        .await
        .unwrap();
    let source_of = |tag: &str| {
        rows.iter()
            .find(|r| r.tag == tag)
            .map(|r| r.source)
            .expect(tag)
    };
    assert_eq!(source_of(new_name), i32::from(Source::Name));
    assert_eq!(source_of(new_content), i32::from(Source::Content));
    assert!(rows
        .iter()
        .all(|r| r.tag != name_tag && r.tag != old_content));

    assert!(ids(&search(&app, &jwt, new_name).await).contains(&file.id));
    assert!(ids(&search(&app, &jwt, new_content).await).contains(&file.id));
    assert!(!ids(&search(&app, &jwt, name_tag).await).contains(&file.id));
    assert!(!ids(&search(&app, &jwt, old_content).await).contains(&file.id));
}

#[actix_web::test]
async fn test_extra_tokens_rejects_over_the_cap() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "cap@test.com").await.jwt;
    let file = create_file(
        &app,
        &jwt,
        &helpers::name_tag("cap.pdf"),
        "aa11aa11aa11aa11aa11aa11aa11aa11",
    )
    .await;

    let too_many: Vec<String> = (0..=MAX_EXTRA_TOKENS)
        .map(|i| format!("{i:032x}:1"))
        .collect();
    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/extra-tokens", file.id))
        .cookie(jwt.clone())
        .set_json(serde_json::json!({ "search_tokens_hashed": too_many }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_extra_tokens_requires_write_access() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;
    let jwt = helpers::register_curve25519(&app, "owner@test.com")
        .await
        .jwt;
    let other = helpers::register_curve25519(&app, "other@test.com")
        .await
        .jwt;
    let file = create_file(
        &app,
        &jwt,
        &helpers::name_tag("secret.pdf"),
        "aa11aa11aa11aa11aa11aa11aa11aa11",
    )
    .await;

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/extra-tokens", file.id))
        .set_json(serde_json::json!({ "search_tokens_root": ["ee55:1"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/extra-tokens", file.id))
        .cookie(other)
        .set_json(serde_json::json!({ "search_tokens_root": ["ee55:1"] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
