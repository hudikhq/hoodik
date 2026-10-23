use sea_orm_migration::prelude::*;

use crate::m20230409_101730_create_user_files::UserFiles;

#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserFiles::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("favorite"))
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(UserFiles::Table)
                    .drop_column(Alias::new("favorite"))
                    .to_owned(),
            )
            .await
    }
}
