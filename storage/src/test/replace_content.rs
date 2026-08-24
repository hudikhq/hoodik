use context::Context;
use entity::{
    file_tokens::{self, Scope, SearchTags, Source},
    files, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    TransactionTrait, Uuid,
};

use crate::{
    data::replace_content::ValidatedReplaceContent,
    mock::{create_file, index_tags, search_key},
    repository::Repository,
};

fn replacement(id: Uuid, name: &str) -> ValidatedReplaceContent {
    ValidatedReplaceContent {
        _id: id,
        size: 42,
        chunks: 1,
        encrypted_name: None,
        encrypted_thumbnail: None,
        search_tags: SearchTags::new(Some(index_tags(&search_key(), name)), None),
        force: false,
    }
}

async fn make_editable(context: &Context, id: Uuid) {
    files::ActiveModel {
        id: ActiveValue::Set(id),
        editable: ActiveValue::Set(true),
        ..Default::default()
    }
    .update(&context.db)
    .await
    .unwrap();
}

async fn tag_count(context: &Context, file_id: Uuid, scope: Scope, source: Source) -> u64 {
    file_tokens::Entity::find()
        .filter(file_tokens::Column::FileId.eq(file_id))
        .filter(file_tokens::Column::Scope.eq(i32::from(scope)))
        .filter(file_tokens::Column::Source.eq(i32::from(source)))
        .count(&context.db)
        .await
        .unwrap()
}

async fn pending_version(context: &Context, file_id: Uuid) -> Option<i32> {
    files::Entity::find_by_id(file_id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap()
        .pending_version
}

#[actix_web::test]
async fn replacing_content_allocates_a_pending_version_and_reindexes() {
    let context = Context::mock_sqlite().await;
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, "note.md", None, Some("text/markdown"))
        .await
        .unwrap();
    make_editable(&context, file.id).await;

    Repository::new(&context.db)
        .manage(user.id)
        .replace_content(file.id, replacement(file.id, "rewritten body"))
        .await
        .unwrap();

    assert_eq!(pending_version(&context, file.id).await, Some(2));
    assert_eq!(
        tag_count(&context, file.id, Scope::Root, Source::Content).await,
        index_tags(&search_key(), "rewritten body").len() as u64
    );
    assert_eq!(
        tag_count(&context, file.id, Scope::Root, Source::Name).await,
        index_tags(&search_key(), "note.md").len() as u64,
        "a content save must not replace name tokens"
    );
}

/// The pending-version allocation and the index rewrite are one state change.
///
/// Proven by running the whole method inside a transaction the test then rolls
/// back: if any part of it wrote through its own connection instead of the one
/// it was handed, that write would survive the rollback. A save that committed
/// the new version but not the index would leave the file pointing at content
/// the index describes wrongly, and nothing would ever correct it — the file
/// looks indexed, so no sweep picks it up.
#[actix_web::test]
async fn replacing_content_is_atomic_with_its_reindex() {
    let context = Context::mock_sqlite().await;
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, "note.md", None, Some("text/markdown"))
        .await
        .unwrap();
    make_editable(&context, file.id).await;

    let tags_before = tag_count(&context, file.id, Scope::Root, Source::Name).await;
    assert!(tags_before > 0, "the file starts indexed");

    let tx = context.db.begin().await.unwrap();
    Repository::new(&tx)
        .manage(user.id)
        .replace_content(file.id, replacement(file.id, "rewritten body"))
        .await
        .unwrap();
    tx.rollback().await.unwrap();

    assert_eq!(
        pending_version(&context, file.id).await,
        None,
        "the pending version must not survive a rolled-back save"
    );
    assert_eq!(
        tag_count(&context, file.id, Scope::Root, Source::Name).await,
        tags_before,
        "the index must not survive a rolled-back save either"
    );
}

/// The concurrent-edit guard refuses before writing anything, so a rejected
/// save cannot leave the index half-rewritten.
#[actix_web::test]
async fn a_refused_concurrent_save_writes_nothing() {
    let context = Context::mock_sqlite().await;
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, "note.md", None, Some("text/markdown"))
        .await
        .unwrap();
    make_editable(&context, file.id).await;

    Repository::new(&context.db)
        .manage(user.id)
        .replace_content(file.id, replacement(file.id, "first"))
        .await
        .unwrap();

    let tags_after_first = tag_count(&context, file.id, Scope::Root, Source::Content).await;

    let refused = Repository::new(&context.db)
        .manage(user.id)
        .replace_content(file.id, replacement(file.id, "second"))
        .await;

    assert!(refused.is_err(), "a second save without force is refused");
    assert_eq!(pending_version(&context, file.id).await, Some(2));
    assert_eq!(
        tag_count(&context, file.id, Scope::Root, Source::Content).await,
        tags_after_first,
        "the refused save must not have touched the index"
    );
}
