use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;
use crate::m20230409_091730_create_files::Files;
use crate::m20260601_000004_create_share_events::ShareEvents;

/// A `key_rotation` audit event belongs to no file, so `share_events.file_id`
/// must accept NULL. File-scoped events still set it and still cascade on file
/// delete; only account-level events leave it empty.
///
/// Postgres relaxes the column in place. SQLite cannot alter a column, so the
/// table is rebuilt: the standard create-copy-drop-rename dance, expressed
/// entirely through the query builder so both dialects stay parametric.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

const REBUILD_ALIAS: &str = "share_events_rebuild";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return manager
                .alter_table(
                    Table::alter()
                        .table(ShareEvents::Table)
                        .modify_column(ColumnDef::new(ShareEvents::FileId).uuid().null())
                        .to_owned(),
                )
                .await;
        }

        rebuild_sqlite(manager, false).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            return manager
                .alter_table(
                    Table::alter()
                        .table(ShareEvents::Table)
                        .modify_column(ColumnDef::new(ShareEvents::FileId).uuid().not_null())
                        .to_owned(),
                )
                .await;
        }

        rebuild_sqlite(manager, true).await
    }
}

/// Columns in `share_events`, in declaration order. The copy lists them
/// explicitly on both sides so an accidental reorder can't silently shift data.
const COLUMNS: [ShareEvents; 11] = [
    ShareEvents::Id,
    ShareEvents::SenderId,
    ShareEvents::RecipientId,
    ShareEvents::FileId,
    ShareEvents::Action,
    ShareEvents::ShareRoleBefore,
    ShareEvents::ShareRoleAfter,
    ShareEvents::CreatedAt,
    ShareEvents::PrevEventHash,
    ShareEvents::ThisEventHash,
    ShareEvents::SenderSignature,
];

/// Rebuild `share_events` with `file_id` either nullable (`up`) or not-null
/// (`down`). Indexes are recreated only after the old table is dropped, because
/// SQLite index names live in one schema-wide namespace and would otherwise
/// collide.
async fn rebuild_sqlite(manager: &SchemaManager<'_>, file_id_not_null: bool) -> Result<(), DbErr> {
    let rebuild = Alias::new(REBUILD_ALIAS);

    let mut file_id = ColumnDef::new(ShareEvents::FileId).uuid().to_owned();
    if file_id_not_null {
        file_id.not_null();
    }

    manager
        .create_table(
            Table::create()
                .table(rebuild.clone())
                .col(ColumnDef::new(ShareEvents::Id).uuid().not_null().primary_key())
                .col(ColumnDef::new(ShareEvents::SenderId).uuid())
                .col(ColumnDef::new(ShareEvents::RecipientId).uuid())
                .col(&mut file_id)
                .col(ColumnDef::new(ShareEvents::Action).string_len(48).not_null())
                .col(ColumnDef::new(ShareEvents::ShareRoleBefore).string_len(16))
                .col(ColumnDef::new(ShareEvents::ShareRoleAfter).string_len(16))
                .col(ColumnDef::new(ShareEvents::CreatedAt).big_integer().not_null())
                .col(ColumnDef::new(ShareEvents::PrevEventHash).binary())
                .col(ColumnDef::new(ShareEvents::ThisEventHash).binary().not_null())
                .col(ColumnDef::new(ShareEvents::SenderSignature).binary())
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_share_events_sender_id")
                        .from(rebuild.clone(), ShareEvents::SenderId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::SetNull)
                        .on_update(ForeignKeyAction::NoAction),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_share_events_recipient_id")
                        .from(rebuild.clone(), ShareEvents::RecipientId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::SetNull)
                        .on_update(ForeignKeyAction::NoAction),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_share_events_file_id")
                        .from(rebuild.clone(), ShareEvents::FileId)
                        .to(Files::Table, Files::Id)
                        .on_delete(ForeignKeyAction::Cascade)
                        .on_update(ForeignKeyAction::NoAction),
                )
                .to_owned(),
        )
        .await?;

    let copy = Query::insert()
        .into_table(rebuild.clone())
        .columns(COLUMNS)
        .select_from(
            Query::select()
                .columns(COLUMNS)
                .from(ShareEvents::Table)
                .to_owned(),
        )
        .map_err(|e| DbErr::Migration(e.to_string()))?
        .to_owned();
    let backend = manager.get_database_backend();
    manager.get_connection().execute(backend.build(&copy)).await?;

    manager
        .drop_table(Table::drop().table(ShareEvents::Table).to_owned())
        .await?;
    manager
        .get_connection()
        .execute(backend.build(
            Table::rename().table(rebuild, ShareEvents::Table),
        ))
        .await?;

    for (name, extra) in [
        ("idx_share_events_sender", ShareEvents::SenderId),
        ("idx_share_events_recipient", ShareEvents::RecipientId),
        ("idx_share_events_file", ShareEvents::FileId),
    ] {
        manager
            .create_index(
                Index::create()
                    .name(name)
                    .table(ShareEvents::Table)
                    .col(extra)
                    .col(ShareEvents::CreatedAt)
                    .to_owned(),
            )
            .await?;
    }

    Ok(())
}
