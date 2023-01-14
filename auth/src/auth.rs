use crate::data::create_user::CreateUser;
use chrono::{Duration, Utc};
use context::Context;
use entity::{
    sessions, users, ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter,
};
use error::{AppResult, Error};

pub struct Auth<'ctx> {
    pub context: &'ctx Context,
}

impl<'ctx> Auth<'ctx> {
    pub async fn create(&self, data: CreateUser) -> AppResult<users::Model> {
        let active_model = data.into_active_model()?;

        active_model
            .insert(&self.context.db)
            .await
            .map_err(Error::from)
    }

    pub async fn get_by_id(&self, id: i32) -> AppResult<users::Model> {
        users::Entity::find_by_id(id)
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{}", id)))
    }

    pub async fn get_by_email(&self, email: &str) -> AppResult<users::Model> {
        users::Entity::find()
            .filter(users::Column::Email.contains(email))
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("user_not_found:{}", email)))
    }

    pub async fn generate_session(
        &self,
        user: &users::Model,
        remember: bool,
    ) -> AppResult<sessions::Model> {
        let expires_at = match remember {
            true => Utc::now() + Duration::days(365),
            false => Utc::now() + Duration::minutes(10),
        };

        let active_model = sessions::ActiveModel {
            id: ActiveValue::NotSet,
            user_id: ActiveValue::Set(user.id),
            token: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
            csrf: ActiveValue::Set(uuid::Uuid::new_v4().to_string()),
            created_at: ActiveValue::Set(Utc::now().naive_utc()),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
            expires_at: ActiveValue::Set(expires_at.naive_utc()),
        };

        active_model
            .insert(&self.context.db)
            .await
            .map_err(Error::from)
    }
}
