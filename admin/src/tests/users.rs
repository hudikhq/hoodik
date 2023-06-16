use crate::data::users::{self, search::UsersSort};
use context::Context;

#[async_std::test]
async fn test_find_all_users() {
    let context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    super::get_users(&context).await;

    let paginated = repository
        .users()
        .find(users::search::Search {
            sort: None,
            order: None,
            search: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();
    let users = paginated.users;

    assert_eq!(users.len(), 9);
    assert_eq!(users[0].email, "1@test.com");
    assert_eq!(users[1].email, "2@test.com");
}

#[async_std::test]
async fn test_pagination_for_users() {
    let context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    super::get_users(&context).await;

    let paginated = repository
        .users()
        .find(users::search::Search {
            sort: None,
            order: None,
            search: None,
            limit: Some(1),
            offset: Some(1),
        })
        .await
        .unwrap();
    let users = paginated.users;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "2@test.com");
}

#[async_std::test]
async fn test_sort_for_users() {
    let context: Context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    super::get_users(&context).await;

    let paginated = repository
        .users()
        .find(users::search::Search {
            sort: Some(UsersSort::Email),
            order: Some("desc".to_string()),
            search: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();
    let users = paginated.users;

    assert_eq!(users.len(), 9);
    assert_eq!(users[0].email, "9@test.com");
}

#[async_std::test]
async fn test_search_user_by_email() {
    let context: Context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    super::get_users(&context).await;

    let paginated = repository
        .users()
        .find(users::search::Search {
            sort: None,
            order: None,
            search: Some("1@".to_string()),
            limit: None,
            offset: None,
        })
        .await
        .unwrap();
    let users = paginated.users;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "1@test.com");
}

#[async_std::test]
async fn test_find_all_users_and_properly_add_session() {
    let context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    let users = super::get_users(&context).await;

    let user = users.get(0).unwrap().clone();
    entity::mock::create_session(&context.db, &user, None, None, true).await;
    entity::mock::create_session(&context.db, &user, None, None, true).await;
    entity::mock::create_session(&context.db, &user, None, None, true).await;
    let last_session = entity::mock::create_session(&context.db, &user, None, None, false).await;

    let paginated = repository
        .users()
        .find(users::search::Search {
            sort: None,
            order: None,
            search: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();
    let users = paginated.users;

    assert_eq!(users.len(), 9);
    assert_eq!(users[0].email, "1@test.com");
    assert!(users[0].last_session.is_some());
    assert_eq!(users[0].last_session.clone().unwrap().id, last_session.id);
    assert_eq!(users[1].email, "2@test.com");
}

#[async_std::test]
async fn test_delete_user() {
    let context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    let users = super::get_users(&context).await;
    let user = users.get(0).unwrap().clone();

    repository.users().delete(user.id).await.unwrap();
}
