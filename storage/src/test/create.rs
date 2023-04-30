use context::Context;
use entity::{users, ConnectionTrait, Uuid};
use error::AppResult;

use crate::{
    data::{app_file::AppFile, create_file::CreateFile, query::Query},
    repository::Repository,
};

fn app_file_vec_to_str_vec(files: &[AppFile]) -> Vec<String> {
    files
        .iter()
        .map(|f| format!("{} -> {}", f.id, f.file_id.clone().unwrap_or_default()))
        .collect()
}

pub async fn create_file<'ctx, T: ConnectionTrait>(
    repository: &'ctx Repository<'ctx, T>,
    user: &users::Model,
    name: &str,
    file_id: Option<Uuid>,
    mime: Option<&str>,
) -> AppResult<AppFile> {
    let mut size = None;
    let mut chunks = None;

    if mime != Some("dir") {
        size = Some(100);
        chunks = Some(1);
    }

    let search_tokens_hashed =
        cryptfns::tokenizer::into_string(cryptfns::tokenizer::into_tokens(name).unwrap())
            .split(";")
            .map(|i| i.to_string())
            .collect::<Vec<_>>();

    let file = CreateFile {
        encrypted_metadata: Some(name.to_string()),
        search_tokens_hashed: Some(search_tokens_hashed),
        mime: mime.map(|m| m.to_string()),
        name_hash: Some(cryptfns::sha256::digest(name.as_bytes())),
        size,
        chunks,
        file_id: file_id.map(|f| f.to_string()),
        file_created_at: None,
    };

    let (am, _, tokens) = file.into_active_model()?;
    repository.manage(&user).create(am, name, tokens).await
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
        .manage(&user)
        .find(Query {
            dir_id: None,
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(response.children.len(), 2);

    let response = repository
        .manage(&user)
        .find(Query {
            dir_id: Some(dir.id.to_string()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(response.parents.last().unwrap().id, dir.id);
    assert_eq!(response.children.len(), 1);

    let response = repository
        .manage(&user)
        .find(Query {
            dir_id: Some(file.id.to_string()),
            ..Default::default()
        })
        .await;

    assert!(response.is_err());
}

#[async_std::test]
async fn get_dir_tree_with_right_ordering() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com").await;

    let mut manual = vec![];

    let dir = create_file(&repository, &user, "dir", None, Some("dir"))
        .await
        .unwrap();
    let dir_id = dir.id.clone();
    manual.push(dir);

    let response = repository.manage(&user).dir_tree(dir_id).await.unwrap();

    assert_eq!(
        app_file_vec_to_str_vec(&manual),
        app_file_vec_to_str_vec(&response)
    );

    let dir2 = create_file(&repository, &user, "dir", Some(dir_id), Some("dir"))
        .await
        .unwrap();
    let dir2_id = dir2.id.clone();
    manual.push(dir2);

    let dir3 = create_file(&repository, &user, "dir", Some(dir2_id), Some("dir"))
        .await
        .unwrap();
    let dir3_id = dir3.id.clone();
    manual.push(dir3);

    let dir4 = create_file(&repository, &user, "dir", Some(dir3_id), Some("dir"))
        .await
        .unwrap();
    let dir4_id = dir4.id.clone();

    let _dir5 = create_file(&repository, &user, "dir", Some(dir4_id), Some("dir"))
        .await
        .unwrap();

    let response = repository.manage(&user).dir_tree(dir3_id).await.unwrap();

    assert_eq!(
        app_file_vec_to_str_vec(&manual),
        app_file_vec_to_str_vec(&response)
    );

    manual.push(dir4);

    let response = repository.manage(&user).dir_tree(dir4_id).await.unwrap();

    assert_eq!(
        app_file_vec_to_str_vec(&manual),
        app_file_vec_to_str_vec(&response)
    );
}

#[async_std::test]
async fn get_file_tree_with_right_ordering() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com").await;

    let mut manual = vec![];

    let dir = create_file(&repository, &user, "dir", None, Some("dir"))
        .await
        .unwrap();
    let dir_id = dir.id.clone();
    manual.push(dir);

    let file = create_file(&repository, &user, "json1", None, Some("application/json"))
        .await
        .unwrap();
    let file1_id = file.id.clone();
    manual.push(file);

    let file = create_file(&repository, &user, "json2", None, Some("application/json"))
        .await
        .unwrap();
    let _file2_id = file.id.clone();
    manual.push(file);

    let dir = create_file(&repository, &user, "dir", Some(dir_id), Some("dir"))
        .await
        .unwrap();
    let dir2_id = dir.id.clone();
    manual.push(dir);

    let file = create_file(&repository, &user, "json3", None, Some("application/json"))
        .await
        .unwrap();
    let _file3_id = file.id.clone();
    manual.push(file);

    let file = create_file(&repository, &user, "json4", None, Some("application/json"))
        .await
        .unwrap();
    let _file4_id = file.id.clone();
    manual.push(file);

    let dir3 = create_file(&repository, &user, "dir", Some(dir2_id), Some("dir"))
        .await
        .unwrap();
    let dir3_id = dir3.id.clone();
    manual.push(dir3);

    let ids = manual.iter().map(|f| f.id.clone()).collect::<Vec<_>>();

    let response = repository.manage(&user).file_tree(dir_id).await.unwrap();

    for file in response.iter() {
        assert!(ids.contains(&file.id));
    }

    let response = repository.manage(&user).file_tree(file1_id).await.unwrap();

    assert_eq!(response.iter().next().unwrap().id, file1_id);

    let response = repository.manage(&user).file_tree(dir3_id).await.unwrap();

    assert_eq!(response.iter().next().unwrap().id, dir3_id);
}
