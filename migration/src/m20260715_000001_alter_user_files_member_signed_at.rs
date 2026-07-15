use sea_orm_migration::prelude::*;

/// Store the timestamp a member signature covered, separately from `shared_at`.
///
/// The member σ signs over `MemberSigPayloadV1.signed_at`, and the client
/// re-verifies against the value the server returns as `added_at`. That value
/// used to come from `shared_at`, which also orders the recipient's shares
/// list — so persisting the signed timestamp there coupled the list order to
/// client-supplied timestamps. This column holds the signed value; `shared_at`
/// stays the server-side share time used for ordering and display.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("user_files"))
                    .add_column(
                        ColumnDef::new(Alias::new("member_signed_at"))
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("user_files"))
                    .drop_column(Alias::new("member_signed_at"))
                    .to_owned(),
            )
            .await
    }
}
