use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;
use crate::m20230409_091730_create_files::Files;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut foreign_key_file_id = ForeignKey::create();
        foreign_key_file_id
            .name("fk_file_versions_file_id")
            .from(FileVersions::Table, FileVersions::FileId)
            .to(Files::Table, Files::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::NoAction);

        let mut foreign_key_user_id = ForeignKey::create();
        foreign_key_user_id
            .name("fk_file_versions_user_id")
            .from(FileVersions::Table, FileVersions::UserId)
            .to(Users::Table, Users::Id)
            .on_delete(ForeignKeyAction::SetNull)
            .on_update(ForeignKeyAction::NoAction);

        manager
            .create_table(
                Table::create()
                    .table(FileVersions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FileVersions::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FileVersions::FileId).uuid().not_null())
                    // Integer version that also names the on-disk directory: v{version}.
                    // (file_id, version) is unique — see index below.
                    .col(ColumnDef::new(FileVersions::Version).integer().not_null())
                    // Saver attribution. NULL when is_anonymous (link-based saves
                    // from A4) or when the user is later deleted (FK SET NULL).
                    .col(ColumnDef::new(FileVersions::UserId).uuid())
                    .col(
                        ColumnDef::new(FileVersions::IsAnonymous)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(FileVersions::Size).big_integer().not_null())
                    .col(ColumnDef::new(FileVersions::Chunks).big_integer().not_null())
                    // Per-version sha256 enables exact restore (overwrites
                    // files.sha256 when the active_version pointer flips back
                    // to this row). Other hash families are deprecated; only
                    // sha256 is computed by current clients.
                    .col(ColumnDef::new(FileVersions::Sha256).string())
                    .col(
                        ColumnDef::new(FileVersions::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(&mut foreign_key_file_id)
                    .foreign_key(&mut foreign_key_user_id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_file_versions_file_version")
                    .table(FileVersions::Table)
                    .col(FileVersions::FileId)
                    .col(FileVersions::Version)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_file_versions_file")
                    .table(FileVersions::Table)
                    .col(FileVersions::FileId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FileVersions::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub(crate) enum FileVersions {
    Table,
    Id,
    FileId,
    Version,
    UserId,
    IsAnonymous,
    Size,
    Chunks,
    Sha256,
    CreatedAt,
}
