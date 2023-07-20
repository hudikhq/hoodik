use chrono::Utc;
use entity::{
    invitations::{self, ActiveModel},
    paginated::Paginated,
    sort::Sortable,
    users, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Uuid,
};
use error::{AppResult, Error};
use validr::Validation;

use crate::{
    data::invitations::{create::Create, search::Search},
    emails::invite,
};

use super::Repository;

pub(crate) struct InvitationsRepository<'ctx, T: ConnectionTrait> {
    repository: &'ctx Repository<'ctx, T>,
}

impl<'ctx, T> InvitationsRepository<'ctx, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(repository: &'ctx Repository<'ctx, T>) -> Self {
        Self { repository }
    }

    /// Find all the invitations, redeemed or not
    pub(crate) async fn find(
        &self,
        invitations: Search,
    ) -> AppResult<Paginated<invitations::Model>> {
        let invitations = invitations.validate()?;

        let mut query = invitations::Entity::find();

        let with_expired = invitations.with_expired.unwrap_or(false);

        if !with_expired {
            query = query.filter(invitations::Column::ExpiresAt.gt(Utc::now().timestamp()));
        }

        if let Some(sort) = invitations.sort.as_ref() {
            query = match invitations.order.as_deref() {
                Some("desc") => sort.sort_desc(query),
                _ => sort.sort_asc(query),
            };
        }

        if let Some(search) = invitations.search {
            let maybe_uuid = Uuid::parse_str(search.as_str()).ok();

            if let Some(uuid) = maybe_uuid {
                query = query.filter(invitations::Column::Id.eq(uuid));
            } else {
                query = query.filter(invitations::Column::Email.contains(search.as_str()));
            }
        }

        let total = query.clone().count(self.repository.connection()).await?;

        query = query.limit(invitations.limit.unwrap_or(15));
        query = query.offset(invitations.offset.unwrap_or(0));

        let invitations = query.all(self.repository.connection()).await?;

        Ok(Paginated::new(invitations, total))
    }

    /// Expire an invitation, this will make sure that the invitation
    /// is no longer valid and can't be used to register a user.
    pub(crate) async fn expire(&self, invitation_id: Uuid) -> AppResult<()> {
        let active_model = ActiveModel {
            id: ActiveValue::Set(invitation_id),
            expires_at: ActiveValue::Set(Utc::now().timestamp()),
            ..Default::default()
        };

        invitations::Entity::update(active_model)
            .exec(self.repository.connection())
            .await?;

        Ok(())
    }

    /// Invite a user to the platform, if the user already exists, throw error,
    /// if the platform has turned off free registration of the users, this invitation
    /// will be the only way to register
    pub(crate) async fn create(&self, invitation: Create) -> AppResult<invitations::Model> {
        let (email, message, role, quota, expires_at) = invitation.into_values()?;

        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email.as_str()))
            .one(self.repository.connection())
            .await?;

        if user.is_some() {
            return Err(Error::BadRequest(
                "User with this email already exists".to_string(),
            ));
        }

        let id = Uuid::new_v4();

        let model = ActiveModel {
            id: ActiveValue::Set(id),
            email: ActiveValue::Set(email.clone()),
            role: ActiveValue::Set(role),
            quota: ActiveValue::Set(quota),
            expires_at: ActiveValue::Set(expires_at),
            created_at: ActiveValue::Set(Utc::now().timestamp()),
            ..Default::default()
        };

        invitations::Entity::insert(model)
            .exec_without_returning(self.repository.connection())
            .await?;

        let invitation = invitations::Entity::find()
            .filter(invitations::Column::Id.eq(id))
            .one(self.repository.connection())
            .await?
            .ok_or_else(|| Error::NotFound("Invitation not found".to_string()))?;

        invite::send(self.repository.context(), &invitation, message).await?;

        Ok(invitation)
    }
}
