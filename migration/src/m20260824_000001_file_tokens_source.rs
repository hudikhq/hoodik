use sea_orm_migration::prelude::*;

/// Add `file_tokens.source` so name, note-body, and extra tags replace
/// independently.
///
/// Existing rows become `name`. Word-scope rows were never distinguished by
/// origin, so that is the only default that keeps filename search working
/// without pretending a note-body token is something else. Digest-scope rows
/// get the same default; they are still replaced by scope, not by source.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(FileTokens::Table)
                    .add_column(
                        ColumnDef::new(FileTokens::Source)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_file_tokens_file_id_source_scope")
                    .table(FileTokens::Table)
                    .col(FileTokens::FileId)
                    .col(FileTokens::Source)
                    .col(FileTokens::Scope)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_file_tokens_file_id_source_scope")
                    .table(FileTokens::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(FileTokens::Table)
                    .drop_column(FileTokens::Source)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum FileTokens {
    Table,
    FileId,
    Scope,
    Source,
}
