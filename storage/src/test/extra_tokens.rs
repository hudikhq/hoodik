use context::Context;
use entity::{
    file_tokens::{self, SearchTags, Source},
    files, ColumnTrait, EntityTrait, Expr, QueryFilter, Uuid,
};

use crate::{
    data::{
        extra_tokens::ExtraTokens, rename::Rename, replace_content::ValidatedReplaceContent,
        search::Search,
    },
    mock::{create_file, index_tags, query_tags, search_key},
    repository::Repository,
};

async fn extra_tags(context: &Context, file_id: Uuid) -> Vec<String> {
    file_tokens::Entity::find()
        .filter(file_tokens::Column::FileId.eq(file_id))
        .filter(file_tokens::Column::Source.eq(i32::from(Source::Extra)))
        .all(&context.db)
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.tag)
        .collect()
}

fn extra(root: Vec<&str>, file: Vec<&str>) -> ExtraTokens {
    ExtraTokens {
        search_tokens_root: Some(root.into_iter().map(|t| t.to_string()).collect()),
        search_tokens_file: Some(file.into_iter().map(|t| t.to_string()).collect()),
        ..Default::default()
    }
}

#[actix_web::test]
async fn extra_tokens_survive_rename_and_content_save() {
    let context = Context::mock_sqlite().await;
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;
    let file = create_file(
        &context,
        &user,
        "invoice.pdf",
        None,
        Some("application/pdf"),
    )
    .await
    .unwrap();

    Repository::new(&context.db)
        .manage(user.id)
        .replace_extra(file.id, extra(vec!["ocr1:1"], vec!["ocr2:1"]))
        .await
        .unwrap();

    files::Entity::update_many()
        .col_expr(files::Column::Editable, Expr::value(true))
        .filter(files::Column::Id.eq(file.id))
        .exec(&context.db)
        .await
        .unwrap();

    Repository::new(&context.db)
        .manage(user.id)
        .rename(
            file.id,
            Rename {
                name_hash: Some("0123456789abcdef0123456789abcdef".to_string()),
                encrypted_name: Some("renamed.pdf".to_string()),
                search_tokens_root: Some(index_tags(&search_key(), "renamed.pdf")),
                search_tokens_file: Some(index_tags(&search_key(), "renamed.pdf")),
            },
        )
        .await
        .unwrap();

    Repository::new(&context.db)
        .manage(user.id)
        .replace_content(
            file.id,
            ValidatedReplaceContent {
                _id: file.id,
                size: 42,
                chunks: 1,
                encrypted_name: None,
                encrypted_thumbnail: None,
                search_tags: SearchTags::new(Some(index_tags(&search_key(), "body text")), None),
                force: false,
            },
        )
        .await
        .unwrap();

    let extra = extra_tags(&context, file.id).await;
    assert!(extra.contains(&"ocr1".to_string()));
    assert!(extra.contains(&"ocr2".to_string()));

    let hits = Repository::new(&context.db)
        .tokens(user.id)
        .search(Search {
            root_tags: Some(vec!["ocr1".to_string()]),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, file.id);
}

#[actix_web::test]
async fn empty_extra_tokens_clear_only_extra() {
    let context = Context::mock_sqlite().await;
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;
    let file = create_file(
        &context,
        &user,
        "invoice.pdf",
        None,
        Some("application/pdf"),
    )
    .await
    .unwrap();

    Repository::new(&context.db)
        .manage(user.id)
        .replace_extra(file.id, extra(vec!["ocr1:1"], vec![]))
        .await
        .unwrap();
    Repository::new(&context.db)
        .manage(user.id)
        .replace_extra(
            file.id,
            ExtraTokens {
                search_tokens_root: Some(vec![]),
                search_tokens_file: Some(vec![]),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    assert!(extra_tags(&context, file.id).await.is_empty());

    let hits = Repository::new(&context.db)
        .tokens(user.id)
        .search(Search {
            root_tags: Some(query_tags(&search_key(), "invoice.pdf")),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, file.id);
}
