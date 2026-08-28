use context::Context;
use entity::{
    file_tokens::{self, Scope, Source},
    files, user_files, users, ActiveValue, ColumnTrait, EntityTrait, Expr, PaginatorTrait,
    QueryFilter, Uuid,
};

use crate::{
    data::{reindex::Reindex, search::Search},
    mock::{create_file, file_search_key, index_tags, query_tags, search_key},
    repository::Repository,
};

/// Put a file into the state the re-key migration leaves behind: root tags
/// gone and `name_hash` blanked, which is what marks it as waiting for the
/// owner's sweep.
async fn blank_name_hash(context: &Context, file_id: Uuid) {
    file_tokens::Entity::delete_many()
        .filter(file_tokens::Column::FileId.eq(file_id))
        .filter(file_tokens::Column::Scope.eq(i32::from(Scope::Root)))
        .exec(&context.db)
        .await
        .unwrap();
    files::Entity::update_many()
        .col_expr(files::Column::NameHash, Expr::value(""))
        .filter(files::Column::Id.eq(file_id))
        .exec(&context.db)
        .await
        .unwrap();
}

/// Mock users are inserted with a blank fingerprint. The reindex write
/// compares against `users.fingerprint`, so tests that go through that path
/// need a real epoch on the row.
async fn stamp_fingerprint(context: &Context, user_id: Uuid) -> String {
    const FP: &str = "reindex-epoch";
    users::Entity::update_many()
        .col_expr(users::Column::Fingerprint, Expr::value(FP))
        .filter(users::Column::Id.eq(user_id))
        .exec(&context.db)
        .await
        .unwrap();
    FP.to_string()
}

/// Give `user_id` non-owner access to `file_id`, the way a share grant does.
/// Deliberately touches nothing but `user_files`: the point of the scheme is
/// that sharing writes no index rows at all.
async fn share_with(context: &Context, file_id: Uuid, user_id: Uuid) {
    user_files::Entity::insert(user_files::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        file_id: ActiveValue::Set(file_id),
        user_id: ActiveValue::Set(user_id),
        encrypted_key: ActiveValue::Set("key".to_string()),
        is_owner: ActiveValue::Set(false),
        created_at: ActiveValue::Set(0),
        expires_at: ActiveValue::Set(None),
        share_role: ActiveValue::Set("reader".to_string()),
        shared_at: ActiveValue::Set(Some(0)),
        shared_by_user_id: ActiveValue::Set(None),
        member_signature: ActiveValue::Set(None),
        member_signed_at: ActiveValue::Set(None),
    })
    .exec_without_returning(&context.db)
    .await
    .unwrap();
}

async fn count_tags(context: &Context, file_id: Uuid, scope: Scope) -> u64 {
    file_tokens::Entity::find()
        .filter(file_tokens::Column::FileId.eq(file_id))
        .filter(file_tokens::Column::Scope.eq(i32::from(scope)))
        .count(&context.db)
        .await
        .unwrap()
}

#[actix_web::test]
async fn indexing_writes_both_scopes() {
    let context = Context::mock_sqlite().await;
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let name = "hello_world.txt";
    let dir = create_file(&context, &user, name, None, Some("dir"))
        .await
        .unwrap();

    let expected = index_tags(&search_key(), name).len() as u64;

    assert_eq!(count_tags(&context, dir.id, Scope::Root).await, expected);
    assert_eq!(count_tags(&context, dir.id, Scope::File).await, expected);
}

#[actix_web::test]
async fn root_tags_find_an_owned_file() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let dir = create_file(&context, &user, "hello", None, Some("dir"))
        .await
        .unwrap();

    let search = Search {
        root_tags: Some(query_tags(&search_key(), "hello")),
        ..Default::default()
    };

    let results = repository.tokens(user.id).search(search).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, dir.id);
}

