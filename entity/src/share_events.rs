use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "share_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub sender_id: Option<Uuid>,
    pub recipient_id: Option<Uuid>,
    /// NULL for account-level events (a `key_rotation` belongs to no file);
    /// file-scoped events set it and still cascade on file delete.
    pub file_id: Option<Uuid>,
    pub action: String,
    pub share_role_before: Option<String>,
    pub share_role_after: Option<String>,
    pub created_at: i64,
    pub prev_event_hash: Option<Vec<u8>>,
    pub this_event_hash: Vec<u8>,
    pub sender_signature: Option<Vec<u8>>,
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
}

impl Related<super::files::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Files.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
