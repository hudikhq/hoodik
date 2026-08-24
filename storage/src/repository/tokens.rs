//! Repository module for the searchable-token index of a file.
//!
//! Rows hold an HMAC tag of each token rather than its digest, under one of the
//! two keys described on [`file_tokens::Scope`]. [`file_tokens::Source`] says
//! which client-side text produced the token; write paths replace only their
//! own source. The server only ever sees tags, so indexing here is a straight
//! insert and searching is an equality match — no server-side tokenization,
//! and nothing in this file can turn a row back into a word.

use entity::{
    file_tokens::{self, DigestTags, Scope, SearchTags, Source},
    files, links, user_files, ActiveValue, ColumnTrait, Condition, ConnectionTrait, EntityTrait,
    Expr, Func, QueryFilter, QueryOrder, QuerySelect, Uuid,
};
use error::AppResult;

use crate::data::{app_file::AppFile, search::Search};

use super::Repository;

pub(crate) struct Tokens<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
    user_id: Uuid,
}

impl<'repository, T> Tokens<'repository, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(repository: &'repository Repository<'repository, T>, user_id: Uuid) -> Self {
        Self {
            repository,
            user_id,
        }
    }

    /// Write the tags for one scope and source of a file.
    pub(crate) async fn upsert(
        &self,
        file_id: Uuid,
        scope: Scope,
        source: Source,
        tagged: Vec<String>,
    ) -> AppResult<u64> {
        let rows = cryptfns::search::from_wire(tagged)
            .into_iter()
            .map(|(tag, weight)| file_tokens::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                file_id: ActiveValue::Set(file_id),
                scope: ActiveValue::Set(scope.into()),
                source: ActiveValue::Set(source.into()),
                tag: ActiveValue::Set(tag),
                weight: ActiveValue::Set(weight),
            })
            .collect::<Vec<_>>();

        if rows.is_empty() {
            return Ok(0);
        }

        let written = rows.len() as u64;
        file_tokens::Entity::insert_many(rows)
            .exec_without_returning(self.repository.connection())
            .await?;

        Ok(written)
    }

    /// Replace the name-source tags of every scope the caller supplied.
    ///
    /// Create, rename, and the re-index sweep write here. Content and extra
    /// sources are left alone, which is what lets extra tags survive a rename.
    pub(crate) async fn reindex(&self, file_id: Uuid, tags: SearchTags) -> AppResult<u64> {
        self.replace_source(file_id, Source::Name, tags).await
    }

    /// Replace word-scope tags for one [`Source`], leaving the other sources
    /// and the digest scopes untouched.
    ///
    /// The asymmetry on [`SearchTags`] is deliberate. An editor who is not the
    /// owner can rewrite the file scope, because the file key reaches them,
    /// but cannot produce the owner's root tags. Their save must therefore
    /// leave scope 0 alone rather than clear an index it has no way to rebuild.
    pub(crate) async fn replace_source(
        &self,
        file_id: Uuid,
        source: Source,
        tags: SearchTags,
    ) -> AppResult<u64> {
        let mut written = 0;

        for (scope, tagged) in [(Scope::Root, tags.root), (Scope::File, tags.file)] {
            let Some(tagged) = tagged else {
                continue;
            };

            file_tokens::Entity::delete_many()
                .filter(file_tokens::Column::FileId.eq(file_id))
                .filter(file_tokens::Column::Source.eq(i32::from(source)))
                .filter(file_tokens::Column::Scope.eq(i32::from(scope)))
                .exec(self.repository.connection())
                .await?;

            written += self.upsert(file_id, scope, source, tagged).await?;
        }

        Ok(written)
    }

    /// Replace the digest tags of every scope the caller supplied.
    ///
    /// Delete-then-insert rather than append: hash writes are retried freely
    /// (the PUT rides a transfer token precisely so a late retry still lands),
    /// and appending on each attempt would duplicate rows and inflate the
    /// weight ranking. Digest scopes are replaced by scope, not source, which
    /// is what lets a rename replace every name token while the file stays
    /// findable by its digest.
    pub(crate) async fn replace_digests(&self, file_id: Uuid, tags: DigestTags) -> AppResult<u64> {
        let mut written = 0;

        for (scope, tagged) in [
            (Scope::DigestRoot, tags.root),
            (Scope::DigestFile, tags.file),
        ] {
            let Some(tagged) = tagged else {
                continue;
            };

            file_tokens::Entity::delete_many()
                .filter(file_tokens::Column::FileId.eq(file_id))
                .filter(file_tokens::Column::Scope.eq(i32::from(scope)))
                .exec(self.repository.connection())
                .await?;

            written += self.upsert(file_id, scope, Source::Name, tagged).await?;
        }

        Ok(written)
    }

    /// Files the caller owns whose `name_hash` is not a keyed tag yet: blank
    /// where the re-key migration purged the old digest, or still the legacy
    /// 64-hex shape on a row that slipped past it.
    ///
    /// After the migration that is every file they have, and the set shrinks
    /// as the client works through it — the keyed `name_hash` every re-index
    /// writes is itself the record of having done so, so a client that closes
    /// mid-sweep resumes where it left off without any progress bookkeeping
    /// to keep in sync. The record deliberately isn't "has root tags": a
    /// name the tokenizer reduces to nothing produces zero tags on a
    /// perfectly successful re-index, and a file like that must leave this
    /// set rather than come back on every fetch forever.
    pub(crate) async fn pending_reindex(&self, limit: u64) -> AppResult<Vec<AppFile>> {
        let results = self
            .repository
            .compact_selector(self.user_id, true)
            .filter(
                Condition::any().add(files::Column::NameHash.eq("")).add(
                    Expr::expr(Func::char_length(Expr::col((
                        files::Entity,
                        files::Column::NameHash,
                    ))))
                    .eq(64),
                ),
            )
            .limit(limit)
            .into_model::<AppFile>()
            .all(self.repository.connection())
            .await?;

        Ok(results)
    }

    /// Search files by tag, ranked by the summed weight of the tags that hit.
    ///
    /// Content-digest lookup is not a separate arm any more: clients index
    /// each file's digests as keyed tags alongside the name and body tokens,
    /// and tag the raw query the same way, so an exact digest match rides the
    /// ordinary tag equality below — always on, and never a plaintext digest
    /// on the wire. Source is not filtered: a query hash matches a file if it
    /// hits name or content or extra.
    pub(crate) async fn search(&self, search: Search) -> AppResult<Vec<AppFile>> {
        let compact = search.compact.unwrap_or(false);
        let (file_id, root_tags, file_tags, limit, skip, editable) = search.into_tuple();

        let user_id = self.user_id;
        let selector = match compact {
            true => self.repository.compact_selector(user_id, false),
            false => self.repository.selector(user_id, false),
        };
        let mut query = selector.inner_join(file_tokens::Entity);

        if let Some(file_id) = file_id {
            query = query.filter(files::Column::FileId.eq(file_id));
        }

        if let Some(editable) = editable {
            query = query.filter(files::Column::Editable.eq(editable));
        }

        let root_tags_empty = root_tags.is_empty();
        let file_tags_empty = file_tags.is_empty();
        let mut filter = Condition::any();

        // Each word scope brings its digest counterpart along: both are keyed
        // on the same key, so the exact-match tag a client appends to its
        // query matches a stored digest row with no extra query field.
        for (scopes, tags) in [
            ([Scope::Root, Scope::DigestRoot], root_tags),
            ([Scope::File, Scope::DigestFile], file_tags),
        ] {
            if tags.is_empty() {
                continue;
            }

            filter = filter.add(
                file_tokens::Column::Scope
                    .is_in(scopes.map(i32::from))
                    .and(file_tokens::Column::Tag.is_in(tags)),
            );
        }

        // Postgres only infers functional dependency from the GROUP BY
        // column to columns of the *same* table. The selector projects
        // columns from `files`, `user_files`, and (left-joined) `links`,
        // so all three primary keys have to appear in the GROUP BY for
        // PG to accept the projection. SQLite is permissive and ignores
        // the extra columns. `links.id` is nullable under the left join,
        // which is fine — NULL forms its own group in PG and rows without
        // a matching link still aggregate correctly.
        let mut query = query
            .filter(filter)
            .group_by(files::Column::Id)
            .group_by(user_files::Column::Id)
            .group_by(links::Column::Id)
            .order_by_desc(file_tokens::Column::Weight.sum());

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(skip) = skip {
            query = query.offset(skip);
        }

        if root_tags_empty && file_tags_empty {
            // No tags to match. Skipping the query matters: an unfiltered one
            // would join every indexed row the caller can see and return the
            // whole drive.
            return Ok(vec![]);
        }

        Ok(query
            .into_model::<AppFile>()
            .all(self.repository.connection())
            .await?)
    }
}
