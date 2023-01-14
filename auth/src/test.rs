use context::Context;

use crate::{contract::AuthContract, data::CreateUser, Auth};

fn create_lib<'ctx>(context: &'ctx Context) -> Auth<'ctx> {
    Auth::<'ctx> { context }
}

#[async_std::test]
async fn auth_create_user() {
    let context = Context::mock_sqlite().await;
    let lib = create_lib(&context);

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        token: None,
    };

    let response = lib.create(create_user).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let user = response.unwrap();

    let response = lib.get_by_id(user.id).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let user_by_id = response.unwrap();

    assert_eq!(user.email, user_by_id.email);
}
