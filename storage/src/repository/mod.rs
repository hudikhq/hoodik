pub(crate) mod cached;
pub(crate) mod manage;
pub(crate) mod query;
pub(crate) mod tokens;
pub(crate) mod versions;

use crate::data::app_file::AppFile;

use self::{manage::Manage, query::Query, tokens::Tokens, versions::Versions};
use entity::{
    files, links, numeric::Numeric, user_files, users, ColumnTrait, ConnectionTrait, EntityTrait,
    Expr, IntoCondition, JoinType, QueryFilter, QuerySelect, RelationTrait, Select, Uuid, Value,
};
use error::{AppResult, Error};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

pub(crate) struct Repository<'ctx, T: ConnectionTrait> {
    connection: &'ctx T,
}

impl<'ctx, T> Repository<'ctx, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(connection: &'ctx T) -> Self {
        Self { connection }
    }
}

impl<T> Repository<'_, T>
where
    T: ConnectionTrait,
{
    /// Query files from any user perspective
    pub(crate) fn query<'repository>(&'repository self, user_id: Uuid) -> Query<'repository, T>
    where
        Self: 'repository,
    {
        Query::<'repository>::new(self, user_id)
    }

    /// Manage files from the owners perspective
    pub(crate) fn manage<'repository>(&'repository self, owner_id: Uuid) -> Manage<'repository, T>
    where
        Self: 'repository,
    {
        Manage::<'repository>::new(self, owner_id)
    }

    /// Manage files from the owners perspective
    pub(crate) fn tokens<'repository>(&'repository self, user_id: Uuid) -> Tokens<'repository, T>
    where
        Self: 'repository,
    {
        Tokens::<'repository>::new(self, user_id)
    }

    /// Versioned-chunks history operations: list/restore/fork/delete.
    pub(crate) fn versions<'repository>(
        &'repository self,
        owner_id: Uuid,
    ) -> Versions<'repository, T>
    where
        Self: 'repository,
    {
        Versions::<'repository>::new(self, owner_id)
    }

    /// Get the inner database connection
    pub(crate) fn connection(&self) -> &impl ConnectionTrait {
        self.connection
    }

    /// Total owner-attributed bytes stored across the whole instance. Mirrors
    /// the per-user [`query::Query::used_space`] aggregate with the per-user
    /// filter dropped, so the instance ceiling counts every owned file exactly
    /// once regardless of who owns it.
    pub(crate) async fn instance_used_space(&self) -> AppResult<i64> {
        let bytes = user_files::Entity::find()
            .select_only()
            .filter(user_files::Column::IsOwner.eq(true))
            .join(JoinType::InnerJoin, user_files::Relation::Files.def())
            .column_as(files::Column::Size.sum(), "sum_of_size")
            .into_tuple::<Option<Numeric>>()
            .one(self.connection)
            .await?;

        Ok(bytes
            .unwrap_or_default()
            .map(|numeric| numeric.into())
            .unwrap_or(0))
    }

    /// Load the file from the database by its id
    pub(crate) async fn by_id<V>(&self, id: V, user_id: Uuid) -> AppResult<AppFile>
    where
        V: Into<Value> + Display + Clone,
    {
        self.selector(user_id, false)
            .filter(files::Column::Id.eq(id.clone()))
            .into_model::<AppFile>()
            .one(self.connection)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("file_not_found:{id}")))
    }

    /// Preset the selector for the given user, maybe check if the user is the owner
    pub(crate) fn selector(&self, user_id: Uuid, check_is_owner: bool) -> Select<files::Entity> {
        self.build_selector(user_id, check_is_owner, false)
    }

    /// Listing variant that leaves `files.encrypted_thumbnail` in the
    /// database and reports only whether one exists. A directory of images
    /// otherwise reads megabytes of base64 off the page and ships it to a
    /// caller that immediately discards it; clients fetch the blob per file
    /// from the thumbnail route instead.
    pub(crate) fn compact_selector(
        &self,
        user_id: Uuid,
        check_is_owner: bool,
    ) -> Select<files::Entity> {
        self.build_selector(user_id, check_is_owner, true)
    }

    fn build_selector(
        &self,
        user_id: Uuid,
        check_is_owner: bool,
        compact: bool,
    ) -> Select<files::Entity> {
        let mut selector = files::Entity::find().select_only();

        match compact {
            true => entity::join::add_columns_with_prefix_nulling::<_, files::Entity>(
                &mut selector,
                "file",
                &["encrypted_thumbnail"],
            ),
            false => {
                entity::join::add_columns_with_prefix::<_, files::Entity>(&mut selector, "file")
            }
        }

        entity::join::add_not_null_flag::<_, files::Entity>(
            &mut selector,
            files::Column::EncryptedThumbnail,
            "has_thumbnail",
        );

        entity::join::add_columns_with_prefix::<_, user_files::Entity>(&mut selector, "user_file");
        entity::join::add_columns_with_prefix::<_, links::Entity>(&mut selector, "link");

        let rel = match check_is_owner {
            true => files::Relation::UserFiles
                .def()
                .on_condition(move |_left, right| {
                    Expr::col((right, user_files::Column::UserId))
                        .eq(user_id)
                        .and(user_files::Column::IsOwner.eq(true))
                        .into_condition()
                }),
            false => files::Relation::UserFiles
                .def()
                .on_condition(move |_left, right| {
                    Expr::col((right, user_files::Column::UserId))
                        .eq(user_id)
                        .into_condition()
                }),
        };

        selector.join(JoinType::InnerJoin, rel).join(
            JoinType::LeftJoin,
            files::Relation::Links
                .def()
                .on_condition(move |_left, right| {
                    Expr::col((right, links::Column::UserId))
                        .eq(user_id)
                        .into_condition()
                }),
        )
    }

    /// Stamp `owner_email` onto each non-owner row in `files`. Owner-of-file
    /// rows skip the lookup — the caller already knows their own address,
    /// and surfacing it in the UI would clutter their own root. Runs at
    /// most two queries regardless of `files.len()`.
    pub(crate) async fn enrich_owner_emails(&self, files: &mut [AppFile]) -> AppResult<()> {
        let target_file_ids: Vec<Uuid> = files
            .iter()
            .filter(|f| !f.is_owner)
            .map(|f| f.id)
            .collect();
        if target_file_ids.is_empty() {
            return Ok(());
        }

        let owner_rows = user_files::Entity::find()
            .filter(user_files::Column::FileId.is_in(target_file_ids))
            .filter(user_files::Column::IsOwner.eq(true))
            .all(self.connection)
            .await?;
        let owner_user_by_file: HashMap<Uuid, Uuid> =
            owner_rows.iter().map(|r| (r.file_id, r.user_id)).collect();

        let owner_user_ids: HashSet<Uuid> = owner_user_by_file.values().copied().collect();
        let emails_by_user: HashMap<Uuid, String> = users::Entity::find()
            .filter(users::Column::Id.is_in(owner_user_ids.into_iter().collect::<Vec<_>>()))
            .all(self.connection)
            .await?
            .into_iter()
            .map(|u| (u.id, u.email))
            .collect();

        for file in files.iter_mut() {
            if file.is_owner {
                continue;
            }
            if let Some(user_id) = owner_user_by_file.get(&file.id) {
                file.owner_email = emails_by_user.get(user_id).cloned();
            }
        }
        Ok(())
    }

    /// Stamp `shared_with_count` onto each owned row. The count is the
    /// number of non-owner `user_files` rows for the file — i.e. how
    /// many other accounts the caller has shared it with. Used by the
    /// file browser to render an inline "shared with others" hint. Runs
    /// a single grouped count regardless of `files.len()`.
    pub(crate) async fn enrich_shared_with_counts(&self, files: &mut [AppFile]) -> AppResult<()> {
        let target_file_ids: Vec<Uuid> = files.iter().filter(|f| f.is_owner).map(|f| f.id).collect();
        if target_file_ids.is_empty() {
            return Ok(());
        }

        let recipient_rows = user_files::Entity::find()
            .filter(user_files::Column::FileId.is_in(target_file_ids))
            .filter(user_files::Column::IsOwner.eq(false))
            .all(self.connection)
            .await?;

        let mut counts: HashMap<Uuid, i64> = HashMap::new();
        for row in recipient_rows {
            *counts.entry(row.file_id).or_insert(0) += 1;
        }

        for file in files.iter_mut() {
            if !file.is_owner {
                continue;
            }
            file.shared_with_count = counts.get(&file.id).copied().unwrap_or(0);
        }
        Ok(())
    }
}
