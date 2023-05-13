use super::m20220101_000001_create_users::Users;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut foreign_key = ForeignKey::create();
        foreign_key
            .from(Sessions::Table, Sessions::UserId)
            .to(Users::Table, Users::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(Sessions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Sessions::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Sessions::UserId).uuid().not_null())
                    .col(ColumnDef::new(Sessions::DeviceId).uuid().not_null())
                    .col(ColumnDef::new(Sessions::Refresh).uuid())
                    .col(ColumnDef::new(Sessions::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Sessions::UpdatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Sessions::ExpiresAt).timestamp().not_null())
                    .col(ColumnDef::new(Sessions::DeletedAt).timestamp())
                    .foreign_key(&mut foreign_key)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Sessions::Table).to_owned())
            .await
    }
}

// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub enum Sessions {
    Table,
    Id,
    UserId,
    DeviceId,
    Refresh,
    CreatedAt,
    UpdatedAt,
    ExpiresAt,
    DeletedAt,
}
