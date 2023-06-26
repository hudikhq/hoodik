use context::Context;

use crate::{data::rename::Rename, mock::create_file, repository::Repository};

#[actix_web::test]
async fn create_file_and_rename_it() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let dir = create_file(&context, &user, "dir", None, Some("dir"))
        .await
        .unwrap();

    let dir2 = create_file(&context, &user, "dir", Some(dir.id), Some("dir"))
        .await
        .unwrap();

    let renamed = repository
        .manage(user.id)
        .rename(
            dir2.id,
            Rename {
                name_hash: Some("dir2".to_string()),
                encrypted_name: Some("dir2".to_string()),
                search_tokens_hashed: Some(vec!["dir2".to_string()]),
            },
        )
        .await
        .unwrap();

    assert_eq!(renamed.name_hash, "dir2");
}
