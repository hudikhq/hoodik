use crate::data::sessions::{search::Search, session::Session};

use super::Repository;
use chrono::Utc;
use entity::{
    sessions::{self, ActiveModel},
    sort::Sortable,
    users, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter,
    QuerySelect, RelationTrait, Uuid,
};
use error::AppResult;
use validr::Validation;

pub(crate) struct SessionsRepository<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
}

impl<'repository, T> SessionsRepository<'repository, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(repository: &'repository Repository<'repository, T>) -> Self {
        Self { repository }
    }

    /// Find all the sessions
    pub(crate) async fn find(&self, sessions: Search) -> AppResult<Vec<Session>> {
        let sessions = sessions.validate()?;

        let mut query = sessions::Entity::find().select_only();

        entity::join::add_columns_with_prefix::<_, sessions::Entity>(&mut query, "session");
        entity::join::add_columns_with_prefix::<_, users::Entity>(&mut query, "user");

        query = query.join(JoinType::InnerJoin, sessions::Relation::Users.def());

        let with_expired = sessions.with_expired.unwrap_or(false);

        if !with_expired {
            query = query.filter(sessions::Column::ExpiresAt.gt(Utc::now().timestamp()));
        }

        if let Some(sort) = sessions.sort.as_ref() {
            query = match sessions.order.as_deref() {
                Some("desc") => sort.sort_desc(query),
                _ => sort.sort_asc(query),
            };
        }

        if let Some(search) = sessions.search {
            let maybe_uuid = Uuid::parse_str(search.as_str()).ok();

            if let Some(uuid) = maybe_uuid {
                query = query.filter(sessions::Column::Id.eq(uuid));
            } else {
                query = query.filter(
                    users::Column::Email
                        .contains(search.as_str())
                        .or(sessions::Column::Ip.contains(search.as_str()))
                        .or(sessions::Column::UserAgent.contains(search.as_str())),
                );
            }
        }

        query = query.limit(sessions.limit.unwrap_or(15));
        query = query.offset(sessions.offset.unwrap_or(0));

        let sessions = query
            .into_model::<Session>()
            .all(self.repository.connection())
            .await?;

        Ok(sessions)
    }

    /// Kill the session instantly
    pub(crate) async fn kill(&self, id: Uuid) -> AppResult<()> {
        let session = sessions::Entity::find_by_id(id)
            .one(self.repository.connection())
            .await?
            .ok_or_else(|| error::Error::NotFound("Session not found".to_string()))?;

        let mut active_model: ActiveModel = session.into();
        active_model.deleted_at = ActiveValue::Set(Some(Utc::now().timestamp()));
        active_model.refresh = ActiveValue::Set(None);

        sessions::Entity::update(active_model)
            .exec(self.repository.connection())
            .await?;

        Ok(())
    }

    /// Kill all session instantly for given user
    pub(crate) async fn kill_for(&self, user_id: Uuid) -> AppResult<u64> {
        let active_model = ActiveModel {
            expires_at: ActiveValue::Set(Utc::now().timestamp()),
            deleted_at: ActiveValue::Set(Some(Utc::now().timestamp())),
            refresh: ActiveValue::Set(None),
            ..Default::default()
        };

        let results = sessions::Entity::update_many()
            .filter(sessions::Column::UserId.eq(user_id))
            .filter(sessions::Column::DeletedAt.gt(Utc::now().naive_utc()))
            .set(active_model)
            .exec(self.repository.connection())
            .await?;

        Ok(results.rows_affected)
    }
}
