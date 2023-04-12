use context::Context;
use entity::{users, ConnectionTrait};
use error::AppResult;

use crate::{
    data::{app_file::AppFile, create_file::CreateFile, query::Query},
    repository::Repository,
};

pub async fn create_file<'ctx, T: ConnectionTrait>(
    repository: &'ctx Repository<'ctx, T>,
    user: &users::Model,
    name: &str,
    file_id: Option<i32>,
    mime: Option<&str>,
) -> AppResult<AppFile> {
    let mut size = None;
    let mut chunks = None;

    if mime != Some("dir") {
        size = Some(100);
        chunks = Some(1);
    }

    let file = CreateFile {
        name_enc: Some(name.to_string()),
        search_tokens_hashed: None,
        encrypted_key: Some("pretending this is an encrypted key".to_string()),
        checksum: Some("dir1".to_string()),
        mime: mime.map(|m| m.to_string()),
        size,
        chunks,
        file_id,
        file_created_at: None,
    };

    let (am, _) = file.into_active_model()?;
    repository.manage(&user).create(am, "file").await
}

#[async_std::test]
async fn create_dir_files() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com").await;
    let user2 = entity::mock::create_user(&context.db, "second@test.com").await;

    let dir = create_file(&repository, &user, "dir", None, Some("dir"))
        .await
        .unwrap();

    let file = create_file(
        &repository,
        &user,
        "file",
        Some(dir.id),
        Some("application/json"),
    )
    .await
    .unwrap();

    let dir2 = create_file(&repository, &user, "dir", None, Some("dir"))
        .await
        .unwrap();

    let file2 = create_file(
        &repository,
        &user2,
        "file",
        Some(dir2.id),
        Some("application/json"),
    )
    .await;

    // Cannot create file in another mans directory
    assert!(file2.is_err());

    let response = repository
        .query(&user)
        .find(Query {
            dir_id: None,
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(response.children.len(), 2);

    let response = repository
        .query(&user)
        .find(Query {
            dir_id: Some(dir.id),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(response.dir.unwrap().id, dir.id);
    assert_eq!(response.children.len(), 1);

    let response = repository
        .query(&user)
        .find(Query {
            dir_id: Some(file.id),
            ..Default::default()
        })
        .await;

    assert!(response.is_err());
}
