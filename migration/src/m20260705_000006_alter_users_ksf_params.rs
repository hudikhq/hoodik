use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;

/// Per-user OPAQUE KSF parameters. `export_key` is a function of these, so
/// raising the work factor later would change every account's `export_key` at
/// once — a mass lockout. Recording the parameters each account registered
/// under lets a raise be lazy: log the user in with their stored values, then
/// silently re-register at the higher ones. Defaults backfill every existing
/// row to today's constants (Argon2id, m=64 MiB, t=3, p=1), so nothing changes
/// for accounts already migrated.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

const COLUMNS: [(&str, i32); 4] = [
    ("ksf_m_cost", 65536),
    ("ksf_t_cost", 3),
    ("ksf_p_cost", 1),
    ("opaque_protocol_version", 1),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("ksf_algorithm"))
                            .string_len(32)
                            .not_null()
                            .default("argon2id"),
                    )
                    .to_owned(),
            )
            .await?;

        for (name, default) in COLUMNS {
            manager
                .alter_table(
                    Table::alter()
                        .table(Users::Table)
                        .add_column(
                            ColumnDef::new(Alias::new(name))
                                .integer()
                                .not_null()
                                .default(default),
                        )
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (name, _) in COLUMNS.into_iter().rev() {
            manager
                .alter_table(
                    Table::alter()
                        .table(Users::Table)
                        .drop_column(Alias::new(name))
                        .to_owned(),
                )
                .await?;
        }

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Alias::new("ksf_algorithm"))
                    .to_owned(),
            )
            .await
    }
}
