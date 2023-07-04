use crate::{data::query::Query, mock::create_file, repository::Repository};
use context::Context;

#[actix_web::test]
async fn create_recursive_mess_to_test_move_many() {
    let context = Context::mock_sqlite().await;
    let repository = Repository::new(&context.db);
    let user = entity::mock::create_user(&context.db, "first@test.com", None).await;

    let mut manual = vec![];

    let dir = create_file(&context, &user, "root.dir", None, Some("dir"))
        .await
        .unwrap();
    let dir_id = dir.id.clone();
    manual.push(dir);

    let file = create_file(
        &context,
        &user,
        "root.json1",
        None,
        Some("application/json"),
    )
    .await
    .unwrap();
    manual.push(file);

    let file = create_file(
        &context,
        &user,
        "root.json2",
        None,
        Some("application/json"),
    )
    .await
    .unwrap();
    let _file2_id = file.id.clone();
    manual.push(file);

    let file = create_file(
        &context,
        &user,
        "root.json3",
        None,
        Some("application/json"),
    )
    .await
    .unwrap();
    manual.push(file);

    let file = create_file(
        &context,
        &user,
        "root.json4",
        None,
        Some("application/json"),
    )
    .await
    .unwrap();
    manual.push(file);

    let dir = create_file(&context, &user, "root.dir.dir2", Some(dir_id), Some("dir"))
        .await
        .unwrap();
    let dir2_id = dir.id.clone();
    manual.push(dir);

    let dir3 = create_file(
        &context,
        &user,
        "root.dir.dir2.dir3",
        Some(dir2_id),
        Some("dir"),
    )
    .await
    .unwrap();
    manual.push(dir3);

    let ids = manual.iter().map(|f| f.id.clone()).collect::<Vec<_>>();

    let moved = repository
        .manage(user.id)
        .move_many(ids.clone(), None)
        .await
        .unwrap();

    for file in manual.iter() {
        assert!(file.is_owner);
    }

    assert_ne!(moved, 0);

    assert_eq!(manual.len(), moved as usize);

    let moved = repository
        .manage(user.id)
        .find(Query {
            dir_id: None,
            ..Default::default()
        })
        .await
        .unwrap()
        .children;

    assert_eq!(moved.len(), manual.len());

    for file in moved.iter() {
        assert!(file.file_id.is_none());
    }
}
