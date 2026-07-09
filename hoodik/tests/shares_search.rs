//! Search index access for shared content.
//!
//! `POST /api/storage/search` runs the same tokenized-hash query that
//! has always backed the search box; the only sharing-era change is
//! that the underlying join no longer filters `user_files.is_owner =
//! true`. The tests below feed the create route a known set of hashed
//! tokens, share the file across roles, and confirm:
//!
//! - the owner still finds their own file
//! - every recipient (Reader / Editor / Co-owner) finds the shared file
//! - an unrelated user does NOT find it, even with the same query
//!
//! Tokens go in through the real `POST /api/storage` route and come
//! out through the real `POST /api/storage/search` route — no DB
//! shortcuts, no mocking the search query.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::ShareRoleEnum;
use cryptfns::tokenizer::into_hashed_tokens;
use hoodik::server;
use storage::data::app_file::AppFile;
use storage::data::create_file::CreateFile;

use crate::shares_common::*;

/// Pick a search term whose BERT tokenization survives intact (single
/// token, weight 1) so the test can drive the search box with the
/// same word that seeded the index. "octopus" tokenizes to itself
/// cleanly under bert-base-cased.
const SEARCH_WORD: &str = "octopus";

/// Build a `CreateFile` payload seeded with the hashed tokens of
/// `SEARCH_WORD`. The repository accepts `["{sha256-hex}:{weight}",
/// ...]` (parsed by `cryptfns::tokenizer::from_vec`), so we project
/// the `Token` list through the same `"{token}:{weight}"` shape the
/// browser sends.
fn make_searchable_file(public_pem: &str, name_hash: &str) -> CreateFile {
    let hashed = into_hashed_tokens(SEARCH_WORD).expect("tokenize search word");
    assert!(
        !hashed.is_empty(),
        "tokenizer returned no tokens for {SEARCH_WORD}; pick a different word"
    );
    let mut payload = make_create_file(public_pem, name_hash);
    payload.search_tokens_hashed = Some(
        hashed
            .into_iter()
            .map(|t| format!("{}:{}", t.token, t.weight))
            .collect(),
    );
    payload
}

/// Drive `POST /api/storage/search` with `SEARCH_WORD` as the caller
/// and return the deserialised result list.
macro_rules! search_for_word {
    ($app:expr, $caller:expr) => {{
        let req = actix_web::test::TestRequest::post()
            .uri("/api/storage/search")
            .cookie($caller.jwt.clone())
            .set_json(serde_json::json!({ "search": SEARCH_WORD }))
            .to_request();
        let resp = actix_web::test::call_service(&$app, req).await;
        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::OK,
            "search returned non-200 for {}: {:?}",
            $caller.email,
            resp.status()
        );
        let body = actix_web::test::read_body(resp).await;
        serde_json::from_slice::<Vec<storage::data::app_file::AppFile>>(&body)
            .expect("search response is a Vec<AppFile>")
    }};
}

/// Insert a file owned by `$user` and seeded with `SEARCH_WORD`
/// tokens. Uses the same `POST /api/storage` path the browser hits;
/// the route forwards `search_tokens_hashed` into the token index
/// (see `storage::repository::manage::create`).
macro_rules! create_searchable_file {
    ($app:expr, $user:expr, $name_hash:expr) => {{
        let payload = make_searchable_file(&$user.public_pem, $name_hash);
        let req = actix_web::test::TestRequest::post()
            .uri("/api/storage")
            .cookie($user.jwt.clone())
            .set_json(&payload)
            .to_request();
        let body = actix_web::test::call_and_read_body(&$app, req).await;
        serde_json::from_slice::<AppFile>(&body).expect("create_searchable_file json")
    }};
}

/// The contract: a recipient at every role finds the
/// shared file when searching by a tokenized word. Owners keep
/// finding their own file (regression check on the pre-existing
/// path), and an unrelated user finds nothing despite issuing the
/// same query.
#[actix_web::test]
async fn test_search_finds_shared_file_for_every_recipient_role_but_not_strangers() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, reader, "reader@example.com");
    register_user!(app, context, editor, "editor@example.com");
    register_user!(app, context, co_owner, "co_owner@example.com");
    register_user!(app, context, stranger, "stranger@example.com");

    let file = create_searchable_file!(app, alice, "octopus-note");

    grant!(app, alice, reader, ShareRoleEnum::Reader, file.id);
    grant!(app, alice, editor, ShareRoleEnum::Editor, file.id);
    grant!(app, alice, co_owner, ShareRoleEnum::CoOwner, file.id);

    // Owner still finds their own file via the existing search code
    // path. A drop here would mean the Postgres GROUP BY rework broke
    // owner search; protect against the regression.
    let alice_hits = search_for_word!(app, alice);
    assert!(
        alice_hits.iter().any(|f| f.id == file.id && f.is_owner),
        "owner alice should still find her own file via search; got {:?}",
        alice_hits.iter().map(|f| (f.id, f.is_owner)).collect::<Vec<_>>()
    );

    for recipient in [&reader, &editor, &co_owner] {
        let hits = search_for_word!(app, *recipient);
        let hit = hits.iter().find(|f| f.id == file.id);
        assert!(
            hit.is_some(),
            "recipient {} (joined via user_files) should find the shared file; got {:?}",
            recipient.email,
            hits.iter().map(|f| f.id).collect::<Vec<_>>()
        );
        assert!(
            !hit.unwrap().is_owner,
            "recipient {} should see is_owner=false on the shared row",
            recipient.email
        );
    }

    // Negation: a user with no `user_files` row for `file.id` must
    // not get a hit even though they issue the exact same query the
    // recipients used. This is the access predicate the join still
    // enforces — only the `is_owner=true` half was dropped, not the
    // `user_files.user_id = current_user` half.
    let stranger_hits = search_for_word!(app, stranger);
    assert!(
        stranger_hits.iter().all(|f| f.id != file.id),
        "stranger must not see the file via search; got {:?}",
        stranger_hits.iter().map(|f| f.id).collect::<Vec<_>>()
    );

    let _ = StatusCode::OK; // suppress unused-import lint on debug only
    let _ = context;
}

/// After a revoke, the recipient's `user_files` row is gone and the
/// search join can no longer reach the file from their side. The
/// owner keeps finding it. This isolates the access predicate from
/// the token-index predicate — revoke does not touch tokens, only
/// the per-user row.
#[actix_web::test]
async fn test_search_stops_returning_shared_file_after_revoke() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");

    let file = create_searchable_file!(app, alice, "octopus-note");
    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let pre_revoke = search_for_word!(app, bob);
    assert!(
        pre_revoke.iter().any(|f| f.id == file.id),
        "bob should find the shared file before revoke"
    );

    let revoke_body = build_revoke_body(&alice, &bob, file.id, ShareRoleEnum::Reader, now_secs());
    let req = test::TestRequest::delete()
        .uri(&format!("/api/shares/{}/{}", file.id, bob.user_id))
        .cookie(alice.jwt.clone())
        .set_json(&revoke_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let post_revoke = search_for_word!(app, bob);
    assert!(
        post_revoke.iter().all(|f| f.id != file.id),
        "bob must not find the file after revoke; got {:?}",
        post_revoke.iter().map(|f| f.id).collect::<Vec<_>>()
    );

    let alice_hits = search_for_word!(app, alice);
    assert!(
        alice_hits.iter().any(|f| f.id == file.id && f.is_owner),
        "owner alice should still find her file after revoke"
    );

    let _ = context;
}
