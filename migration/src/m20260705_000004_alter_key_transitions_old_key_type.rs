use sea_orm_migration::prelude::*;

use crate::m20260705_000002_create_key_transitions::KeyTransitions;

/// Record the algorithm of the superseded key on each transition row.
///
/// Reconstructing the old public key from `old_key_spki` to verify a
/// pre-rotation signature needs to know its type: PKCS#1 for RSA, SPKI for
/// Curve25519. Every existing transition is an RSA→Curve25519 migration, so
/// the backfill default is `rsa`; a future curve→curve rotation writes
/// `curve25519`.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(KeyTransitions::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("old_key_type"))
                            .string_len(32)
                            .not_null()
                            .default("rsa"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(KeyTransitions::Table)
                    .drop_column(Alias::new("old_key_type"))
                    .to_owned(),
            )
            .await
    }
}
