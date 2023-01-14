use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users", schema_name = "public")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i32,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    #[serde(skip_serializing)]
    pub secret: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::session::Entity")]
    Session,
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
