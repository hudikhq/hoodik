use sea_orm_migration::prelude::*;

use crate::m20230409_101730_create_user_files::UserFiles;

/// Adds the three-tier permission column to `user_files`.
///
/// `share_role` is the column the `permission()` helper folds into the
/// `SharePermission` enum (`entity/permission.rs`). The CHECK constraint
/// keeps the closed set explicit at the schema layer; future tiers can be
/// added by amending the CHECK in a follow-up migration without reshaping
/// the column.
///
/// Backfill: existing rows are all `is_owner = true` on pre-1.16 servers
/// (no non-owner rows exist), so they receive `'co-owner'` (the highest
/// non-owner tier). The value is moot on owner rows — `permission()`
/// returns `Owner` whenever `is_owner = true` regardless of `share_role`.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserFiles::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("share_role"))
                            .string_len(16)
                            .not_null()
                            .default("reader")
                            .check(
                                Expr::col(Alias::new("share_role"))
                                    .is_in(["reader", "editor", "co-owner"]),
                            ),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "UPDATE user_files SET share_role = 'co-owner' WHERE is_owner = TRUE",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserFiles::Table)
                    .drop_column(Alias::new("share_role"))
                    .to_owned(),
            )
            .await
    }
}
