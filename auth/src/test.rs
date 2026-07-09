use std::str::FromStr;

use actix_web::{http::header, HttpResponse};
use chrono::{Duration, Utc};
use context::{Context, SenderContract};
use entity::{users, ActiveValue, Uuid};
use log::debug;

use crate::{
    auth::Auth,
    contracts::{
        cookies::Cookies, provider::AuthProvider, register::Register, repository::Repository,
    },
    data::{create_user::CreateUser, credentials::Credentials},
    providers::credentials::CredentialsProvider,
};

fn create_lib<'ctx>(context: &'ctx Context) -> Auth<'ctx> {
    Auth::<'ctx> { context }
}

/// Insert a legacy (RSA + bcrypt) account directly. Registration no longer
/// creates these — only login-time migration touches them — so the tests that
/// exercise the credentials provider seed one at the data layer.
async fn seed_legacy_user(lib: &Auth<'_>, email: &str, password: &str) -> users::Model {
    let private = cryptfns::rsa::private::generate().unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let pubkey = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    let now = Utc::now().timestamp();
    lib.create_user(users::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        role: ActiveValue::NotSet,
        quota: ActiveValue::NotSet,
        email: ActiveValue::Set(email.to_string()),
        password: ActiveValue::Set(Some(util::password::hash(password))),
        secret: ActiveValue::NotSet,
        pubkey: ActiveValue::Set(pubkey),
        fingerprint: ActiveValue::Set(fingerprint),
        key_type: ActiveValue::Set("rsa".to_string()),
        wrapping_pubkey: ActiveValue::NotSet,
        security_version: ActiveValue::Set(0),
        opaque_password_file: ActiveValue::NotSet,
        encrypted_private_key: ActiveValue::Set(Some("legacy-encrypted-key".to_string())),
        email_verified_at: ActiveValue::Set(Some(now)),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
        share_notifications_enabled: ActiveValue::Set(true),
    })
    .await
    .unwrap()
}

/// Build the client half of a v2 signup: a fresh Curve25519 identity and its
/// OPAQUE registration upload, run through an in-process server setup so the
/// upload the server finishes is a real one.
fn create_curve25519_user(email: &str, password: &str) -> CreateUser {
    let ed_private = cryptfns::ed25519::private::generate().unwrap();
    let pubkey = cryptfns::ed25519::public::from_private(&ed_private).unwrap();
    let fingerprint = cryptfns::spki::fingerprint(&pubkey).unwrap();
    let wrapping_pubkey =
        cryptfns::ecdh::public::from_private(&cryptfns::ecdh::private::generate().unwrap()).unwrap();

    let server_setup = cryptfns::opaque::server_setup_new();
    let reg_start = cryptfns::opaque::client_registration_start(password.as_bytes()).unwrap();
    let reg_response =
        cryptfns::opaque::server_registration_start(&server_setup, &reg_start.message, email.as_bytes())
            .unwrap();
    let reg_finish = cryptfns::opaque::client_registration_finish(
        &reg_start.state,
        &reg_response,
        password.as_bytes(),
    )
    .unwrap();

    CreateUser {
        email: Some(email.to_string()),
        password: None,
        secret: None,
        token: None,
        pubkey: Some(pubkey),
        fingerprint: Some(fingerprint),
        key_type: Some("curve25519".to_string()),
        wrapping_pubkey: Some(wrapping_pubkey),
        encrypted_private_key: Some("encrypted-gibberish".to_string()),
        opaque_registration_upload: Some(reg_finish.message),
        invitation_id: None,
    }
}

#[async_std::test]
async fn auth_create_user() {
    let context = Context::mock_sqlite().await;
    let lib = create_lib(&context);

    let response = lib
        .register(create_curve25519_user("john@doe.com", "very-strong-password"))
        .await;

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
    assert_eq!(user_by_id.key_type, "curve25519");
    assert_eq!(user_by_id.security_version, 1);
}

#[async_std::test]
async fn test_credentials_valid() {
    let context = Context::mock_sqlite().await;
    let auth = create_lib(&context);

    let user = seed_legacy_user(&auth, "john@doe.com", "very-strong-password").await;

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        token: None,
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

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

    seed_legacy_user(&auth, "john@doe.com", "very-strong-password").await;

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("wrong-password".to_string()),
        token: None,
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

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

    seed_legacy_user(&auth, "john@doe.com", "very-strong-password").await;

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        token: None,
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

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
}

#[async_std::test]
async fn test_jwt_generate_and_claim() {
    let context = Context::mock_sqlite().await;
    let auth = create_lib(&context);

    seed_legacy_user(&auth, "john@doe.com", "very-strong-password").await;

    let credentials = Credentials {
        email: Some("john@doe.com".to_string()),
        password: Some("very-strong-password".to_string()),
        token: None,
    };

    let credentials_provider = CredentialsProvider::new(&auth, credentials);

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

    let response = auth
        .register(create_curve25519_user("john@doe.com", "very-strong-password"))
        .await;

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

    let response = auth
        .register(create_curve25519_user("john@doe.com", "very-strong-password"))
        .await;

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

    seed_legacy_user(&auth, "john@doe.com", "very-strong-password").await;

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

    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!()).unwrap();

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
