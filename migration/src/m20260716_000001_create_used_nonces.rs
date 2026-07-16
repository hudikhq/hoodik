use sea_orm_migration::prelude::*;

/// Nonces already spent on signature login. Without this record a captured
/// signature-login body stays valid for the rest of its acceptance window —
/// the signed timestamp's skew allowance, or the minute bucket for legacy
/// clients — and can mint additional sessions. The composite primary key is
/// the replay guard: a second insert of the same pair conflicts and the login
/// is refused.
///
/// No foreign key to `users` — the fingerprint may be a superseded one
/// resolved through `key_transitions`, which no longer matches any
/// `users.fingerprint`.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UsedNonces::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(UsedNonces::Fingerprint).text().not_null())
                    .col(ColumnDef::new(UsedNonces::Nonce).text().not_null())
                    .col(ColumnDef::new(UsedNonces::ExpiresAt).big_integer().not_null())
                    .primary_key(
                        Index::create().col(UsedNonces::Fingerprint).col(UsedNonces::Nonce),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(UsedNonces::Table).to_owned()).await
    }
}

#[derive(Iden)]
pub(crate) enum UsedNonces {
    Table,
    Fingerprint,
    Nonce,
    ExpiresAt,
}
