use sea_orm_migration::prelude::*;

use crate::m20260601_000007_create_share_group_members::ShareGroupMembers;

/// Adds the group-membership role to `share_group_members`.
///
/// This is the *group* role (`reader` | `editor` | `co-owner`) — what a
/// member may do to the group itself. It is a distinct axis from the
/// *file* `user_files.share_role`, which governs access to a shared file.
/// The two enums share the same three words; they never share a column.
///
/// `reader` is the safe default: a row that predates the column gets the
/// least-privileged tier (view-only). The owner never has a member row,
/// so the owner's implicit `co-owner+` standing is resolved in code, not
/// stored here.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ShareGroupMembers::Table)
                    .add_column(
                        ColumnDef::new(ShareGroupMembers::Role)
                            .string_len(16)
                            .not_null()
                            .default("reader")
                            .check(
                                Expr::col(ShareGroupMembers::Role)
                                    .is_in(["reader", "editor", "co-owner"]),
                            ),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ShareGroupMembers::Table)
                    .drop_column(ShareGroupMembers::Role)
                    .to_owned(),
            )
            .await
    }
}
