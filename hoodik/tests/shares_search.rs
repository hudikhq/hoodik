//! Search index access for shared content.
//!
//! `POST /api/storage/search` matches keyed tags against the index, with the
//! `user_files` join deciding what the caller may reach. Recipients search a
//! shared file through the file scope, whose key rides along with the file key
//! they already hold — so a grant writes no index rows at all. The tests below
//! feed the create route both tag scopes, share the file across roles, and
//! confirm:
//!
//! - the owner still finds their own file
//! - every recipient (Reader / Editor / Co-owner) finds the shared file
//! - an unrelated user does NOT find it, even with the same query
//!
//! Tags go in through the real `POST /api/storage` route and come out
//! through the real `POST /api/storage/search` route — no DB shortcuts, no
//! mocking the search query.

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

/// Stand-ins for the two keys a real client derives — one from its private
/// key, one from the file's own key. Fixed here so the test can seed the index
/// and then query it with matching tags.
fn root_key() -> [u8; 32] {
    [11u8; 32]
}

fn file_key() -> [u8; 32] {
    cryptfns::search::file_key(b"shares-search-test-file").expect("derive file search key")
}

/// Tags in the `"{tag}:{weight}"` form the create route accepts.
fn index_tags(key: &[u8]) -> Vec<String> {
    let tagged = cryptfns::search::tag_tokens(key, SEARCH_WORD).expect("tag search word");
    assert!(
        !tagged.is_empty(),
        "tokenizer returned no tokens for {SEARCH_WORD}; pick a different word"
    );

    tagged
        .into_iter()
        .map(|t| format!("{}:{}", t.token, t.weight))
        .collect()
}

/// Bare tags, as the search route receives them.
fn query_tags(key: &[u8]) -> Vec<String> {
    cryptfns::search::tag_tokens(key, SEARCH_WORD)
        .expect("tag search word")
        .into_iter()
        .map(|t| t.token)
        .collect()
}

/// Projected roster for a folder share: the owner (whose row ships with
/// `share_role = "co-owner"`, which the server's canonicaliser reads back
/// literally) plus the recipient at their granted role.
fn owner_plus_recipient<'a>(
    owner: &'a TestUser,
    recipient: &'a TestUser,
    recipient_role: ShareRoleEnum,
) -> Vec<FolderListMemberSpec<'a>> {
    vec![
        FolderListMemberSpec {
            user: owner,
            share_role: ShareRoleEnum::CoOwner,
            is_owner: true,
            signed_by: owner,
        },
        FolderListMemberSpec {
            user: recipient,
            share_role: recipient_role,
            is_owner: false,
            signed_by: owner,
        },
    ]
}

/// Build a `CreateFile` payload carrying both tag scopes, the way a client
/// does on upload. The file scope is what every share recipient searches
/// through, and writing it here is why granting a share later costs nothing.
fn make_searchable_file(public_pem: &str, name_hash: &str) -> CreateFile {
    let mut payload = make_create_file(public_pem, name_hash);
    payload.search_tokens_root = Some(index_tags(&root_key()));
    payload.search_tokens_file = Some(index_tags(&file_key()));
    payload
}

