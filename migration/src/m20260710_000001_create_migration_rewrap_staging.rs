use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;

/// Staging area for the re-wrapped keys a migrating account submits in batches,
/// applied atomically by `migration/complete`. See the entity doc for the
/// column semantics. The two unique indexes make a re-submitted batch replace
/// its rows rather than duplicate them; nulls in a composite unique index do not
/// collide, so file rows (null `link_id`) and link rows (null `file_id`)
/// coexist without conflicting on the other target's index.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MigrationRewrapStaging::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MigrationRewrapStaging::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MigrationRewrapStaging::UserId).uuid().not_null())
                    .col(ColumnDef::new(MigrationRewrapStaging::FileId).uuid().null())
                    .col(ColumnDef::new(MigrationRewrapStaging::LinkId).uuid().null())
                    .col(
                        ColumnDef::new(MigrationRewrapStaging::EncryptedKey)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(MigrationRewrapStaging::Signature).text().null())
                    .col(
                        ColumnDef::new(MigrationRewrapStaging::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_migration_rewrap_staging_user_id")
                            .from(MigrationRewrapStaging::Table, MigrationRewrapStaging::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_migration_rewrap_staging_user_file")
                    .table(MigrationRewrapStaging::Table)
                    .col(MigrationRewrapStaging::UserId)
                    .col(MigrationRewrapStaging::FileId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_migration_rewrap_staging_user_link")
                    .table(MigrationRewrapStaging::Table)
                    .col(MigrationRewrapStaging::UserId)
                    .col(MigrationRewrapStaging::LinkId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MigrationRewrapStaging::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub(crate) enum MigrationRewrapStaging {
    Table,
    Id,
    UserId,
    FileId,
    LinkId,
    EncryptedKey,
    Signature,
    CreatedAt,
}
