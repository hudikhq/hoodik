use sea_orm_migration::prelude::*;

/// Empty the word half of the search index so every client rebuilds it with
/// whole-word tokens.
///
/// The index used to hold wordpiece fragments. Fragments like `01` or `de`
/// occur incidentally in every text-rich note, so a query for a filename
/// ranked the file itself behind dozens of documents that merely contained
/// pieces of it. Tokenization is now whole words, and old fragment rows can
/// never match a whole-word query again — they are dead weight that also
/// keeps the fragment pattern of every name and note readable in tag form.
///
/// Only word-scope rows produced from names and note bodies go. Digest-scope
/// rows are derived from file bytes, not from tokenization, and stay valid.
/// Extra-source rows are written through their own route by clients and would
/// not be recreated by the re-index sweep, so they stay too.
///
/// Blanking `name_hash` is what puts every file back on the login-time
/// re-index sweep — an empty hash is the pending marker, exactly as the keyed
/// re-key migration used it. Until the sweep reaches a file, duplicate-name
/// detection and resume-by-name skip it, which the transition tolerates.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Word scopes are 0 (root) and 1 (file); sources 0 (name) and
        // 1 (content). Digest scopes (2, 3) and the extra source (2) stay.
        manager
            .get_connection()
            .execute_unprepared(
                "DELETE FROM file_tokens WHERE scope IN (0, 1) AND source IN (0, 1)",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared("UPDATE files SET name_hash = '' WHERE name_hash <> ''")
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // The deleted rows only existed client-side as plaintext; there is
        // nothing to restore from here. Rolling back simply leaves the files
        // pending until clients re-index.
        Ok(())
    }
}
