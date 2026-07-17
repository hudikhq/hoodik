use sea_orm_migration::prelude::*;

use crate::m20260705_000002_create_key_transitions::KeyTransitions;

/// Persist the keys a transition rotated *to*, so every hop self-verifies.
///
/// The transition certificate's signed canonical commits to the new identity
/// and new wrapping public keys, but the original `key_transitions` row stored
/// neither. A row could therefore be verified only while the account's live
/// `users.pubkey`/`users.wrapping_pubkey` were still that row's new keys — i.e.
/// only the most recent transition. In a chain `F0 → F1 → current`, hop
/// `F0 → F1` needs F1's wrapping key, which lives nowhere: it was generated
/// independently at F1's migration and is a hybrid X25519+ML-KEM container, not
/// derivable from the identity key. So the first transition became permanently
/// unverifiable the moment a second rotation occurred. Storing both new keys on
/// the row closes that gap for every future rotation.
///
/// Both columns are `NOT NULL`: a transition always rotates to a concrete
/// identity and wrapping key, and the sole writer (`auth::migration_complete`)
/// always has both. The table is empty in every deployment — the branch is
/// unpushed and that migration, gated on `security_version = 0`, is the only
/// writer — so there are no rows to backfill. The empty-string default is not a
/// meaningful value; it exists only because SQLite's `ALTER TABLE ADD COLUMN`
/// rejects a `NOT NULL` column without one, and no row will ever carry it. Each
/// column is added in its own statement because SQLite adds one per `ALTER`.
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
                        ColumnDef::new(Alias::new("new_identity_key_pem"))
                            .text()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(KeyTransitions::Table)
                    .add_column(
                        ColumnDef::new(Alias::new("new_wrapping_key_pem"))
                            .text()
                            .not_null()
                            .default(""),
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
                    .drop_column(Alias::new("new_identity_key_pem"))
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(KeyTransitions::Table)
                    .drop_column(Alias::new("new_wrapping_key_pem"))
                    .to_owned(),
            )
            .await
    }
}
