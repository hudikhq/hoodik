use sea_orm_migration::prelude::*;

use crate::m20230409_091730_create_files::Files;

/// Folder-level member-list metadata used by the editable-folder
/// protocol.
///
/// * `last_membership_change_at` — bumped whenever a member is added,
///   removed, or role-changed on the folder. The TOCTOU defense in
///   `upload-multikey` and `move-into-shared` compares the uploader's
///   snapshot against this stamp.
/// * `members_list_signature` — RSA-PSS-SHA256 by the folder owner or
///   any current Co-owner over the canonical member list. Stored verbatim
///   so the per-folder `/members` endpoint can return it without ever
///   re-encoding. NULL until the first client writes it.
/// * `members_list_signed_by_user_id` — actor that produced the
///   signature. Foreign key onto `users`; SET NULL on user delete so the
///   audit trail of the action survives even when the actor's account is
///   gone.
/// * `members_list_signed_at` — timestamp covered by the signature.
///
/// All four fields are NULL for non-folder rows. Owner-row presence of
/// these fields on a non-folder row would be semantically meaningless;
/// the application code keeps writes scoped to `mime = 'dir'` rows.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("last_membership_change_at")).big_integer(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .add_column(ColumnDef::new(Alias::new("members_list_signature")).binary())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .add_column(ColumnDef::new(Alias::new("members_list_signed_at")).big_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Files::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("members_list_signed_by_user_id"))
                            .uuid()
                            .extra(
                                "REFERENCES users(id) ON DELETE SET NULL ON UPDATE NO ACTION"
                                    .to_string(),
                            ),
                    )
                    .to_owned(),
            )
            .await?;

        if manager.get_database_backend() != sea_orm_migration::sea_orm::DatabaseBackend::Sqlite {
            manager
                .create_foreign_key(
                    ForeignKey::create()
                        .name("fk_files_members_list_signed_by_user_id")
                        .from(Files::Table, Alias::new("members_list_signed_by_user_id"))
                        .to(Alias::new("users"), Alias::new("id"))
                        .on_delete(ForeignKeyAction::SetNull)
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
                        .name("fk_files_members_list_signed_by_user_id")
                        .table(Files::Table)
                        .to_owned(),
                )
                .await
                .ok();
        }

        for column in [
            "members_list_signed_by_user_id",
            "members_list_signed_at",
            "members_list_signature",
            "last_membership_change_at",
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Files::Table)
                        .drop_column(Alias::new(column))
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}
