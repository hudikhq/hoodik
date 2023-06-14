use context::Context;

#[async_std::test]
async fn test_find_sessions() {
    let context = Context::mock_sqlite().await;
    let repository = crate::tests::get_repo(&context).await;
    let user = super::get_users(&context).await.get(0).unwrap().clone();
    let _sessions = super::create_sessions(&context, &user).await;

    let sessions = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
            with_deleted: None,
            with_expired: None,
            user_id: None,
            search: None,
            sort: None,
            order: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();

    assert_eq!(sessions.len(), 5);
}

#[async_std::test]
async fn test_find_sessions_by_ip() {
    let context = Context::mock_sqlite().await;
    let repository = crate::tests::get_repo(&context).await;
    let user = super::get_users(&context).await.get(0).unwrap().clone();
    let _sessions = super::create_sessions(&context, &user).await;

    let sessions = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
            with_deleted: None,
            with_expired: None,
            user_id: None,
            search: Some("123.123.123.1".to_string()),
            sort: None,
            order: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();

    assert_eq!(sessions.len(), 1);
}

#[async_std::test]
async fn test_find_sessions_by_email() {
    let context = Context::mock_sqlite().await;
    let repository = crate::tests::get_repo(&context).await;
    let users = super::get_users(&context).await;
    let user = users.get(0).unwrap().clone();
    let user2 = users.get(1).unwrap().clone();

    let _sessions = super::create_sessions(&context, &user).await;

    entity::mock::create_session(
        &context.db,
        &user2,
        Some("123.123.123.69"),
        Some("IE Something?"),
    )
    .await;

    let sessions = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
            with_deleted: None,
            with_expired: None,
            user_id: None,
            search: Some(user2.email.clone()),
            sort: None,
            order: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();

    assert_eq!(sessions.len(), 1);
}

#[async_std::test]
async fn test_find_sessions_by_user_agent() {
    let context = Context::mock_sqlite().await;
    let repository = crate::tests::get_repo(&context).await;
    let user = super::get_users(&context).await.get(0).unwrap().clone();
    let _sessions = super::create_sessions(&context, &user).await;

    let sessions = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
            with_deleted: None,
            with_expired: None,
            user_id: None,
            search: Some("brave".to_string()),
            sort: None,
            order: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();

    assert_eq!(sessions.len(), 1);
}