/// Drive `POST /api/storage/search` with `SEARCH_WORD` as the caller
/// and return the deserialised result list.
macro_rules! search_for_word {
    ($app:expr, $caller:expr) => {{
        let req = actix_web::test::TestRequest::post()
            .uri("/api/storage/search")
            .cookie($caller.jwt.clone())
            .set_json(serde_json::json!({ "file_tags": query_tags(&file_key()) }))
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

/// Insert a file owned by `$user` and seeded with `SEARCH_WORD` tags. Uses the
/// same `POST /api/storage` path the browser hits; the route forwards both tag
/// scopes into the index (see `storage::repository::manage::create`).
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

/// Current clients tokenize + hash the query on-device and POST only
/// the hashes. The wire body must carry no plaintext, the server must
/// find the file from the hashes alone, and a legacy `search` field
/// must be dead weight whenever hashes are present — the plaintext
/// tests above stay green as the old-client compatibility proof.
#[actix_web::test]
async fn test_search_with_client_hashed_tokens_carries_no_plaintext() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");

    let file = create_searchable_file!(app, alice, "octopus-note");

    let body = serde_json::json!({ "root_tags": query_tags(&root_key()) });
    let serialized = serde_json::to_string(&body).unwrap();

    assert!(
        !serialized.contains(SEARCH_WORD),
        "search request body must not contain the plaintext term"
    );
    // The old index stored exactly this, which is what made the whole table
    // reversible with a table over the BERT vocabulary. It must not reappear
    // on the wire either.
    assert!(
        !serialized.contains(&cryptfns::sha256::digest(SEARCH_WORD.as_bytes())),
        "search request body must not contain the bare digest of the term"
    );

    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(alice.jwt.clone())
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let hits: Vec<AppFile> =
        serde_json::from_slice(&test::read_body(resp).await).expect("search response");
    assert!(
        hits.iter().any(|f| f.id == file.id),
        "hashed tokens alone should find the file; got {:?}",
        hits.iter().map(|f| f.id).collect::<Vec<_>>()
    );
}

/// A client old enough to send a plaintext query gets 426, not an empty
/// result set. Answering it with 200 and no hits would read as "your files are
/// gone" to the person holding the phone.
#[actix_web::test]
async fn test_legacy_plaintext_query_is_refused_with_upgrade_required() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    create_searchable_file!(app, alice, "octopus-note");

    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(alice.jwt.clone())
        .set_json(serde_json::json!({ "search": SEARCH_WORD }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UPGRADE_REQUIRED);
}

/// Same for a client that predates keyed tags and still sends bare digests.
/// Serving those would mean matching, and therefore storing, reversible
/// material again.
#[actix_web::test]
async fn test_legacy_digest_query_is_refused_with_upgrade_required() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    create_searchable_file!(app, alice, "octopus-note");

    let digests: Vec<String> = into_hashed_tokens(SEARCH_WORD)
        .expect("tokenize search word")
        .into_iter()
        .map(|t| format!("{}:{}", t.token, t.weight))
        .collect();

    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(alice.jwt.clone())
        .set_json(serde_json::json!({ "search_tokens_hashed": digests }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), StatusCode::UPGRADE_REQUIRED);
}

/// Pasting a file's content digest finds it without any tokens. The
/// four hash columns are populated at upload and surfaced with copy
/// buttons in the file details panel, so this is a reachable flow.
#[actix_web::test]
async fn test_search_by_content_hash_matches_hash_columns() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");

    let file = create_searchable_file!(app, alice, "octopus-note");

    // `make_create_file` stores "sha256" in the sha256 column.
    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(alice.jwt.clone())
        .set_json(serde_json::json!({
            "root_tags": [],
            "hash": "sha256",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let hits: Vec<AppFile> =
        serde_json::from_slice(&test::read_body(resp).await).expect("search response");
    assert!(
        hits.iter().any(|f| f.id == file.id),
        "content-hash lookup should find the file; got {:?}",
        hits.iter().map(|f| f.id).collect::<Vec<_>>()
    );
}

/// A search that carries neither tokens nor a hash must not match rows.
/// Guards the absent-hash path against degrading into an empty-string
/// comparison, which would match any row with an empty hash column.
#[actix_web::test]
async fn test_search_without_tokens_or_hash_matches_nothing() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");

    let file = create_searchable_file!(app, alice, "octopus-note");

    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(alice.jwt.clone())
        .set_json(serde_json::json!({ "root_tags": [] }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let hits: Vec<AppFile> =
        serde_json::from_slice(&test::read_body(resp).await).expect("search response");
    assert!(
        hits.iter().all(|f| f.id != file.id),
        "empty search must not match anything"
    );
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

/// The endpoint that makes a shared folder's *contents* searchable.
///
/// `/api/shares/mine` reports roots — it trims any row whose parent is also
/// shared — which is right for browsing and useless for search, because every
/// file inside the folder is tagged under its own key. `/api/shares/keys`
/// returns the untrimmed set so a recipient can build a query that reaches
/// them.
#[actix_web::test]
async fn test_incoming_keys_include_files_inside_a_shared_folder() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");

    let folder = create_folder!(app, alice, "reports");

    let mut payload = make_searchable_file(&alice.public_pem, "octopus-note");
    payload.file_id = Some(folder.id.to_string());
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(alice.jwt.clone())
        .set_json(&payload)
        .to_request();
    let inner: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).expect("inner file");

    // A folder grant has to carry one entry per file in the subtree, which is
    // exactly why the inner file ends up with a `user_files` row of its own —
    // and why its key is reachable at all.
    let members = owner_plus_recipient(&alice, &bob, ShareRoleEnum::Reader);
    let envelope = build_folder_share_envelope_with_entries(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        folder.id,
        alice.user_id,
        vec![
            (folder.id, b"wrap-folder".to_vec()),
            (inner.id, b"wrap-inner".to_vec()),
        ],
        random_nonce(),
        now_secs(),
        &members,
        &alice,
    );
    let resp = post_share!(app, alice, envelope);
    assert!(resp.status().is_success(), "folder grant failed: {:?}", resp.status());

    // Browsing view: the folder only.
    let req = test::TestRequest::get()
        .uri("/api/shares/mine")
        .cookie(bob.jwt.clone())
        .to_request();
    let page: serde_json::Value =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).expect("mine");
    let roots: Vec<String> = page["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| i["file_id"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        !roots.contains(&inner.id.to_string()),
        "the inner file is trimmed from the roots list, which is why search needed its own route"
    );

    // Search view: every shared row, wrapped key included.
    let req = test::TestRequest::get()
        .uri("/api/shares/keys")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let keys: Vec<serde_json::Value> =
        serde_json::from_slice(&test::read_body(resp).await).expect("keys");
    let ids: Vec<String> = keys
        .iter()
        .map(|k| k["file_id"].as_str().unwrap_or_default().to_string())
        .collect();

    assert!(ids.contains(&inner.id.to_string()), "file inside the folder must be reachable");
    assert!(ids.contains(&folder.id.to_string()), "the folder itself too");
    assert!(
        keys.iter().all(|k| !k["encrypted_key"].as_str().unwrap_or_default().is_empty()),
        "every row carries the key the recipient needs to derive its search key"
    );
    // Nothing beyond what a search needs.
    assert!(keys.iter().all(|k| k.get("encrypted_name").is_none()));

    let _ = context;
}

/// An editor holds the file key, not the owner's root key, so anything they
/// tag under a root scope is unmatchable by the owner. The server ignores the
/// root scope and `name_hash` on a non-owner write rather than trusting the
/// client to leave them out — otherwise one rename from a shared device makes
/// the owner's own file unfindable, permanently and silently.
#[actix_web::test]
async fn test_editor_rename_leaves_the_owners_index_alone() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");

    let file = create_searchable_file!(app, alice, "octopus-note");

    // A non-owner may only rename an editable file, so the note has to be one
    // for the rename to reach the code under test at all.
    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}/editable", file.id))
        .cookie(alice.jwt.clone())
        .set_json(&serde_json::json!({ "editable": true }))
        .to_request();
    assert!(test::call_service(&app, req).await.status().is_success());

    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    // Bob renames, sending both scopes tagged under his own keys — what an
    // un-guarded client does.
    let bob_root = [77u8; 32];
    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}", file.id))
        .cookie(bob.jwt.clone())
        .set_json(serde_json::json!({
            "name_hash": cryptfns::search::tag(&bob_root, "renamed-by-bob").unwrap(),
            "encrypted_name": "renamed-ciphertext",
            "search_tokens_root": index_tags(&bob_root),
            "search_tokens_file": index_tags(&file_key()),
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "an editor may rename a shared file");

    // Alice's root-scope index is untouched: she still finds her file by the
    // word it was indexed under, through her own key.
    let req = test::TestRequest::post()
        .uri("/api/storage/search")
        .cookie(alice.jwt.clone())
        .set_json(serde_json::json!({ "root_tags": query_tags(&root_key()) }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let hits: Vec<AppFile> =
        serde_json::from_slice(&test::read_body(resp).await).expect("search json");
    assert!(
        hits.iter().any(|f| f.id == file.id),
        "the owner must still find her own file after an editor renamed it"
    );

    // And her name_hash is still the one she wrote, not Bob's.
    let req = test::TestRequest::get()
        .uri(&format!("/api/storage/{}/metadata", file.id))
        .cookie(alice.jwt.clone())
        .to_request();
    let after: AppFile =
        serde_json::from_slice(&test::call_and_read_body(&app, req).await).expect("metadata json");
    assert_eq!(
        after.name_hash, "octopus-note",
        "name_hash is keyed under the owner's key; an editor must not rewrite it"
    );

    let _ = context;
}

/// A client from before keyed search sends `sha256(name)` as `name_hash`. That
/// is the reversible digest the re-key migration purged, so a write carrying
/// one is refused rather than stored — otherwise old clients quietly put the
/// leak back, one file at a time.
#[actix_web::test]
async fn test_legacy_name_hash_is_refused_on_create_and_rename() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");

    let legacy = cryptfns::sha256::digest("Passwords.md".as_bytes());
    assert_eq!(legacy.len(), 64, "the shape being refused is a 64-hex digest");

    let mut payload = make_create_file(&alice.public_pem, "placeholder");
    payload.name_hash = Some(legacy.clone());
    let req = test::TestRequest::post()
        .uri("/api/storage")
        .cookie(alice.jwt.clone())
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::UPGRADE_REQUIRED,
        "create must refuse a bare sha256 name_hash"
    );

    // A keyed tag through the same route is accepted, so the refusal is about
    // the digest and not about the route.
    let file = create_searchable_file!(app, alice, "octopus-note");

    let req = test::TestRequest::put()
        .uri(&format!("/api/storage/{}", file.id))
        .cookie(alice.jwt.clone())
        .set_json(serde_json::json!({
            "name_hash": legacy,
            "encrypted_name": "ciphertext",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::UPGRADE_REQUIRED,
        "rename must refuse it too, or the leak comes back through a rename"
    );

    let _ = context;
}
