use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut foreign_key_user_id = ForeignKey::create();
        foreign_key_user_id
            .from(Invitations::Table, Invitations::UserId)
            .to(Users::Table, Users::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(Invitations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Invitations::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Invitations::UserId).uuid())
                    .col(ColumnDef::new(Invitations::Email).string().not_null())
                    .col(
                        ColumnDef::new(Invitations::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Invitations::ExpiresAt)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(&mut foreign_key_user_id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Invitations::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum Invitations {
    Table,
    Id,
    UserId,
    Email,
    CreatedAt,
    ExpiresAt,
}