#[actix_web::test]
async fn heavier_matches_rank_first() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let dir = create_file(&context, &user, "hello", None, Some("dir"))
        .await
        .unwrap();

    let dir2 = create_file(&context, &user, "hello hello", None, Some("dir"))
        .await
        .unwrap();

    let search = Search {
        root_tags: Some(query_tags(&search_key(), "hello")),
        ..Default::default()
    };

    let mut results = repository.tokens(user.id).search(search).await.unwrap();

    let second = results.pop().unwrap();
    let first = results.pop().unwrap();

    assert_eq!(first.id, dir2.id);
    assert_eq!(second.id, dir.id);
}

/// A pasted filename must surface that file first, however much weight
/// text-rich rows accumulate on the same tokens. The query's own name hash
/// rides along and an exact `files.name_hash` match outranks every token
/// score.
#[actix_web::test]
async fn exact_name_hash_outranks_token_weight() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let target = create_file(&context, &user, "IMG_0179.mov", None, Some("video/quicktime"))
        .await
        .unwrap();
    let heavy = create_file(
        &context,
        &user,
        "img 0179 mov img 0179 mov notes",
        None,
        Some("dir"),
    )
    .await
    .unwrap();

    let query = "IMG_0179.mov";
    let name_hash = cryptfns::search::tag(&search_key(), query).unwrap();

    // Without the name hash the heavier row out-weighs the file itself.
    let blind = Search {
        root_tags: Some(query_tags(&search_key(), query)),
        ..Default::default()
    };
    let results = repository.tokens(user.id).search(blind).await.unwrap();
    assert_eq!(results[0].id, heavy.id);

    // With it, the exact match ranks first and carries its evidence.
    let precise = Search {
        root_tags: Some(query_tags(&search_key(), query)),
        name_hash: Some(name_hash.clone()),
        ..Default::default()
    };
    let results = repository.tokens(user.id).search(precise).await.unwrap();
    assert_eq!(results[0].id, target.id);
    assert!(results[0].search_hits.unwrap_or(0) >= 1);
    assert!(results[0].search_name_hits.unwrap_or(0) >= 1);

    // The hash alone is a valid query: the row surfaces with no tags sent.
    let exact_only = Search {
        name_hash: Some(name_hash),
        ..Default::default()
    };
    let results = repository.tokens(user.id).search(exact_only).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, target.id);
}

/// The property the whole scheme exists for: a recipient searches a shared
/// file through the file scope, and the grant itself wrote nothing.
#[actix_web::test]
async fn a_share_costs_no_index_rows_and_stays_searchable() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let owner = entity::mock::create_user(&context.db, "owner@test.com", None).await;
    let recipient = entity::mock::create_user(&context.db, "recipient@test.com", None).await;

    let name = "quarterly";
    let file = create_file(&context, &owner, name, None, Some("dir"))
        .await
        .unwrap();

    let before = count_tags(&context, file.id, Scope::File).await;
    share_with(&context, file.id, recipient.id).await;

    assert_eq!(count_tags(&context, file.id, Scope::File).await, before);
    assert_eq!(
        count_tags(&context, file.id, Scope::Root).await,
        index_tags(&search_key(), name).len() as u64
    );

    let search = Search {
        file_tags: Some(query_tags(&file_search_key(name), name)),
        ..Default::default()
    };

    let results = repository
        .tokens(recipient.id)
        .search(search)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, file.id);
}

/// A recipient holds the file key but not the owner's root key, so the owner's
/// scope must be useless to them even though the rows are right there.
#[actix_web::test]
async fn a_recipient_cannot_match_the_owners_scope() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let owner = entity::mock::create_user(&context.db, "owner@test.com", None).await;
    let recipient = entity::mock::create_user(&context.db, "recipient@test.com", None).await;

    let name = "quarterly";
    let file = create_file(&context, &owner, name, None, Some("dir"))
        .await
        .unwrap();
    share_with(&context, file.id, recipient.id).await;

    // File-scope tags sent against the root scope match nothing, and vice
    // versa — the scopes are keyed independently.
    let search = Search {
        root_tags: Some(query_tags(&file_search_key(name), name)),
        ..Default::default()
    };

    let results = repository
        .tokens(recipient.id)
        .search(search)
        .await
        .unwrap();

    assert!(results.is_empty());
}

