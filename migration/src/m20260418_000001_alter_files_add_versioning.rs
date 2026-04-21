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
                        ColumnDef::new(Alias::new("active_version"))
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .add_column(ColumnDef::new(Alias::new("pending_version")).integer())
                    .to_owned(),
            )
            .await?;

        // Pending upload's expected chunk count. NULL when no pending upload
        // is in progress; auto-finalize fires when chunks_stored == pending_chunks.
        // Kept separate from `chunks` (which always reflects the active version)
        // so readers don't see a momentarily wrong chunk count mid-edit.
        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .add_column(ColumnDef::new(Alias::new("pending_chunks")).big_integer())
                    .to_owned(),
            )
            .await?;

        // Pending upload's expected total size. Same rationale as pending_chunks
        // — `size` always reflects the active version so the file browser keeps
        // showing the correct size during a save.
        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .add_column(ColumnDef::new(Alias::new("pending_size")).big_integer())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .drop_column(Alias::new("pending_size"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .drop_column(Alias::new("pending_chunks"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .drop_column(Alias::new("pending_version"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .drop_column(Alias::new("active_version"))
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
