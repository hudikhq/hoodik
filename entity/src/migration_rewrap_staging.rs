use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Re-wrapped keys a legacy account stages while migrating, before the one-shot
/// `migration/complete` applies them. Splitting the re-wrap into batched
/// `migration/rewrap` writes keeps each request small; `complete` then reads the
/// whole set from here inside its transaction. A row targets exactly one of a
/// file (`file_id`) or a public link (`link_id`); the other stays null. Unique
/// on `(user_id, file_id)` and `(user_id, link_id)` so a retried batch replaces
/// its rows instead of duplicating them. `created_at` drives a TTL purge of sets
/// left behind by a migration the client never completed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "migration_rewrap_staging")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub file_id: Option<Uuid>,
    pub link_id: Option<Uuid>,
    pub encrypted_key: String,
    /// The owner's re-signature over the link's `file_id` under the new identity
    /// key. Set for link rows, null for file rows; verified by `complete`.
    pub signature: Option<String>,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
