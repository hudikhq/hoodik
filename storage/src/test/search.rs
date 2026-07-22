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
    let initial_tokens = cryptfns::tokenizer::into_hashed_tokens(name).unwrap();

    let dir = create_file(&context, &user, name, None, Some("dir"))
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
        search: Some("hello".to_string()),
        ..Default::default()
    };

    let mut results = repository.tokens(user.id).search(search).await.unwrap();

    let second = results.pop().unwrap();
    let first = results.pop().unwrap();

    // println!("First {:#?}", first);
    // println!("Second {:#?}", second);

    assert_eq!(first.id, dir2.id);
    assert_eq!(second.id, dir.id);
}

fn wire_tokens(input: &str) -> Vec<String> {
    cryptfns::tokenizer::into_hashed_tokens(input)
        .unwrap()
        .into_iter()
        .map(|t| format!("{}:{}", t.token, t.weight))
        .collect()
}

#[test]
fn into_tuple_prefers_client_hashed_tokens_and_ignores_plaintext() {
    let search = Search {
        search: Some("this plaintext must not be tokenized".to_string()),
        search_tokens_hashed: Some(wire_tokens("hello")),
        ..Default::default()
    };

    let (_, tokens, _, _, _) = search.into_tuple();

    let expected = cryptfns::tokenizer::into_hashed_tokens("hello").unwrap();
    assert_eq!(tokens.len(), expected.len());
    for token in &tokens {
        let matching = expected.iter().find(|t| t.token == token.token).unwrap();
        assert_eq!(token.weight, matching.weight);
    }
}

#[test]
fn into_tuple_tokenizes_plaintext_for_legacy_clients() {
    let search = Search {
        search: Some("hello world".to_string()),
        ..Default::default()
    };

    let (_, tokens, _, _, _) = search.into_tuple();

    let expected = cryptfns::tokenizer::into_hashed_tokens("hello world").unwrap();
    assert_eq!(tokens.len(), expected.len());
    for token in &tokens {
        assert!(expected.iter().any(|t| t.token == token.token));
    }
}

#[actix_web::test]
async fn search_with_client_hashed_tokens_finds_file() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let dir = create_file(&context, &user, "hello", None, Some("dir"))
        .await
        .unwrap();

    let search = Search {
        search_tokens_hashed: Some(wire_tokens("hello")),
        ..Default::default()
    };

    let results = repository.tokens(user.id).search(search).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, dir.id);
}

#[actix_web::test]
async fn hashed_tokens_present_means_plaintext_is_never_used() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    create_file(&context, &user, "hello", None, Some("dir"))
        .await
        .unwrap();

    // The plaintext names an existing file; the hashed tokens do not. A
    // server that still tokenized the plaintext would return a hit here.
    let search = Search {
        search: Some("hello".to_string()),
        search_tokens_hashed: Some(wire_tokens("xylophone")),
        ..Default::default()
    };

    let results = repository.tokens(user.id).search(search).await.unwrap();

    assert!(results.is_empty());
}

#[actix_web::test]
async fn search_with_no_tokens_matches_nothing() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    create_file(&context, &user, "hello", None, Some("image/png"))
        .await
        .unwrap();

    let search = Search {
        search_tokens_hashed: Some(vec![]),
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
