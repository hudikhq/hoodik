use context::Context;

use crate::{data::search::Search, mock::create_file, repository::Repository};

#[actix_web::test]
async fn create_token_and_get_it() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;
    let tokens = repository.tokens(user.id);

    let token = cryptfns::tokenizer::Token {
        token: "hello".to_string(),
        weight: 1,
    };

    let model = tokens.create(token).await.unwrap();
    let gotten = tokens.get(&model.hash).await.unwrap();

    assert_eq!(model.hash, gotten.hash);
    assert_eq!(model.id, gotten.id);
}

#[actix_web::test]
async fn create_file_with_tokens() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let name = "hello_world.txt";
    let initial_tokens = cryptfns::tokenizer::into_hashed_tokens(&name).unwrap();

    let dir = create_file(&context, &user, &name, None, Some("dir"))
        .await
        .unwrap();

    let tokens = repository.tokens(user.id).get_tokens(dir.id).await.unwrap();

    assert_eq!(initial_tokens.len(), tokens.len());

    for token in tokens {
        let initial = initial_tokens
            .iter()
            .find(|t| t.token == token.token)
            .unwrap();

        assert_eq!(initial.weight, token.weight);
    }
}

#[actix_web::test]
async fn create_files_and_try_searching() {
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
        dir_id: None,
        search: Some("hello".to_string()),
        skip: None,
        limit: None,
    };

    let mut results = repository.tokens(user.id).search(search).await.unwrap();

    let second = results.pop().unwrap();
    let first = results.pop().unwrap();

    // println!("First {:#?}", first);
    // println!("Second {:#?}", second);

    assert_eq!(first.id, dir2.id);
    assert_eq!(second.id, dir.id);
}

#[actix_web::test]
async fn create_files_and_try_searching_by_hash() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let file = create_file(&context, &user, "hello", None, Some("image/png"))
        .await
        .unwrap();

    let search = Search {
        dir_id: None,
        search: Some("asd".to_string()), // the hashes aren't actually calculated in the tests, all the files have "asd" as hash
        skip: None,
        limit: None,
    };

    let mut results = repository.tokens(user.id).search(search).await.unwrap();

    let first = results.pop().unwrap();

    // println!("First {:#?}", first);

    assert_eq!(first.id, file.id);
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