/// Scope 0 and scope 1 are never both sent for one file, so a file cannot be
/// counted twice and outrank a genuinely better match.
#[actix_web::test]
async fn scopes_do_not_cross_match() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let name = "hello";
    create_file(&context, &user, name, None, Some("dir"))
        .await
        .unwrap();

    let search = Search {
        root_tags: Some(query_tags(&file_search_key(name), name)),
        file_tags: Some(query_tags(&search_key(), name)),
        ..Default::default()
    };

    let results = repository.tokens(user.id).search(search).await.unwrap();

    assert!(results.is_empty());
}

/// A keyed hash is not a marker, so a file that carries one is invisible to
/// the sweep no matter how empty its index is.
///
/// That is not hypothetical: an updated app writing to a server still on the
/// old version sends a keyed hash, and that server stores it while dropping
/// the tags it does not understand. The file ends up with a hash the sweep
/// cannot select and no tags to its name. It is why the rekey migration
/// blanks every hash rather than only the reversible ones — at that moment it
/// has just dropped the token tables, so every file is pending and the ones
/// already holding a keyed hash are the only ones that could be missed.
#[actix_web::test]
async fn pending_reindex_cannot_see_a_file_that_kept_its_keyed_hash() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "kept@test.com", None).await;

    let file = create_file(&context, &user, "stranded", None, Some("dir"))
        .await
        .unwrap();

    // The state an old server leaves: the hash it stored, none of the tags.
    file_tokens::Entity::delete_many()
        .filter(file_tokens::Column::FileId.eq(file.id))
        .exec(&context.db)
        .await
        .unwrap();

    let pending = repository
        .tokens(user.id)
        .pending_reindex(100)
        .await
        .unwrap();
    assert!(
        pending.is_empty(),
        "a keyed hash keeps a file off the sweep even with no tags at all — \
         blanking it is the migration\'s job, not something the sweep can infer"
    );

    // And blanking it is all it takes to enrol.
    blank_name_hash(&context, file.id).await;

    let pending = repository
        .tokens(user.id)
        .pending_reindex(100)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, file.id);
}

