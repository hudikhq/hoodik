use sea_orm_migration::prelude::*;

/// Replace the search index with one whose entries are keyed.
///
/// The old index stored an unsalted SHA-256 of every BERT token. That
/// vocabulary is public, pre-trained and roughly thirty thousand entries, so a
/// rainbow table reverses the whole table in seconds — recovering file names
/// and, because note bodies are indexed word for word, note contents. Nothing
/// here can be re-keyed in place: the server holds no plaintext, so every
/// client rebuilds its own index against the new scheme.
///
/// The old rows therefore go now rather than after clients finish rebuilding.
/// They are the thing being removed, and leaving them in place for a
/// comfortable transition would keep the readable copy alive for exactly as
/// long as the slowest client takes — which for anyone who never updates is
/// indefinitely.
///
/// `tokens` disappears with them. It existed to deduplicate digests across
/// every account on the instance, which is cross-tenant linkage that an
/// end-to-end encrypted index should not have been doing. Tags now live inline
/// on `file_tokens`, which also drops a join from the search query.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FileTokens::Table).if_exists().to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tokens::Table).if_exists().to_owned())
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(FileTokens::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(FileTokens::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(FileTokens::FileId).uuid().not_null())
                    // 0 tags under the owner's root key and answers their own
                    // search in one tag per word. 1 tags under a key derived
                    // from the file's own key, which every share recipient
                    // already holds, so a share grant writes nothing here.
                    .col(ColumnDef::new(FileTokens::Scope).integer().not_null())
                    .col(ColumnDef::new(FileTokens::Tag).string().not_null())
                    .col(ColumnDef::new(FileTokens::Weight).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_file_tokens_file_id")
                            .from(FileTokens::Table, FileTokens::FileId)
                            .to(Files::Table, Files::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await?;

        // The search predicate matches on tag alone and lets the `user_files`
        // join do the access filtering, so this is the index the query rides.
        manager
            .create_index(
                Index::create()
                    .name("idx_file_tokens_tag")
                    .table(FileTokens::Table)
                    .col(FileTokens::Tag)
                    .to_owned(),
            )
            .await?;

        // Re-indexing a file replaces one scope at a time: an editor who is
        // not the owner can rewrite scope 1 and must leave scope 0 alone.
        manager
            .create_index(
                Index::create()
                    .name("idx_file_tokens_file_id_scope")
                    .table(FileTokens::Table)
                    .col(FileTokens::FileId)
                    .col(FileTokens::Scope)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FileTokens::Table).if_exists().to_owned())
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Tokens::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Tokens::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Tokens::Hash).string().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(FileTokens::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(FileTokens::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(FileTokens::FileId).uuid().not_null())
                    .col(ColumnDef::new(FileTokens::TokenId).uuid().not_null())
                    .col(ColumnDef::new(FileTokens::Weight).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_file_tokens_file_id")
                            .from(FileTokens::Table, FileTokens::FileId)
                            .to(Files::Table, Files::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_file_tokens_token_id")
                            .from(FileTokens::Table, FileTokens::TokenId)
                            .to(Tokens::Table, Tokens::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::NoAction),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum FileTokens {
    Table,
    Id,
    FileId,
    Scope,
    Tag,
    Weight,
    TokenId,
}

#[derive(Iden)]
enum Tokens {
    Table,
    Id,
    Hash,
}

#[derive(Iden)]
enum Files {
    Table,
    Id,
}
