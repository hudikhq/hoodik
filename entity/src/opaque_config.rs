use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Singleton (`id` is always [`Model::SINGLETON_ID`]) holding the server's
/// OPAQUE OPRF seed. `server_setup` is base64 and must never be exposed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "opaque_config")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i32,
    #[serde(skip_serializing)]
    pub server_setup: String,
}

impl Model {
    pub const SINGLETON_ID: i32 = 1;
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
