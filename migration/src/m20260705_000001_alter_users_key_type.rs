use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;

/// Key-algorithm support for the Curve25519 migration. `key_type` records
/// which algorithm the account's keys use (existing rows are `rsa`);
/// `wrapping_pubkey` holds the X25519 public key of `curve25519` accounts,
/// whose identity/signing key lives in `pubkey`. RSA accounts wrap and sign
/// with the same key, so their `wrapping_pubkey` stays NULL.
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
                        ColumnDef::new(Alias::new("key_type"))
                            .string_len(32)
                            .not_null()
                            .default("rsa"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(ColumnDef::new(Alias::new("wrapping_pubkey")).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Alias::new("key_type"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Alias::new("wrapping_pubkey"))
                    .to_owned(),
            )
            .await
    }
}
