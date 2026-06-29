use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;
use crate::m20230409_091730_create_files::Files;

/// Append-only audit log with a per-sender SHA-256 hash chain
/// and per-row sender signature.
///
/// `sender_signature` is RSA-PSS-SHA256 by the actor's privkey over
/// `b"hoodik-audit-sig-v1\0" || encode_audit_event_sig_input_v1`. NULL
/// only on system-cascade rows (Co-owner-revoke fan-out, account-delete
/// cleanup) — the hash chain still covers those rows for deletion
/// detection.
///
/// FK actions: `sender_id` and `recipient_id` SET NULL on user delete so
/// the chain stays intact for surviving participants; `file_id` CASCADE
/// drops audit rows when the file itself is removed (consistent with the
/// rest of `user_files` cascade semantics).
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ShareEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ShareEvents::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ShareEvents::SenderId).uuid())
                    .col(ColumnDef::new(ShareEvents::RecipientId).uuid())
                    .col(ColumnDef::new(ShareEvents::FileId).uuid().not_null())
                    .col(
                        ColumnDef::new(ShareEvents::Action)
                            .string_len(48)
                            .not_null(),
                    )
                    .col(ColumnDef::new(ShareEvents::ShareRoleBefore).string_len(16))
                    .col(ColumnDef::new(ShareEvents::ShareRoleAfter).string_len(16))
                    .col(
                        ColumnDef::new(ShareEvents::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ShareEvents::PrevEventHash).binary())
                    .col(
                        ColumnDef::new(ShareEvents::ThisEventHash)
                            .binary()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ShareEvents::SenderSignature).binary())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_share_events_sender_id")
                            .from(ShareEvents::Table, ShareEvents::SenderId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_share_events_recipient_id")
                            .from(ShareEvents::Table, ShareEvents::RecipientId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_share_events_file_id")
                            .from(ShareEvents::Table, ShareEvents::FileId)
                            .to(Files::Table, Files::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_share_events_sender")
                    .table(ShareEvents::Table)
                    .col(ShareEvents::SenderId)
                    .col(ShareEvents::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_share_events_recipient")
                    .table(ShareEvents::Table)
                    .col(ShareEvents::RecipientId)
                    .col(ShareEvents::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_share_events_file")
                    .table(ShareEvents::Table)
                    .col(ShareEvents::FileId)
                    .col(ShareEvents::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ShareEvents::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub(crate) enum ShareEvents {
    Table,
    Id,
    SenderId,
    RecipientId,
    FileId,
    Action,
    ShareRoleBefore,
    ShareRoleAfter,
    CreatedAt,
    PrevEventHash,
    ThisEventHash,
    SenderSignature,
}