/// The re-index sweep is resumable because "pending" is derived from the
/// blank `name_hash` the migration left rather than tracked separately:
/// writing the keyed hash is what takes a file off the list.
#[actix_web::test]
async fn pending_reindex_shrinks_as_files_are_indexed() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let a = create_file(&context, &user, "alpha", None, Some("dir"))
        .await
        .unwrap();
    create_file(&context, &user, "beta", None, Some("dir"))
        .await
        .unwrap();

    // Freshly created files carry a keyed hash, so nothing is pending.
    assert!(repository
        .tokens(user.id)
        .pending_reindex(100)
        .await
        .unwrap()
        .is_empty());

    blank_name_hash(&context, a.id).await;

    let fingerprint = stamp_fingerprint(&context, user.id).await;

    let pending = repository
        .tokens(user.id)
        .pending_reindex(100)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, a.id);

    // Re-indexing it takes it off the list, with no progress state anywhere.
    repository
        .manage(user.id)
        .reindex(
            a.id,
            Reindex {
                name_hash: Some(cryptfns::search::tag(&search_key(), "alpha").unwrap()),
                fingerprint: Some(fingerprint),
                search_tokens_root: Some(index_tags(&search_key(), "alpha")),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert!(repository
        .tokens(user.id)
        .pending_reindex(100)
        .await
        .unwrap()
        .is_empty());
}

/// The record of "done" is deliberately the keyed `name_hash`, not "has root
/// tags": a name the tokenizer reduces to nothing re-indexes successfully
/// with zero tags, and it must leave the pending list rather than come back
/// on every fetch forever — the sweep in every client loops until this list
/// drains.
#[actix_web::test]
async fn pending_reindex_lets_go_of_a_file_that_indexed_to_zero_tags() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, ";", None, Some("dir"))
        .await
        .unwrap();
    blank_name_hash(&context, file.id).await;

    let fingerprint = stamp_fingerprint(&context, user.id).await;

    assert_eq!(
        repository
            .tokens(user.id)
            .pending_reindex(100)
            .await
            .unwrap()
            .len(),
        1
    );

    repository
        .manage(user.id)
        .reindex(
            file.id,
            Reindex {
                name_hash: Some(cryptfns::search::tag(&search_key(), ";").unwrap()),
                fingerprint: Some(fingerprint),
                search_tokens_root: Some(vec![]),
                search_tokens_file: Some(vec![]),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert!(repository
        .tokens(user.id)
        .pending_reindex(100)
        .await
        .unwrap()
        .is_empty());
}

/// A write keyed under a discarded private key must not take the file off
/// the pending list. The 32-hex shape is what marks "done", and nothing
/// would ever revisit a row that landed that way.
#[actix_web::test]
async fn reindex_rejects_a_fingerprint_that_is_not_the_live_key() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, "alpha", None, Some("dir"))
        .await
        .unwrap();
    blank_name_hash(&context, file.id).await;
    stamp_fingerprint(&context, user.id).await;

    let err = repository
        .manage(user.id)
        .reindex(
            file.id,
            Reindex {
                name_hash: Some(cryptfns::search::tag(&search_key(), "alpha").unwrap()),
                fingerprint: Some("the-previous-key".to_string()),
                search_tokens_root: Some(index_tags(&search_key(), "alpha")),
                ..Default::default()
            },
        )
        .await
        .unwrap_err();

    assert!(
        matches!(err, error::Error::BadRequest(ref m) if m == "reindex_key_rotated"),
        "got {err:?}"
    );
    assert_eq!(
        repository
            .tokens(user.id)
            .pending_reindex(100)
            .await
            .unwrap()
            .len(),
        1
    );
}

/// A row still carrying the legacy 64-hex digest counts as pending too:
/// nothing writes that shape any more, but a row that slipped past the
/// migration must be offered to the sweep rather than hidden from it.
#[actix_web::test]
async fn pending_reindex_includes_a_row_with_a_legacy_digest() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, "alpha", None, Some("dir"))
        .await
        .unwrap();
    files::Entity::update_many()
        .col_expr(
            files::Column::NameHash,
            Expr::value(cryptfns::sha256::digest("alpha".as_bytes())),
        )
        .filter(files::Column::Id.eq(file.id))
        .exec(&context.db)
        .await
        .unwrap();

    assert_eq!(
        repository
            .tokens(user.id)
            .pending_reindex(100)
            .await
            .unwrap()
            .len(),
        1
    );
}

/// Another user's unindexed files must never appear in this user's sweep.
#[actix_web::test]
async fn pending_reindex_is_scoped_to_the_caller() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let owner = entity::mock::create_user(&context.db, "owner@test.com", None).await;
    let other = entity::mock::create_user(&context.db, "other@test.com", None).await;

    let file = create_file(&context, &owner, "alpha", None, Some("dir"))
        .await
        .unwrap();
    blank_name_hash(&context, file.id).await;

    // Even shared, it is not the recipient's to re-index — they cannot
    // produce the owner's root tags.
    share_with(&context, file.id, other.id).await;

    assert_eq!(
        repository
            .tokens(owner.id)
            .pending_reindex(100)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(repository
        .tokens(other.id)
        .pending_reindex(100)
        .await
        .unwrap()
        .is_empty());
}

