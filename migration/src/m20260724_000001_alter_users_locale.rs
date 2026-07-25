use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;

/// Preferred language for outbound email, set by the client at registration
/// and editable via `PATCH /api/users/me`. Null means English.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Alias::new("locale")).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Alias::new("locale"))
                    .to_owned(),
            )
            .await
    }
}
