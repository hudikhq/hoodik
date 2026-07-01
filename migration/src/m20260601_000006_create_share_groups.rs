use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;

/// Owner-defined named recipient bags. The
/// `(owner_id, name)` uniqueness lets the UI safely auto-complete by
/// name without per-user prefix.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShareGroups::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ShareGroups::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ShareGroups::OwnerId).uuid().not_null())
                    .col(ColumnDef::new(ShareGroups::Name).string().not_null())
                    .col(
                        ColumnDef::new(ShareGroups::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_share_groups_owner_id")
                            .from(ShareGroups::Table, ShareGroups::OwnerId)
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
                    .name("idx_share_groups_owner")
                    .table(ShareGroups::Table)
                    .col(ShareGroups::OwnerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uniq_share_groups_owner_name")
                    .table(ShareGroups::Table)
                    .col(ShareGroups::OwnerId)
                    .col(ShareGroups::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ShareGroups::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub(crate) enum ShareGroups {
    Table,
    Id,
    OwnerId,
    Name,
    CreatedAt,
}
