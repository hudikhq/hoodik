use context::Context;

use crate::{data::create_link::CreateLink, repository::Repository};

#[actix_web::test]
async fn test_create_link() {
    let context = Context::mock_sqlite().await;
    let private_key = cryptfns::rsa::private::generate().unwrap();
    let public_key = cryptfns::rsa::public::from_private(&private_key).unwrap();
    let private_key_string = cryptfns::rsa::private::to_string(&private_key).unwrap();
    let public_key_string = cryptfns::rsa::public::to_string(&public_key).unwrap();

    let user = entity::mock::create_user(
        &context.db,
        "john@test.com",
        Some(public_key_string.clone()),
    )
    .await;
    let (file, _user_file) =
        entity::mock::create_file(&context.db, &user, "test-file", "application/json", None).await;

    let signature =
        cryptfns::rsa::private::sign(&file.id.to_string(), &private_key_string).unwrap();

    let repository = Repository::new(&context);

    let create_link = CreateLink {
        file_id: Some(file.id.to_string()),
        signature: Some(signature),
        encrypted_name: Some("test-file".to_string()),
        encrypted_link_key: Some("test-link-key".to_string()),
        encrypted_thumbnail: None,
        encrypted_file_key: Some("test-file-key".to_string()),
        expires_at: None,
    };

    let res = repository.create(create_link, &user).await;

    assert!(res.is_ok());

    let link = res.unwrap();

    assert_eq!(&link.owner_id, &user.id);
    assert_eq!(&link.owner_email, &user.email);

    repository.increment_downloads(link.id).await.unwrap();

    let link = repository.get(link.id).await.unwrap();

    assert_eq!(link.downloads, 1);
}

#[actix_web::test]
async fn test_trying_to_create_link_for_other_users_file_errors() {
    let context = Context::mock_sqlite().await;
    let private_key = cryptfns::rsa::private::generate().unwrap();
    let public_key = cryptfns::rsa::public::from_private(&private_key).unwrap();
    let private_key_string = cryptfns::rsa::private::to_string(&private_key).unwrap();
    let public_key_string = cryptfns::rsa::public::to_string(&public_key).unwrap();

    let user = entity::mock::create_user(
        &context.db,
        "john@test.com",
        Some(public_key_string.clone()),
    )
    .await;

    // Keeping the same pubkey to make things easier
    let impostor = entity::mock::create_user(
        &context.db,
        "impostor@test.com",
        Some(public_key_string.clone()),
    )
    .await;

    let (file, _user_file) =
        entity::mock::create_file(&context.db, &user, "test-file", "application/json", None).await;

    let signature =
        cryptfns::rsa::private::sign(&file.id.to_string(), &private_key_string).unwrap();

    let repository = Repository::new(&context);

    let create_link = CreateLink {
        file_id: Some(file.id.to_string()),
        signature: Some(signature),
        encrypted_name: Some("test-file".to_string()),
        encrypted_link_key: Some("test-link-key".to_string()),
        encrypted_thumbnail: None,
        encrypted_file_key: Some("test-file-key".to_string()),
        expires_at: None,
    };

    let res = repository.create(create_link, &impostor).await;

    assert!(res.is_err());
}
