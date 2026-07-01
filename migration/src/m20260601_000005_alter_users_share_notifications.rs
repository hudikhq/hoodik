use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;

/// Per-user opt-out for share-grant emails.
/// Default is opt-in; users disable via `PATCH /api/users/me`.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("share_notifications_enabled"))
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Alias::new("share_notifications_enabled"))
                    .to_owned(),
            )
            .await
    }
}
