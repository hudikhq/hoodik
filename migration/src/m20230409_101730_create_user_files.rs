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
            .from(UserFiles::Table, UserFiles::FileId)
            .to(Files::Table, Files::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);
        let mut foreign_key_user_id = ForeignKey::create();
        foreign_key_user_id
            .from(UserFiles::Table, UserFiles::UserId)
            .to(Users::Table, Users::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(UserFiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserFiles::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserFiles::FileId).integer().not_null())
                    .col(ColumnDef::new(UserFiles::UserId).integer().not_null())
                    .col(ColumnDef::new(UserFiles::EncryptedKey).text().not_null())
                    .col(ColumnDef::new(UserFiles::IsOwner).boolean().not_null())
                    .col(ColumnDef::new(UserFiles::CreatedAt).date_time().not_null())
                    .col(ColumnDef::new(UserFiles::ExpiresAt).date_time())
                    .foreign_key(&mut foreign_key_file_id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserFiles::Table).to_owned())
            .await
    }
}

// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub enum UserFiles {
    Table,
    Id,
    FileId,
    UserId,
    EncryptedKey,
    IsOwner,
    CreatedAt,
    ExpiresAt,
}
