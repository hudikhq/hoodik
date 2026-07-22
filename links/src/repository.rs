use std::str::FromStr;

use context::Context;
use entity::{
    files,
    links::{self},
    user_files, users, ColumnTrait, ConnectionTrait, EntityTrait, Expr, IntoCondition, JoinType,
    QueryFilter, QuerySelect, RelationTrait, Statement, Uuid,
};
use error::{AppResult, Error};

use crate::data::{app_link::AppLink, create_link::CreateLink};

pub(crate) struct Repository<'ctx> {
    context: &'ctx Context,
}

impl<'ctx> Repository<'ctx> {
    pub(crate) fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    /// Create a shareable link for a file.
    /// Before creating:
    /// - verify the passed signature is valid.
    /// - verify the user is the owner of the file.
    pub(crate) async fn create(
        &self,
        create_link: CreateLink,
        user: &entity::users::Model,
    ) -> AppResult<AppLink> {
        let (data, signature, file_id) = create_link.into_active_model(user.id)?;

        cryptfns::identity::KeyType::from_str(&user.key_type)?.verify(
            file_id.to_string().as_str(),
            &signature,
            &user.pubkey,
        )?;

        let (_file, user_file) = self.get_file_with_owner(file_id).await?;

        if user_file.user_id != user.id {
            return Err(Error::Forbidden("cannot_share_not_owner".to_string()));
        }

        let id = entity::active_value_to_uuid(data.id.clone()).ok_or(Error::as_wrong_id("link"))?;

        links::Entity::insert(data)
            .exec_without_returning(&self.context.db)
            .await?;

        self.get_by_id(id).await
    }

    /// Get a link by id and verify it is not expired.
    pub(crate) async fn get(&self, id: Uuid) -> AppResult<AppLink> {
        let app_link = self.get_by_id(id).await?;

        Ok(app_link)
    }

    /// Delete a link by id.
    /// This will not delete the file.
    pub(crate) async fn delete(&self, id: Uuid, user_id: Uuid) -> AppResult<()> {
        let link = self.get_by_id(id).await?;

        if link.owner_id != user_id {
            return Err(Error::Forbidden("cannot_delete_not_owner".to_string()));
        }

        links::Entity::delete_by_id(id)
            .exec(&self.context.db)
            .await?;

        Ok(())
    }

    /// Update the expires_at field for a link.
    /// If the expires at is set to before now, the link will be purged
    /// from the database when the cron service runs next time.
    pub(crate) async fn update_expires_at(
        &self,
        id: Uuid,
        user_id: Uuid,
        expires_at: Option<i64>,
    ) -> AppResult<AppLink> {
        let link = links::Entity::find_by_id(id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::as_not_found("link"))?;

        if link.user_id != user_id {
            return Err(Error::Forbidden("cannot_update_not_owner".to_string()));
        }

        let link = links::ActiveModel {
            expires_at: entity::ActiveValue::Set(expires_at),
            ..link.into()
        };

        links::Entity::update(link).exec(&self.context.db).await?;

        self.get_by_id(id).await
    }

    /// Increment file downloads counter.
    pub(crate) async fn increment_downloads(&self, id: Uuid) -> AppResult<()> {
        self.context
            .db
            .execute(Statement::from_sql_and_values(
                self.context.db.get_database_backend(),
                r"UPDATE links
                    SET downloads = downloads + 1
                    WHERE id = $1;",
                [id.into()],
            ))
            .await?;
        Ok(())
    }

    /// Get all the links for a user.
    /// This will not include expired links.
    ///
    /// `compact` leaves `links.encrypted_thumbnail` in the database and
    /// reports only whether one exists — a page of image links otherwise
    /// reads megabytes of base64 the caller discards; clients fetch the
    /// blob per link from the metadata route instead.
    pub(crate) async fn links(
        &self,
        user_id: Uuid,
        with_expired: bool,
        compact: bool,
    ) -> AppResult<Vec<AppLink>> {
        let mut selector = links::Entity::find().select_only();

        match compact {
            true => entity::join::add_columns_with_prefix_nulling::<_, links::Entity>(
                &mut selector,
                "link",
                &["encrypted_thumbnail"],
            ),
            false => {
                entity::join::add_columns_with_prefix::<_, links::Entity>(&mut selector, "link")
            }
        }

        entity::join::add_not_null_flag::<_, links::Entity>(
            &mut selector,
            links::Column::EncryptedThumbnail,
            "has_thumbnail",
        );

        entity::join::add_columns_with_prefix::<_, users::Entity>(&mut selector, "user");
        entity::join::add_columns_with_prefix::<_, files::Entity>(&mut selector, "file");

        if !with_expired {
            // `expires_at` is a `bigint` unix timestamp, not a datetime
            // column. Postgres rejects bigint > timestamp comparisons;
            // SQLite is lax. Compare against a unix timestamp on both.
            selector = selector.filter(
                links::Column::ExpiresAt
                    .is_null()
                    .or(links::Column::ExpiresAt.gt(chrono::Utc::now().timestamp())),
            );
        }

        let links = selector
            .filter(links::Column::UserId.eq(user_id))
            .join(JoinType::InnerJoin, links::Relation::Users.def())
            .join(JoinType::InnerJoin, links::Relation::Files.def())
            .into_model::<AppLink>()
            .all(&self.context.db)
            .await?;

        Ok(links)
    }

    /// Load the link, file and user from the database and pack it into `AppLink`.
    async fn get_by_id(&self, id: Uuid) -> AppResult<AppLink> {
        let mut selector = links::Entity::find().select_only();

        entity::join::add_columns_with_prefix::<_, links::Entity>(&mut selector, "link");
        entity::join::add_columns_with_prefix::<_, users::Entity>(&mut selector, "user");
        entity::join::add_columns_with_prefix::<_, files::Entity>(&mut selector, "file");

        let app_link = selector
            .filter(links::Column::Id.eq(id))
            .join(JoinType::InnerJoin, links::Relation::Users.def())
            .join(JoinType::InnerJoin, links::Relation::Files.def())
            .into_model::<AppLink>()
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::as_not_found("link"))?;

        Ok(app_link)
    }

    /// Before we create a shared link, we will first load the relation from the file
    /// to verify that the user trying to share the file is the actual owner of the file.
    async fn get_file_with_owner(&self, id: Uuid) -> AppResult<(files::Model, user_files::Model)> {
        let (file, user_file) = files::Entity::find()
            .filter(files::Column::Id.eq(id))
            .join(
                JoinType::InnerJoin,
                files::Relation::UserFiles
                    .def()
                    .on_condition(move |_left, right| {
                        Expr::col((right, user_files::Column::IsOwner))
                            .eq(true)
                            .into_condition()
                    }),
            )
            .select_also(user_files::Entity)
            .one(&self.context.db)
            .await
            .map_err(Error::from)?
            .ok_or_else(|| Error::NotFound(format!("file_not_found:{id}")))?;

        Ok((file, user_file.unwrap()))
    }
}
