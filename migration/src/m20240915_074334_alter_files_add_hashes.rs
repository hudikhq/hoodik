use sea_orm_migration::prelude::*;

use crate::m20230409_091730_create_files::Files;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter()
            .table(Files::Table)
            .add_column(
                ColumnDef::new(Alias::new("md5"))
                    .string()
                    .null()
            )
            .to_owned())
            .await?;

            manager
            .alter_table(Table::alter()
            .table(Files::Table)
            .add_column(
                ColumnDef::new(Alias::new("sha1"))
                    .string()
                    .null()
            )
            .to_owned())
            .await?;

            manager
            .alter_table(Table::alter()
            .table(Files::Table)
            .add_column(
                ColumnDef::new(Alias::new("sha256"))
                    .string()
                    .null()
            )
            .to_owned())
            .await?;

            manager
            .alter_table(Table::alter()
            .table(Files::Table)
            .add_column(
                ColumnDef::new(Alias::new("blake2b"))
                    .string()
                    .null()
            )
            .to_owned())
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(Table::alter()
            .table(Files::Table)
            .drop_column(Alias::new("md5"))
            .to_owned())
            .await?;

        manager
            .alter_table(Table::alter()
            .table(Files::Table)
            .drop_column(Alias::new("sha1"))
            .to_owned())
            .await?;

        manager
            .alter_table(Table::alter()
            .table(Files::Table)
            .drop_column(Alias::new("sha256"))
            .to_owned())
            .await?;

        manager
            .alter_table(Table::alter()
            .table(Files::Table)
            .drop_column(Alias::new("blake2b"))
            .to_owned())
            .await?;

        Ok(())
    }
}
