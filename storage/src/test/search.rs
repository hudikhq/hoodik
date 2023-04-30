use context::Context;
use entity::{users, ConnectionTrait, Uuid};
use error::AppResult;

use crate::{
    data::{app_file::AppFile, create_file::CreateFile, search::Search},
    repository::Repository,
};

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
async fn create_token_and_get_it() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com").await;
    let tokens = repository.tokens(&user);

    let token = cryptfns::tokenizer::Token {
        token: "hello".to_string(),
        weight: 1,
    };

    let model = tokens.create(token).await.unwrap();
    let gotten = tokens.get(&model.hash).await.unwrap();

    assert_eq!(model.hash, gotten.hash);
    assert_eq!(model.id, gotten.id);
}

#[async_std::test]
async fn create_file_with_tokens() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com").await;

    let name = "hello_world.txt";
    let initial_tokens = cryptfns::tokenizer::into_tokens(&name).unwrap();

    let dir = create_file(&repository, &user, &name, None, Some("dir"))
        .await
        .unwrap();

    let tokens = repository.tokens(&user).get_tokens(dir.id).await.unwrap();

    assert_eq!(initial_tokens.len(), tokens.len());

    for token in tokens {
        let initial = initial_tokens
            .iter()
            .find(|t| t.token == token.token)
            .unwrap();

        assert_eq!(initial.weight, token.weight);
    }
}

#[async_std::test]
async fn create_files_and_try_searching() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com").await;

    let dir = create_file(&repository, &user, "hello", None, Some("dir"))
        .await
        .unwrap();

    let dir2 = create_file(&repository, &user, "hello hello", None, Some("dir"))
        .await
        .unwrap();

    let search = Search {
        dir_id: None,
        search_tokens_hashed: Some(vec!["hello".to_string()]),
        skip: None,
        limit: None,
    };

    let results = repository.tokens(&user).search(search).await.unwrap();

    let first = results.iter().next().unwrap();
    let second = results.iter().next().unwrap();

    assert_eq!(first.id, dir2.id);
    assert_eq!(second.id, dir.id);
}
