use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;
use crate::m20230409_101730_create_user_files::UserFiles;

/// Adds three sharing-bookkeeping columns to `user_files`.
///
/// `shared_by_user_id` carries `ON DELETE CASCADE` — when an actor's
/// account is deleted, every row they granted is dropped at the engine
/// level. This is the same cascade that fires when a
/// Co-owner is revoked in-life. The strict semantics avoid orphan grants
/// whose only legitimate signer is no longer trusted; SET NULL would
/// leave those rows behind for the application to clean up.
///
/// `member_signature` carries the folder owner's (or any current
/// Co-owner's) RSA-PSS signature over `MemberSigPayloadV1`. Required on
/// rows that are members of an editable-folder share;
/// NULL on plain shares and on owner rows.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // SQLite supports only one ALTER option per statement, so each
        // ADD COLUMN gets its own alter_table call. PG handles either
        // shape; keeping them split is portable across both backends.
        manager
            .alter_table(
                Table::alter()
                    .table(UserFiles::Table)
                    .add_column(ColumnDef::new(Alias::new("shared_at")).big_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(UserFiles::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("shared_by_user_id"))
                            .uuid()
                            .extra(
                                "REFERENCES users(id) ON DELETE CASCADE ON UPDATE NO ACTION"
                                    .to_string(),
                            ),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(UserFiles::Table)
                    .add_column(ColumnDef::new(Alias::new("member_signature")).binary())
                    .to_owned(),
            )
            .await?;

        // PG only — SQLite encodes the FK inline above via REFERENCES
        // because `create_foreign_key` after the fact is unsupported for
        // SQLite (it cannot ALTER a table to add a constraint).
        if manager.get_database_backend() != sea_orm_migration::sea_orm::DatabaseBackend::Sqlite {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_user_files_shared_by_user_id")
                        .from(UserFiles::Table, Alias::new("shared_by_user_id"))
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::NoAction)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != sea_orm_migration::sea_orm::DatabaseBackend::Sqlite {
            manager
                .drop_foreign_key(
                    ForeignKey::drop()
                        .name("fk_user_files_shared_by_user_id")
                        .table(UserFiles::Table)
                        .to_owned(),
                )
                .await
                .ok();
        }

        for column in ["member_signature", "shared_by_user_id", "shared_at"] {
            manager
                .alter_table(
                    Table::alter()
                        .table(UserFiles::Table)
                        .drop_column(Alias::new(column))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
