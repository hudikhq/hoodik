use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "links")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub file_id: Uuid,
    /// Signature the creator made over the shared file_id when the link
    /// was created; used to prove ownership without exposing the key.
    pub signature: String,
    /// Counts every download attempt, including failures. Not decremented.
    pub downloads: i32,
    /// Encrypted under the link key — not refreshed if the file is
    /// renamed after the link is created.
    pub encrypted_name: String,
    /// Link's symmetric key, RSA-encrypted under the creator's pubkey so
    /// only the creator can recover it from the DB; recipients receive
    /// it out-of-band (URL fragment).
    pub encrypted_link_key: String,
    pub encrypted_thumbnail: Option<String>,
    /// File's symmetric key wrapped with the link key. Never served to
    /// clients; decrypted server-side during a link download stream.
    #[serde(skip_serializing)]
    pub encrypted_file_key: Option<String>,
    pub created_at: i64,
    /// Cron periodically purges expired rows' file metadata and
    /// `encrypted_file_key` to cut the attack surface on stale links.
    pub expires_at: Option<i64>,
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
    #[sea_orm(
        belongs_to = "super::files::Entity",
        from = "Column::FileId",
        to = "super::files::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Files,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl Related<super::files::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Files.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
