use sea_orm_migration::prelude::*;

use crate::m20230409_091730_create_files::Files;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("cipher"))
                            .string_len(32)
                            .not_null()
                            .default("ascon128a"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .drop_column(Alias::new("cipher"))
                    .to_owned(),
            )
            .await
    }
}
