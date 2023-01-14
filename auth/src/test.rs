use chrono::{Duration, Utc};
use context::Context;
use log::debug;

use crate::{
    auth::Auth,
    contract::AuthProviderContract,
    data::{create_user::CreateUser, credentials::Credentials},
    providers::credentials::CredentialsProvider,
};

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

#[async_std::test]
async fn test_credentials_valid() {
    let context = Context::mock_sqlite().await;
    let auth = create_lib(&context);

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        token: None,
    };

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        token: None,
        remember: Some(true),
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

    let response = auth.create(create_user).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let user = response.unwrap();

    let response = credentials_provider.authenticate().await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let authenticated = response.unwrap();

    assert!(authenticated.session.expires_at > (Utc::now() + Duration::minutes(20)).naive_utc());
    assert_eq!(authenticated.user.id, user.id);
}

#[async_std::test]
async fn test_credentials_invalid() {
    let context = Context::mock_sqlite().await;
    let auth = create_lib(&context);

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        token: None,
    };

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("wrong-password".to_string()),
        token: None,
        remember: Some(true),
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

    let response = auth.create(create_user).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let _user = response.unwrap();

    let response = credentials_provider.authenticate().await;

    if let Err(e) = &response {
        debug!("Errored: {:#?}", e);

        assert_eq!(
            e,
            &error::Error::Unauthorized("invalid_credentials".to_string())
        );
    } else {
        panic!("Authentication passed with incorrect credentials")
    }
}
