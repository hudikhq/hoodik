use sea_orm_migration::prelude::*;

use crate::{m20220101_000001_create_users::Users, m20230409_091730_create_files::Files};

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut foreign_key_user_id = ForeignKey::create();
        foreign_key_user_id
            .from(Links::Table, Links::UserId)
            .to(Users::Table, Users::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        let mut foreign_key_file_id = ForeignKey::create();
        foreign_key_file_id
            .from(Links::Table, Links::FileId)
            .to(Files::Table, Files::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(Links::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Links::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Links::UserId).uuid().not_null())
                    .col(ColumnDef::new(Links::FileId).uuid().not_null())
                    .col(ColumnDef::new(Links::Signature).string().not_null())
                    .col(ColumnDef::new(Links::EncryptedName).string().not_null())
                    .col(ColumnDef::new(Links::EncryptedLinkKey).string().not_null())
                    .col(ColumnDef::new(Links::Downloads).integer().not_null())
                    .col(ColumnDef::new(Links::EncryptedThumbnail).text())
                    .col(ColumnDef::new(Links::EncryptedFileKey).text())
                    .col(ColumnDef::new(Links::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Links::ExpiresAt).big_integer())
                    .foreign_key(&mut foreign_key_user_id)
                    .foreign_key(&mut foreign_key_file_id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Links::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum Links {
    Table,
    Id,
    UserId,
    FileId,
    Signature,
    Downloads,
    EncryptedName,
    EncryptedLinkKey,
    EncryptedThumbnail,
    EncryptedFileKey,
    CreatedAt,
    ExpiresAt,
}
