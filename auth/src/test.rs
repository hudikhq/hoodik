use std::str::FromStr;

use actix_web::{http::header, HttpResponse};
use chrono::{Duration, Utc};
use context::{Context, SenderContract};
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

fn get_pubkey_and_fingerprint() -> (Option<String>, Option<String>) {
    let pubkey = cryptfns::rsa::get_string_pubkey().unwrap();
    let fingerprint =
        cryptfns::rsa::fingerprint(cryptfns::rsa::public::from_str(&pubkey).unwrap()).unwrap();

    (Some(pubkey), Some(fingerprint))
}

#[async_std::test]
async fn auth_create_user() {
    let context = Context::mock_sqlite().await;
    let lib = create_lib(&context);
    let (pubkey, fingerprint) = get_pubkey_and_fingerprint();

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        pubkey,
        fingerprint,
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        token: None,
    };

    let response = lib.register(create_user).await;

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
    let (pubkey, fingerprint) = get_pubkey_and_fingerprint();

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        pubkey,
        fingerprint,
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        token: None,
    };

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        token: None,
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

    let response = auth.register(create_user).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let user = response.unwrap();

    let response = credentials_provider.authenticate("n/a", "127.0.0.1").await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let authenticated = response.unwrap();

    assert!(authenticated.session.expires_at > (Utc::now() + Duration::minutes(1)).timestamp());
    assert_eq!(authenticated.user.id, user.id);
}

#[async_std::test]
async fn test_credentials_invalid() {
    let context = Context::mock_sqlite().await;
    let auth = create_lib(&context);
    let (pubkey, fingerprint) = get_pubkey_and_fingerprint();

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        pubkey,
        fingerprint,
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        token: None,
    };

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("wrong-password".to_string()),
        token: None,
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

    let response = auth.register(create_user).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let _user = response.unwrap();

    let response = credentials_provider.authenticate("n/a", "127.0.0.1").await;

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

#[async_std::test]
async fn test_retrieve_authenticated_session_by_token_and_csrf() {
    let context = Context::mock_sqlite().await;
    let auth = create_lib(&context);
    let (pubkey, fingerprint) = get_pubkey_and_fingerprint();

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        pubkey,
        fingerprint,
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        token: None,
    };

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        token: None,
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

    let response = auth.register(create_user).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let response = credentials_provider.authenticate("n/a", "127.0.0.1").await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let authenticated = response.unwrap();
    let session = authenticated.session.clone();

    let response = auth.get_by_refresh(session.refresh.unwrap()).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let _authenticated = response.unwrap();

    // println!("{:#?}", authenticated);
}

#[async_std::test]
async fn test_jwt_generate_and_claim() {
    let context = Context::mock_sqlite().await;
    let auth = create_lib(&context);
    let (pubkey, fingerprint) = get_pubkey_and_fingerprint();

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        pubkey,
        fingerprint,
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        token: None,
    };

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        token: None,
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

    let response = auth.register(create_user).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let response = credentials_provider.authenticate("n/a", "127.0.0.1").await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let authenticated = response.unwrap();

    let jwt = crate::jwt::generate(&authenticated, module_path!(), "some-secret").unwrap();

    let response = crate::jwt::extract(&jwt, "some-secret");

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }
}

#[async_std::test]
async fn test_register_and_send_email() {
    let context = Context::mock_sqlite().await;
    let context = Context::add_mock_sender(context);
    let auth = create_lib(&context);

    let (pubkey, fingerprint) = get_pubkey_and_fingerprint();

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        pubkey,
        fingerprint,
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        token: None,
    };

    let response = auth.register(create_user).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let id = context
        .sender
        .unwrap()
        .find("Account activation token:")
        .unwrap()
        .replace("Account activation token: ", "");

    assert!(entity::Uuid::from_str(&id).is_ok());
}

#[async_std::test]
async fn test_activate_user() {
    let context = Context::mock_sqlite().await;
    let context = Context::add_mock_sender(context);
    let auth = create_lib(&context);

    let (pubkey, fingerprint) = get_pubkey_and_fingerprint();

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        pubkey,
        fingerprint,
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        token: None,
    };

    let response = auth.register(create_user).await;

    if let Err(e) = response {
        panic!("Errored: {:#?}", e);
    }

    let id = context
        .sender
        .as_ref()
        .unwrap()
        .find("Account activation token:")
        .unwrap()
        .replace("Account activation token: ", "");

    let id = entity::Uuid::from_str(&id).unwrap();

    let activated_user = auth.activate(id).await.unwrap();

    assert_eq!(activated_user.email, "john@doe.com");
    assert!(activated_user.email_verified_at.is_some());
}

#[async_std::test]
async fn test_set_cookie_for_both() {
    let context = Context::mock_sqlite().await;
    let context = Context::add_mock_sender(context);
    let auth = create_lib(&context);

    let (pubkey, fingerprint) = get_pubkey_and_fingerprint();

    let create_user = CreateUser {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        secret: None,
        pubkey,
        fingerprint,
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        token: None,
    };

    auth.register(create_user).await.unwrap();

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        token: None,
    };
    let credentials_provider = CredentialsProvider::new(&auth, credentials);
    let authenticated = credentials_provider
        .authenticate("n/a", "127.0.0.1")
        .await
        .unwrap();

    let (jwt, refresh) = auth
        .manage_cookies(&authenticated, module_path!())
        .await
        .unwrap();

    let mut res = HttpResponse::Ok();

    res.cookie(jwt.clone());
    res.cookie(refresh.clone());

    let res = res.finish();

    let mut headers = res.headers().get_all(header::SET_COOKIE);

    assert!(headers.len() == 2);

    let res_jwt = headers.next().unwrap();
    let res_refresh = headers.next().unwrap();

    assert_eq!(res_jwt.to_str().unwrap(), jwt.to_string());
    assert_eq!(res_refresh.to_str().unwrap(), refresh.to_string());
}
