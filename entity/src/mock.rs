use super::files::{ActiveModel as FileActiveModel, Model as File};
use super::user_files::{ActiveModel as UserFileActiveModel, Model as UserFile};
use super::users::{ActiveModel as UserActiveModel, Model as User};
use crate::{invitations, user_files, Uuid};

use chrono::{Duration, Utc};
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, QueryFilter};

/// Create a user in the database.
pub async fn create_user<T: super::ConnectionTrait>(
    db: &T,
    email: &str,
    pubkey: Option<String>,
) -> User {
    let id = Uuid::new_v4();

    let user = UserActiveModel {
        id: ActiveValue::Set(id),
        role: ActiveValue::NotSet,
        quota: ActiveValue::NotSet,
        email: ActiveValue::Set(email.to_string()),
        password: ActiveValue::Set(Some("".to_string())),
        secret: ActiveValue::NotSet,
        pubkey: ActiveValue::Set(pubkey.unwrap_or_default()),
        fingerprint: ActiveValue::Set("".to_string()),
        encrypted_private_key: ActiveValue::NotSet,
        email_verified_at: ActiveValue::Set(Some(Utc::now().timestamp())),
        created_at: ActiveValue::Set(Utc::now().timestamp()),
        updated_at: ActiveValue::Set(Utc::now().timestamp()),
    };

    crate::users::Entity::insert(user)
        .exec_without_returning(db)
        .await
        .unwrap();

    crate::users::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap()
}

/// Create session for the user
pub async fn create_session<T: super::ConnectionTrait>(
    db: &T,
    user: &User,
    ip: Option<&str>,
    user_agent: Option<&str>,
    expired: bool,
) -> super::sessions::Model {
    let id = Uuid::new_v4();

    let expires_at = if expired {
        Utc::now().naive_utc() - Duration::days(1)
    } else {
        Utc::now().naive_utc() + Duration::minutes(5)
    }
    .timestamp();

    let session = super::sessions::ActiveModel {
        id: ActiveValue::Set(id),
        user_id: ActiveValue::Set(user.id),
        user_agent: ActiveValue::Set(
            user_agent
                .map(|user_agent| user_agent.to_string())
                .unwrap_or_else(|| "".to_string()),
        ),
        device_id: ActiveValue::Set(Uuid::new_v4()),
        ip: ActiveValue::Set(
            ip.map(|ip| ip.to_string())
                .unwrap_or_else(|| "127.0.0.1".to_string()),
        ),
        refresh: ActiveValue::Set(Some(Uuid::new_v4())),
        created_at: ActiveValue::Set((Utc::now().naive_utc() - Duration::minutes(5)).timestamp()),
        updated_at: ActiveValue::Set((Utc::now().naive_utc() - Duration::minutes(5)).timestamp()),
        expires_at: ActiveValue::Set(expires_at),
    };

    super::sessions::Entity::insert(session)
        .exec_without_returning(db)
        .await
        .unwrap();

    super::sessions::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap()
}

/// Create a file in the database
pub async fn create_file<T: super::ConnectionTrait>(
    db: &T,
    user: &super::users::Model,
    name: &str,
    mime: &str,
    file_id: Option<Uuid>,
) -> (File, UserFile) {
    let id = Uuid::new_v4();
    let mut size = None;
    let mut chunks = None;

    if mime != "dir" {
        size = Some(100);
        chunks = Some(1);
    }

    let file = FileActiveModel {
        id: ActiveValue::Set(id),
        mime: ActiveValue::Set(mime.to_string()),
        file_id: ActiveValue::Set(file_id),
        name_hash: ActiveValue::Set(cryptfns::sha256::digest(name.as_bytes())),
        encrypted_name: ActiveValue::Set(name.to_string()),
        encrypted_thumbnail: ActiveValue::NotSet,
        size: ActiveValue::Set(size),
        chunks: ActiveValue::Set(chunks),
        chunks_stored: ActiveValue::Set(chunks),
        file_created_at: ActiveValue::Set(Utc::now().timestamp()),
        created_at: ActiveValue::Set(Utc::now().timestamp()),
        finished_upload_at: ActiveValue::Set(Some(Utc::now().timestamp())),
    };

    crate::files::Entity::insert(file)
        .exec_without_returning(db)
        .await
        .unwrap();

    let file = crate::files::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap();

    let user_file = UserFileActiveModel {
        id: ActiveValue::Set(id),
        file_id: ActiveValue::Set(file.id),
        user_id: ActiveValue::Set(user.id),
        is_owner: ActiveValue::Set(true),
        encrypted_key: ActiveValue::Set(name.to_string()),
        created_at: ActiveValue::Set(Utc::now().timestamp()),
        expires_at: ActiveValue::NotSet,
    };

    user_files::Entity::insert(user_file)
        .exec_without_returning(db)
        .await
        .unwrap();

    let user_file = user_files::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap();

    (file, user_file)
}

/// Create invitation for the user in database
pub async fn create_invitation<T: super::ConnectionTrait>(
    db: &T,
    email: &str,
) -> super::invitations::Model {
    let id = Uuid::new_v4();

    let invitation = super::invitations::ActiveModel {
        id: ActiveValue::Set(id),
        user_id: ActiveValue::NotSet,
        email: ActiveValue::Set(email.to_string()),
        role: ActiveValue::NotSet,
        quota: ActiveValue::NotSet,
        created_at: ActiveValue::Set(Utc::now().timestamp()),
        expires_at: ActiveValue::Set((Utc::now() + chrono::Duration::days(7)).timestamp()),
    };

    invitations::Entity::insert(invitation)
        .exec_without_returning(db)
        .await
        .unwrap();

    invitations::Entity::find()
        .filter(invitations::Column::Id.eq(id))
        .one(db)
        .await
        .unwrap()
        .unwrap()
}
