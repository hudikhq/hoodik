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

    users.push(entity::mock::create_user(&context.db, "1@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "2@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "3@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "4@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "5@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "6@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "7@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "8@test.com", None).await);
    users.push(entity::mock::create_user(&context.db, "9@test.com", None).await);

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
            false,
        )
        .await,
    );
    sessions.push(
        entity::mock::create_session(
            &context.db,
            &user,
            Some("123.123.123.2"),
            Some("Chrome Something?"),
            false,
        )
        .await,
    );
    sessions.push(
        entity::mock::create_session(
            &context.db,
            &user,
            Some("123.123.123.3"),
            Some("Edge Something?"),
            false,
        )
        .await,
    );
    sessions.push(
        entity::mock::create_session(
            &context.db,
            &user,
            Some("123.123.123.4"),
            Some("Brave Something?"),
            false,
        )
        .await,
    );
    sessions.push(
        entity::mock::create_session(
            &context.db,
            &user,
            Some("123.123.123.5"),
            Some("Safari Something?"),
            false,
        )
        .await,
    );

    sessions
}
