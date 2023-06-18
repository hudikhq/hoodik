use chrono::Utc;
use context::Context;

#[async_std::test]
async fn test_find_sessions() {
    let context = Context::mock_sqlite().await;
    let repository = crate::tests::get_repo(&context).await;
    let user = super::get_users(&context).await.get(0).unwrap().clone();
    let _sessions = super::create_sessions(&context, &user).await;

    let paginated = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
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
    let sessions = paginated.sessions;

    assert_eq!(sessions.len(), 5);
}

#[async_std::test]
async fn test_find_sessions_by_ip() {
    let context = Context::mock_sqlite().await;
    let repository = crate::tests::get_repo(&context).await;
    let user = super::get_users(&context).await.get(0).unwrap().clone();
    let _sessions = super::create_sessions(&context, &user).await;

    let paginated = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
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
    let sessions = paginated.sessions;

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
        false,
    )
    .await;

    let paginated = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
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
    let sessions = paginated.sessions;

    assert_eq!(sessions.len(), 1);
}

#[async_std::test]
async fn test_find_sessions_by_user_agent() {
    let context = Context::mock_sqlite().await;
    let repository = crate::tests::get_repo(&context).await;
    let user = super::get_users(&context).await.get(0).unwrap().clone();
    let _sessions = super::create_sessions(&context, &user).await;

    let paginated = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
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
    let sessions = paginated.sessions;

    assert_eq!(sessions.len(), 1);
}

#[async_std::test]
async fn test_session_killing() {
    let context = Context::mock_sqlite().await;
    let repository = crate::tests::get_repo(&context).await;
    let user = super::get_users(&context).await.get(0).unwrap().clone();
    let sessions = super::create_sessions(&context, &user).await;

    let session = sessions.get(0).unwrap().clone();

    let _ = repository.sessions().kill(session.id).await.unwrap();

    let paginated = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
            with_expired: Some(true),
            user_id: None,
            search: None,
            sort: None,
            order: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();

    let sessions = paginated.sessions;
    let should_be_expired = sessions.iter().find(|s| s.id == session.id).unwrap();

    assert!(should_be_expired.expires_at <= Utc::now().timestamp());

    repository.sessions().kill_for(user.id).await.unwrap();

    let paginated = repository
        .sessions()
        .find(crate::data::sessions::search::Search {
            with_expired: Some(true),
            user_id: Some(user.id),
            search: None,
            sort: None,
            order: None,
            limit: None,
            offset: None,
        })
        .await
        .unwrap();

    let sessions = paginated.sessions;

    for session in sessions {
        assert!(!session.active);
        assert!(session.expires_at <= Utc::now().timestamp());
    }
}
