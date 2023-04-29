use super::users::{ActiveModel as UserActiveModel, Model as User};
use chrono::Utc;
use sea_orm::{ActiveValue, EntityTrait};

/// Create a user in the database.
pub async fn create_user<T: super::ConnectionTrait>(db: &T, email: &str) -> User {
    let id = crate::Uuid::new_v4();

    let user = UserActiveModel {
        id: ActiveValue::Set(id),
        email: ActiveValue::Set(email.to_string()),
        password: ActiveValue::Set(Some("".to_string())),
        secret: ActiveValue::NotSet,
        pubkey: ActiveValue::Set("".to_string()),
        fingerprint: ActiveValue::Set("".to_string()),
        encrypted_private_key: ActiveValue::NotSet,
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
