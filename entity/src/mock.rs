use super::users::{ActiveModel as UserActiveModel, Model as User};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue};

/// Create a user in the database.
pub async fn create_user<T: super::ConnectionTrait>(db: &T, email: &str) -> User {
    let user = UserActiveModel {
        id: ActiveValue::NotSet,
        email: ActiveValue::Set(email.to_string()),
        password: ActiveValue::Set(Some("".to_string())),
        secret: ActiveValue::NotSet,
        pubkey: ActiveValue::Set("".to_string()),
        fingerprint: ActiveValue::Set("".to_string()),
        encrypted_private_key: ActiveValue::NotSet,
        created_at: ActiveValue::Set(Utc::now().naive_utc()),
        updated_at: ActiveValue::Set(Utc::now().naive_utc()),
    };

    user.insert(db).await.unwrap()
}
