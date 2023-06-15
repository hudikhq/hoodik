use sea_orm_migration::{prelude::*, sea_orm::prelude::Uuid};

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .uuid()
                            .default(Uuid::new_v4().to_string())
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Users::Role).string())
                    .col(ColumnDef::new(Users::Email).string().not_null())
                    .col(ColumnDef::new(Users::Password).string())
                    .col(ColumnDef::new(Users::Secret).string())
                    .col(ColumnDef::new(Users::Pubkey).string())
                    .col(ColumnDef::new(Users::Fingerprint).string())
                    .col(ColumnDef::new(Users::EncryptedPrivateKey).string())
                    .col(ColumnDef::new(Users::EmailVerifiedAt).big_integer())
                    .col(ColumnDef::new(Users::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Users::UpdatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum Users {
    Table,
    Id,
    Role,
    Email,
    Password,
    Secret,
    Pubkey,
    Fingerprint,
    EncryptedPrivateKey,
    EmailVerifiedAt,
    CreatedAt,
    UpdatedAt,
}
