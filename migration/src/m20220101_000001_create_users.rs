use sea_orm_migration::{prelude::*, sea_orm::prelude::Uuid};

#[derive(DeriveMigrationName)]
pub struct Migration;

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
                    .col(ColumnDef::new(Users::Email).string().not_null())
                    .col(ColumnDef::new(Users::Password).string())
                    .col(ColumnDef::new(Users::Secret).string())
                    .col(ColumnDef::new(Users::Pubkey).string())
                    .col(ColumnDef::new(Users::Fingerprint).string())
                    .col(ColumnDef::new(Users::EncryptedPrivateKey).string())
                    .col(ColumnDef::new(Users::EmailVerifiedAt).timestamp())
                    .col(ColumnDef::new(Users::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Users::UpdatedAt).timestamp().not_null())
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
pub enum Users {
    Table,
    Id,
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
