use context::Context;
use entity::{
    file_tokens::{self, Scope},
    user_files, ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Uuid,
};

use crate::{
    data::search::Search,
    mock::{create_file, file_search_key, index_tags, query_tags, search_key},
    repository::Repository,
};

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

    let results = repository.tokens(recipient.id).search(search).await.unwrap();

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

    let results = repository.tokens(recipient.id).search(search).await.unwrap();

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

/// The re-index sweep is resumable because "pending" is derived from the
/// absence of root tags rather than tracked separately: writing a file's tags
/// is what takes it off the list.
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

    // Freshly created files are already indexed, so nothing is pending.
    assert!(repository
        .tokens(user.id)
        .pending_reindex(100)
        .await
        .unwrap()
        .is_empty());

    // Clear one file's root scope, standing in for what the migration did.
    file_tokens::Entity::delete_many()
        .filter(file_tokens::Column::FileId.eq(a.id))
        .filter(file_tokens::Column::Scope.eq(i32::from(Scope::Root)))
        .exec(&context.db)
        .await
        .unwrap();

    let pending = repository.tokens(user.id).pending_reindex(100).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, a.id);

    // Re-indexing it takes it off the list, with no progress state anywhere.
    repository
        .tokens(user.id)
        .reindex(
            a.id,
            entity::file_tokens::SearchTags::new(Some(index_tags(&search_key(), "alpha")), None),
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
    file_tokens::Entity::delete_many()
        .filter(file_tokens::Column::FileId.eq(file.id))
        .exec(&context.db)
        .await
        .unwrap();

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

#[actix_web::test]
async fn search_by_content_hash_finds_file_without_any_tags() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, "hello", None, Some("image/png"))
        .await
        .unwrap();

    let search = Search {
        hash: Some("asd".to_string()), // mock files carry "asd" in every hash column
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

    // No tags and no hash — the absent hash must not degrade into an
    // empty-string comparison that matches rows.
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
