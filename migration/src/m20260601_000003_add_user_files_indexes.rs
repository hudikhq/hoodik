use sea_orm::{ConnectionTrait, FromQueryResult, Statement};
use sea_orm_migration::prelude::*;

use crate::m20230409_101730_create_user_files::UserFiles;

/// Two indexes that become hot once sharing ships.
///
/// `(user_id, is_owner)` powers "Shared with me" / "My shares".
/// `(file_id, user_id)` UNIQUE is the conflict target for the upsert in
/// `POST /api/shares` and makes the `permission()` helper a single keyed
/// lookup.
///
/// Pre-flight guard: aborts with a clear error if the table already
/// contains duplicate `(file_id, user_id)` pairs from a pre-1.16 install
/// hitting some bug. None expected; this is a safety belt.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[derive(FromQueryResult)]
struct DupeCount {
    duplicates: i64,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        let backend = conn.get_database_backend();

        let stmt = Statement::from_string(
            backend,
            "SELECT COUNT(*) AS duplicates FROM ( \
                SELECT 1 FROM user_files \
                GROUP BY file_id, user_id \
                HAVING COUNT(*) > 1 \
             ) AS dupes"
                .to_string(),
        );

        if let Some(row) = DupeCount::find_by_statement(stmt).one(conn).await? {
            if row.duplicates > 0 {
                return Err(DbErr::Custom(format!(
                    "user_files contains {} duplicate (file_id, user_id) pair(s); the unique \
                     index this migration adds cannot be created until each pair is unique. \
                     Remove the duplicate rows (keep one per (file_id, user_id)) and re-run.",
                    row.duplicates
                )));
            }
        }

        manager
            .create_index(
                Index::create()
                    .name("idx_user_files_user_owner")
                    .table(UserFiles::Table)
                    .col(UserFiles::UserId)
                    .col(UserFiles::IsOwner)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uniq_user_files_file_user")
                    .table(UserFiles::Table)
                    .col(UserFiles::FileId)
                    .col(UserFiles::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("uniq_user_files_file_user")
                    .table(UserFiles::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_user_files_user_owner")
                    .table(UserFiles::Table)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
