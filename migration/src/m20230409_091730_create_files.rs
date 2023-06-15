use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut foreign_key_file_id = ForeignKey::create();
        foreign_key_file_id
            .from(Files::Table, Files::FileId)
            .to(Files::Table, Files::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(Files::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Files::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Files::NameHash).string().not_null())
                    .col(ColumnDef::new(Files::EncryptedName).string().not_null())
                    .col(ColumnDef::new(Files::EncryptedThumbnail).string())
                    .col(ColumnDef::new(Files::Mime).string().not_null())
                    .col(ColumnDef::new(Files::Size).big_integer())
                    .col(ColumnDef::new(Files::Chunks).big_integer())
                    .col(ColumnDef::new(Files::ChunksStored).integer())
                    .col(ColumnDef::new(Files::FileId).uuid())
                    .col(
                        ColumnDef::new(Files::FileCreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Files::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(Files::FinishedUploadAt).big_integer())
                    .foreign_key(&mut foreign_key_file_id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Files::Table).to_owned())
            .await
    }
}

// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
pub(crate) enum Files {
    Table,
    Id,
    EncryptedName,
    EncryptedThumbnail,
    NameHash,
    Mime,
    Size,
    Chunks,
    ChunksStored,
    FileId,
    FileCreatedAt,
    CreatedAt,
    FinishedUploadAt,
}
