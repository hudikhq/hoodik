use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Link::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Link::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Link::Title).string().not_null())
                    .col(ColumnDef::new(Link::Text).string().not_null())
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
    Title,
    Text,
}
