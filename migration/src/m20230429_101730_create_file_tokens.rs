use super::m20220101_000001_create_users::Users;
use super::m20230409_091730_create_files::Files;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut foreign_key_file_id = ForeignKey::create();
        foreign_key_file_id
            .from(FileTokens::Table, FileTokens::FileId)
            .to(Files::Table, Files::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(FileTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FileTokens::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FileTokens::FileId).uuid().not_null())
                    .foreign_key(&mut foreign_key_file_id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FileTokens::Table).to_owned())
            .await
    }
}

// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub enum FileTokens {
    Table,
    Id,
    FileId,
}
