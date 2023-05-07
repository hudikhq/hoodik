use sea_orm_migration::{prelude::*, sea_orm::prelude::Uuid};

use crate::m20220101_000001_create_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut foreign_key = ForeignKey::create();
        foreign_key
            .from(UserActions::Table, UserActions::UserId)
            .to(Users::Table, Users::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(UserActions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserActions::Id)
                            .uuid()
                            .default(Uuid::new_v4().to_string())
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserActions::UserId).uuid().not_null())
                    .col(ColumnDef::new(UserActions::Email).string().not_null())
                    .col(ColumnDef::new(UserActions::Action).string().not_null())
                    .col(
                        ColumnDef::new(UserActions::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(&mut foreign_key)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserActions::Table).to_owned())
            .await
    }
}

// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub enum UserActions {
    Table,
    Id,
    UserId,
    Email,
    Action,
    CreatedAt,
}
