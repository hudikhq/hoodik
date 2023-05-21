use sea_orm_migration::prelude::*;

use crate::{m20220101_000001_create_users::Users, m20230409_091730_create_files::Files};

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut foreign_key_user_id = ForeignKey::create();
        foreign_key_user_id
            .from(Link::Table, Link::FileId)
            .to(Users::Table, Users::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);
        let mut foreign_key_file_id = ForeignKey::create();
        foreign_key_file_id
            .from(Link::Table, Link::FileId)
            .to(Files::Table, Files::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(Link::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Link::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Link::UserId).uuid().not_null())
                    .col(ColumnDef::new(Link::FileId).uuid().not_null())
                    .col(ColumnDef::new(Link::Signature).string().not_null())
                    .col(ColumnDef::new(Link::Name).string().not_null())
                    .col(ColumnDef::new(Link::Downloads).integer().not_null())
                    .col(ColumnDef::new(Link::Thumbnail).text())
                    .col(ColumnDef::new(Link::EncryptedFileKey).text())
                    .col(ColumnDef::new(Link::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Link::ExpiresAt).timestamp())
                    .foreign_key(&mut foreign_key_user_id)
                    .foreign_key(&mut foreign_key_file_id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Link::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum Link {
    Table,
    Id,
    UserId,
    FileId,
    Signature,
    Name,
    Downloads,
    Thumbnail,
    EncryptedFileKey,
    CreatedAt,
    ExpiresAt,
}
