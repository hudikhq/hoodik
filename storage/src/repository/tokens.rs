//! Repository module for the searchable-token index of a file.
//!
//! Rows hold an HMAC tag of each token rather than its digest, under one of the
//! two keys described on [`file_tokens::Scope`]. The server only ever sees
//! tags, so indexing here is a straight insert and searching is an equality
//! match — no server-side tokenization, and nothing in this file can turn a row
//! back into a word.

use entity::{
    file_tokens::{self, Scope, SearchTags},
    files, links, user_files, ActiveValue, ColumnTrait, Condition, ConnectionTrait, EntityTrait,
    Query, QueryFilter, QueryOrder, QuerySelect, Uuid,
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

    /// Write the tags for one scope of a file.
    pub(crate) async fn upsert(
        &self,
        file_id: Uuid,
        scope: Scope,
        tagged: Vec<String>,
    ) -> AppResult<u64> {
        let rows = cryptfns::search::from_wire(tagged)
            .into_iter()
            .map(|(tag, weight)| file_tokens::ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                file_id: ActiveValue::Set(file_id),
                scope: ActiveValue::Set(scope.into()),
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

    /// Replace the tags of every scope the caller supplied, leaving the others
    /// untouched.
    ///
    /// The asymmetry is deliberate. An editor who is not the owner can rewrite
    /// the file scope, because the file key reaches them, but cannot produce
    /// the owner's root tags. Their save must therefore leave scope 0 alone
    /// rather than clear an index it has no way to rebuild. The owner's own
    /// scope goes stale on that file until they re-index it, which only
    /// matters once the file stops being shared — until then both sides search
    /// it through the file scope.
    pub(crate) async fn reindex(&self, file_id: Uuid, tags: SearchTags) -> AppResult<u64> {
        let mut written = 0;

        for (scope, tagged) in [(Scope::Root, tags.root), (Scope::File, tags.file)] {
            let Some(tagged) = tagged else {
                continue;
            };

            file_tokens::Entity::delete_many()
                .filter(file_tokens::Column::FileId.eq(file_id))
                .filter(file_tokens::Column::Scope.eq(i32::from(scope)))
                .exec(self.repository.connection())
                .await?;

            written += self.upsert(file_id, scope, tagged).await?;
        }

        Ok(written)
    }

    /// Files the caller owns that carry no root-scope tags.
    ///
    /// After the re-key migration that is every file they have, and the set
    /// shrinks as the client works through it — writing the tags is itself the
    /// record of having done so, so a client that closes mid-sweep resumes
    /// where it left off without any progress bookkeeping to keep in sync.
    pub(crate) async fn pending_reindex(&self, limit: u64) -> AppResult<Vec<AppFile>> {
        let results = self
            .repository
            .compact_selector(self.user_id, true)
            .filter(
                files::Column::Id.not_in_subquery(
                    Query::select()
                        .column(file_tokens::Column::FileId)
                        .from(file_tokens::Entity)
                        .and_where(file_tokens::Column::Scope.eq(i32::from(Scope::Root)))
                        .to_owned(),
                ),
            )
            .limit(limit)
            .into_model::<AppFile>()
            .all(self.repository.connection())
            .await?;

        Ok(results)
    }

    /// Search files by tag, ranked by the summed weight of the tags that hit.
    pub(crate) async fn search(&self, search: Search) -> AppResult<Vec<AppFile>> {
        let compact = search.compact.unwrap_or(false);
        let (file_id, hash, root_tags, file_tags, limit, skip, editable) = search.into_tuple();

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

        for (scope, tags) in [(Scope::Root, root_tags), (Scope::File, file_tags)] {
            if tags.is_empty() {
                continue;
            }

            filter = filter.add(
                file_tokens::Column::Scope
                    .eq(i32::from(scope))
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

        let mut results = if root_tags_empty && file_tags_empty {
            // No tags to match. Skipping the query matters: an unfiltered one
            // would join every indexed row the caller can see and return the
            // whole drive.
            vec![]
        } else {
            query
                .into_model::<AppFile>()
                .all(self.repository.connection())
                .await?
        };

        // A query that is itself a content digest also matches the file whose
        // bytes hash to it. Run as its own lookup rather than another branch
        // of the tag filter: the tag query inner-joins the index, so a file
        // with no tags could never come back from it — and whether a file is
        // indexed has nothing to do with whether its bytes match.
        if let Some(hash) = hash {
            let seen: Vec<Uuid> = results.iter().map(|f| f.id).collect();

            for file in self
                .repository
                .query(self.user_id)
                .by_hash(&hash, compact)
                .await?
            {
                if !seen.contains(&file.id) {
                    results.push(file);
                }
            }
        }

        Ok(results)
    }
}
