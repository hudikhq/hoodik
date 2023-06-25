use context::Context;

#[async_std::test]
async fn test_file_stats() {
    let context = Context::mock_sqlite().await;
    let repository = super::get_repo(&context).await;
    super::get_users(&context).await;

    let user = entity::mock::create_user(&context.db, "eleven@test.com", None).await;

    entity::mock::create_file(&context.db, &user, "one", "application/json", None).await; // 100b
    entity::mock::create_file(&context.db, &user, "one", "application/json", None).await; // 100b

    entity::mock::create_file(&context.db, &user, "one", "image/png", None).await; // 100b
    entity::mock::create_file(&context.db, &user, "one", "image/png", None).await; // 100b

    entity::mock::create_file(&context.db, &user, "one", "dir", None).await; // 0b

    let stats = repository.files().stats().await.unwrap();

    assert_eq!(stats.len(), 2);

    for stat in stats {
        match stat.mime.as_str() {
            "application/json" => {
                assert_eq!(stat.count, 2);
                assert_eq!(stat.size, 200);
            }
            "image/png" => {
                assert_eq!(stat.count, 2);
                assert_eq!(stat.size, 200);
            }
            _ => unreachable!(),
        }
    }

    let stats = repository.files().stats_for(user.id).await.unwrap();

    assert_eq!(stats.len(), 2);

    for stat in stats {
        match stat.mime.as_str() {
            "application/json" => {
                assert_eq!(stat.count, 2);
                assert_eq!(stat.size, 200);
            }
            "image/png" => {
                assert_eq!(stat.count, 2);
                assert_eq!(stat.size, 200);
            }
            _ => unreachable!(),
        }
    }
}
