use super::files::{ActiveModel as FileActiveModel, Model as File};
use super::user_files::{ActiveModel as UserFileActiveModel, Model as UserFile};
use super::users::{ActiveModel as UserActiveModel, Model as User};
use crate::{user_files, Uuid};

use chrono::Utc;
use sea_orm::{ActiveValue, EntityTrait};

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
        email: ActiveValue::Set(email.to_string()),
        password: ActiveValue::Set(Some("".to_string())),
        secret: ActiveValue::NotSet,
        pubkey: ActiveValue::Set(pubkey.unwrap_or_default()),
        fingerprint: ActiveValue::Set("".to_string()),
        encrypted_private_key: ActiveValue::NotSet,
        email_verified_at: ActiveValue::Set(Some(Utc::now().naive_utc())),
        created_at: ActiveValue::Set(Utc::now().naive_utc()),
        updated_at: ActiveValue::Set(Utc::now().naive_utc()),
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
        file_created_at: ActiveValue::Set(Utc::now().naive_utc()),
        created_at: ActiveValue::Set(Utc::now().naive_utc()),
        finished_upload_at: ActiveValue::Set(Some(Utc::now().naive_utc())),
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
        created_at: ActiveValue::Set(Utc::now().naive_utc()),
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
