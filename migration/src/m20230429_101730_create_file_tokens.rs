use super::m20230409_091730_create_files::Files;
use crate::m20230429_091730_create_tokens::Tokens;

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

        let mut foreign_key_token_id = ForeignKey::create();
        foreign_key_token_id
            .from(FileTokens::Table, FileTokens::TokenId)
            .to(Tokens::Table, Tokens::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(FileTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FileTokens::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FileTokens::FileId).uuid().not_null())
                    .col(ColumnDef::new(FileTokens::TokenId).uuid().not_null())
                    .col(ColumnDef::new(FileTokens::Weight).integer().not_null())
                    .foreign_key(&mut foreign_key_file_id)
                    .foreign_key(&mut foreign_key_token_id)
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
    TokenId,
    Weight,
}
