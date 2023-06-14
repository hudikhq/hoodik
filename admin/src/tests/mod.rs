use context::{Context, DatabaseConnection};

use crate::repository::Repository;

mod files;
mod invitations;
mod sessions;
mod users;

pub(crate) async fn get_repo<'ctx>(context: &'ctx Context) -> Repository<'ctx, DatabaseConnection> {
    let repository = Repository::new(&context, &context.db);

    repository
}

pub(crate) async fn get_users<'ctx>(context: &'ctx Context) -> Vec<entity::users::Model> {
    let mut users = vec![];

    users.push(entity::mock::create_user(&context.db, "one@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "two@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "three@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "four@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "five@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "six@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "seven@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "eight@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "nine@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "ten@test.com", None).await);

    users
}

pub(crate) async fn create_sessions<'ctx>(
    context: &'ctx Context,
    user: &entity::users::Model,
) -> Vec<entity::sessions::Model> {
    let mut sessions = vec![];

    sessions.push(
        entity::mock::create_session(
            &context.db,
            &user,
            Some("123.123.123.1"),
            Some("Mozilla Something?"),
        )
        .await,
    );
    sessions.push(
        entity::mock::create_session(
            &context.db,
            &user,
            Some("123.123.123.2"),
            Some("Chrome Something?"),
        )
        .await,
    );
    sessions.push(
        entity::mock::create_session(
            &context.db,
            &user,
            Some("123.123.123.3"),
            Some("Edge Something?"),
        )
        .await,
    );
    sessions.push(
        entity::mock::create_session(
            &context.db,
            &user,
            Some("123.123.123.4"),
            Some("Brave Something?"),
        )
        .await,
    );
    sessions.push(
        entity::mock::create_session(
            &context.db,
            &user,
            Some("123.123.123.5"),
            Some("Safari Something?"),
        )
        .await,
    );

    sessions
}
