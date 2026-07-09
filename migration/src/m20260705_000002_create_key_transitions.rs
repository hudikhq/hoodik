use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;

/// Migration foundation for the OPAQUE + Curve25519 upgrade.
///
/// `users.security_version` marks how an account authenticates and wraps its
/// key: `0` is the legacy bcrypt + weak-KDF world (every existing row), `1` is
/// migrated (OPAQUE password file + envelope-wrapped keys). The flip happens
/// in one transaction on first login after the update.
///
/// `users.opaque_password_file` holds the per-user OPAQUE registration record,
/// NULL until migration. It reveals nothing about the password.
///
/// `key_transitions` is the append-only chain of re-key endorsements. Each row
/// stores the certificate components (never the wire DER, so verifiers
/// re-encode the canonical) plus both signatures. `old_fingerprint` is unique
/// — a key can be superseded exactly once — and is the lookup used to resolve
/// an old fingerprint to the current identity for share acceptance, signature
/// login, and TOFU continuity.
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
                        ColumnDef::new(Alias::new("security_version"))
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Alias::new("opaque_password_file")).text().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(KeyTransitions::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(KeyTransitions::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(KeyTransitions::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(KeyTransitions::OldFingerprint)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(KeyTransitions::OldKeySpki).binary().not_null())
                    .col(ColumnDef::new(KeyTransitions::NewFingerprint).string_len(64).not_null())
                    .col(ColumnDef::new(KeyTransitions::OldSignature).binary().not_null())
                    .col(ColumnDef::new(KeyTransitions::NewSignature).binary().not_null())
                    .col(ColumnDef::new(KeyTransitions::IssuedAt).big_integer().not_null())
                    .col(ColumnDef::new(KeyTransitions::CreatedAt).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_key_transitions_user_id")
                            .from(KeyTransitions::Table, KeyTransitions::UserId)
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
                    .name("idx_key_transitions_user")
                    .table(KeyTransitions::Table)
                    .col(KeyTransitions::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(KeyTransitions::Table).to_owned())
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Alias::new("opaque_password_file"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Alias::new("security_version"))
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
pub(crate) enum KeyTransitions {
    Table,
    Id,
    UserId,
    OldFingerprint,
    OldKeySpki,
    NewFingerprint,
    OldSignature,
    NewSignature,
    IssuedAt,
    CreatedAt,
}