/// A content digest is findable through the ordinary index: the digest is
/// tagged like any other token at upload, the query tags the raw string the
/// user typed, and equality does the rest. No digest ever crosses the wire.
#[actix_web::test]
async fn a_digest_tag_finds_the_file_it_was_indexed_for() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, "hello", None, Some("image/png"))
        .await
        .unwrap();

    let digest = cryptfns::sha256::digest("file bytes".as_bytes());
    let tag = cryptfns::search::tag(&search_key(), &digest).unwrap();
    repository
        .tokens(user.id)
        .upsert(file.id, Scope::Root, Source::Name, vec![format!("{tag}:1")])
        .await
        .unwrap();

    let search = Search {
        root_tags: Some(vec![tag]),
        ..Default::default()
    };
    let results = repository.tokens(user.id).search(search).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, file.id);
}

#[actix_web::test]
async fn search_with_no_tags_matches_nothing() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    create_file(&context, &user, "hello", None, Some("image/png"))
        .await
        .unwrap();

    // No tags at all must match nothing rather than degrade into an
    // unfiltered join that returns the whole drive.
    let search = Search {
        root_tags: Some(vec![]),
        ..Default::default()
    };

    let results = repository.tokens(user.id).search(search).await.unwrap();

    assert!(results.is_empty());
}

#[actix_web::test]
async fn create_files_and_try_getting_total_used_space() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, "hello", None, Some("application/json"))
        .await
        .unwrap();

    let file2 = create_file(
        &context,
        &user,
        "hello hello",
        None,
        Some("application/json"),
    )
    .await
    .unwrap();

    let total = file.size.unwrap() + file2.size.unwrap();

    let used_space = repository.query(user.id).used_space().await.unwrap();

    assert_eq!(total, used_space)
}

/// The same digest indexed by two accounts under their own keys stays two
/// unrelated tags: keying, not the ACL, is what makes a digest lifted from
/// elsewhere useless for probing another account.
#[actix_web::test]
async fn the_same_digest_tags_differently_per_account() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let owner = entity::mock::create_user(&context.db, "owner@test.com", None).await;
    let stranger = entity::mock::create_user(&context.db, "stranger@test.com", None).await;

    let file = create_file(&context, &owner, "hello", None, Some("image/png"))
        .await
        .unwrap();

    let digest = cryptfns::sha256::digest("shared bytes".as_bytes());
    let owner_tag = cryptfns::search::tag(&search_key(), &digest).unwrap();
    repository
        .tokens(owner.id)
        .upsert(
            file.id,
            Scope::Root,
            Source::Name,
            vec![format!("{owner_tag}:1")],
        )
        .await
        .unwrap();

    let stranger_key: [u8; 32] = [9u8; 32];
    let stranger_tag = cryptfns::search::tag(&stranger_key, &digest).unwrap();
    assert_ne!(owner_tag, stranger_tag);

    let search = Search {
        root_tags: Some(vec![stranger_tag]),
        ..Default::default()
    };
    assert!(repository
        .tokens(stranger.id)
        .search(search)
        .await
        .unwrap()
        .is_empty());
}

/// A digest tagged under the file scope answers a recipient's query — the
/// backup case for an account holding incoming shares, through the same
/// file-key expansion every other shared-file query uses.
#[actix_web::test]
async fn a_file_scope_digest_tag_reaches_the_share_recipient() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let owner = entity::mock::create_user(&context.db, "owner@test.com", None).await;
    let recipient = entity::mock::create_user(&context.db, "recipient@test.com", None).await;

    let file = create_file(&context, &owner, "hello", None, Some("image/png"))
        .await
        .unwrap();
    share_with(&context, file.id, recipient.id).await;

    let digest = cryptfns::sha256::digest("shared bytes".as_bytes());
    let tag = cryptfns::search::tag(&file_search_key("digest-test"), &digest).unwrap();
    repository
        .tokens(owner.id)
        .upsert(file.id, Scope::File, Source::Name, vec![format!("{tag}:1")])
        .await
        .unwrap();

    let search = Search {
        file_tags: Some(vec![tag]),
        ..Default::default()
    };
    let found = repository
        .tokens(recipient.id)
        .search(search)
        .await
        .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, file.id);
}
