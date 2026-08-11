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
    let users = paginated.data;

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
    let users = paginated.data;

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
    let users = paginated.data;

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
    let users = paginated.data;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].email, "1@test.com");
}

#[async_std::test]
async fn test_find_all_users_and_properly_add_session() {
    let context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    let users = super::get_users(&context).await;

    let user = users.first().unwrap().clone();
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
    let users = paginated.data;

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
    let user = users.first().unwrap().clone();

    repository.users().delete(user.id).await.unwrap();
}

#[async_std::test]
async fn test_verify_email_clears_pending_activation() {
    use entity::{ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    let context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    let user = entity::mock::create_user(&context.db, "unverified@test.com", None).await;

    entity::users::Entity::update(entity::users::ActiveModel {
        id: ActiveValue::Set(user.id),
        email_verified_at: ActiveValue::Set(None),
        ..Default::default()
    })
    .exec(&context.db)
    .await
    .unwrap();

    entity::user_actions::Entity::insert(entity::user_actions::ActiveModel {
        id: ActiveValue::Set(entity::Uuid::new_v4()),
        user_id: ActiveValue::Set(user.id),
        email: ActiveValue::Set(user.email.clone()),
        action: ActiveValue::Set("activate-email".to_string()),
        created_at: ActiveValue::Set(chrono::Utc::now().timestamp()),
    })
    .exec_without_returning(&context.db)
    .await
    .unwrap();

    repository.users().verify_email(user.id).await.unwrap();

    let updated = entity::users::Entity::find_by_id(user.id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();

    assert!(updated.email_verified_at.is_some());

    let pending = entity::user_actions::Entity::find()
        .filter(entity::user_actions::Column::UserId.eq(user.id))
        .count(&context.db)
        .await
        .unwrap();

    assert_eq!(pending, 0);
}
