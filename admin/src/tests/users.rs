use crate::data::users::{self, search::UsersSort};
use context::Context;

#[async_std::test]
async fn test_find_all_users() {
    let context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    super::get_users(&context).await;

    let users = repository
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

    assert_eq!(users.len(), 10);
    assert_eq!(users[0].email, "one@test.com");
}

#[async_std::test]
async fn test_pagination_for_users() {
    let context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    super::get_users(&context).await;

    let users = repository
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

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "two@test.com");
}

#[async_std::test]
async fn test_sort_for_users() {
    let context: Context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    super::get_users(&context).await;

    let users = repository
        .users()
        .find(users::search::Search {
            sort: Some(UsersSort::CreatedAt),
            order: Some("desc".to_string()),
            search: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();

    assert_eq!(users.len(), 10);
    assert_eq!(users[0].email, "ten@test.com");
}

#[async_std::test]
async fn test_search_user_by_email() {
    let context: Context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    super::get_users(&context).await;

    let users = repository
        .users()
        .find(users::search::Search {
            sort: None,
            order: None,
            search: Some("one".to_string()),
            limit: None,
            offset: None,
        })
        .await
        .unwrap();

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "one@test.com");
}
