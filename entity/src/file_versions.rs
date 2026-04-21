//! `SeaORM` Entity for versioned-chunks history.
//!
//! One row per committed version of a file. The `version` integer doubles
//! as the on-disk directory name (`{data_dir}/{file_id}/v{version}/`) — the
//! repository layer never has to translate UUID ↔ directory.
//!
//! Pending uploads do NOT have a row here; they only exist via the
//! `pending_version` pointer on the parent file row. A row is inserted at
//! commit time, snapshotting the previously-active version's metadata.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "file_versions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub file_id: Uuid,
    /// Integer version number. Also names the on-disk directory `v{version}`.
    /// `(file_id, version)` is unique.
    pub version: i32,
    /// Saver attribution. NULL for anonymous link saves (A4) or after a
    /// user is deleted (FK SET NULL).
    pub user_id: Option<Uuid>,
    pub is_anonymous: bool,
    pub size: i64,
    pub chunks: i64,
    /// Per-version sha256 — used to restore `files.sha256` exactly when
    /// the active_version pointer flips back to this row. Other hash
    /// families are deprecated; clients only compute sha256.
    pub sha256: Option<String>,
    pub created_at: i64,
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
        on_delete = "SetNull"
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
