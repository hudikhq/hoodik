//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.6

use crate::JsonValue;
use chrono::NaiveDateTime;
use error::Error as AppError;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user_files")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub file_id: i32,
    pub user_id: i32,
    pub encrypted_key: String,
    pub is_owner: bool,
    pub created_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
}

impl TryFrom<&JsonValue> for Model {
    type Error = AppError;

    fn try_from(value: &JsonValue) -> Result<Self, Self::Error> {
        let id = value["user_files.id"]
            .as_i64()
            .ok_or(AppError::BadRequest("user_files.id".to_string()))? as i32;
        let file_id = value["user_files.file_id"]
            .as_i64()
            .ok_or(AppError::BadRequest("user_files.file_id".to_string()))?
            as i32;
        let user_id = value["user_files.user_id"]
            .as_i64()
            .ok_or(AppError::BadRequest("user_files.user_id".to_string()))?
            as i32;
        let encrypted_key = value["user_files.encrypted_key"]
            .as_str()
            .ok_or(AppError::BadRequest("user_files.encrypted_key".to_string()))?
            .to_string();
        let is_owner = value["user_files.is_owner"]
            .as_bool()
            .ok_or(AppError::BadRequest("user_files.is_owner".to_string()))?;
        let created_at = util::datetime::parse_into_naive_datetime_db(
            value["user_files.created_at"]
                .as_str()
                .ok_or(AppError::BadRequest("user_files.created_at".to_string()))?,
            None,
        )?;
        let expires_at = value["user_files.expires_at"]
            .as_str()
            .map(|s| {
                util::datetime::parse_into_naive_datetime_db(s, None)
                    .map(Some)
                    .unwrap_or(None)
            })
            .unwrap_or(None);

        Ok(Self {
            id,
            file_id,
            user_id,
            encrypted_key,
            is_owner,
            created_at,
            expires_at,
        })
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::files::Entity",
        from = "Column::FileId",
        to = "super::files::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Files,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::files::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Files.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
