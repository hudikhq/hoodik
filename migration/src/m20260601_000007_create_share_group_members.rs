use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;
use crate::m20260601_000006_create_share_groups::ShareGroups;

/// Composite-PK group membership. Both FKs cascade so removing a user
/// or a group cleans up membership rows in one engine pass.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShareGroupMembers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ShareGroupMembers::GroupId).uuid().not_null())
                    .col(ColumnDef::new(ShareGroupMembers::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(ShareGroupMembers::AddedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .primary_key(
                        Index::create()
                            .col(ShareGroupMembers::GroupId)
                            .col(ShareGroupMembers::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_share_group_members_group_id")
                            .from(ShareGroupMembers::Table, ShareGroupMembers::GroupId)
                            .to(ShareGroups::Table, ShareGroups::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_share_group_members_user_id")
                            .from(ShareGroupMembers::Table, ShareGroupMembers::UserId)
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
                    .name("idx_share_group_members_user")
                    .table(ShareGroupMembers::Table)
                    .col(ShareGroupMembers::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ShareGroupMembers::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub(crate) enum ShareGroupMembers {
    Table,
    GroupId,
    UserId,
    AddedAt,
    Role,
}
