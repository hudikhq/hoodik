use sea_orm_migration::prelude::*;

use crate::m20220101_000001_create_users::Users;

/// OPAQUE server state.
///
/// `opaque_config` holds the single server OPRF seed (the [`ServerSetup`]).
/// It is a singleton — `id` is always 1 — generated lazily on first use and
/// never changed, because every registration is bound to it.
///
/// `opaque_login_sessions` holds the per-login server state between the two
/// OPAQUE round trips. Rows are short-lived (a minute) and consumed on finish;
/// a restart in between just makes the user retry the login.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OpaqueConfig::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(OpaqueConfig::Id).integer().not_null().primary_key())
                    .col(ColumnDef::new(OpaqueConfig::ServerSetup).text().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(OpaqueLoginSessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(OpaqueLoginSessions::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(OpaqueLoginSessions::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(OpaqueLoginSessions::ServerLoginState)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(OpaqueLoginSessions::ExpiresAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_opaque_login_sessions_user_id")
                            .from(OpaqueLoginSessions::Table, OpaqueLoginSessions::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OpaqueLoginSessions::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(OpaqueConfig::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub(crate) enum OpaqueConfig {
    Table,
    Id,
    ServerSetup,
}

#[derive(Iden)]
pub(crate) enum OpaqueLoginSessions {
    Table,
    Id,
    UserId,
    ServerLoginState,
    ExpiresAt,
}
