//! Share-group CRUD, role management, and roster.
//!
//! A share group is a named recipient bag with role-scoped membership.
//! The owner is the implicit co-owner; members hold a `group_role`
//! (reader/editor/co-owner) governing what they may do *to the group*.
//! That axis is distinct from the *file* role a shared file lands at.
//!
//! The group carries no file associations of its own. Sharing to a group
//! is a client-side fan-out: the client reads the roster, then issues one
//! ordinary share per recipient. The server's only group writes are CRUD
//! and roster mutations.

use std::collections::HashMap;

use entity::{
    group_permission::{group_role, member_role_from_str, GroupRole},
    share_group_members, share_groups, users, ActiveValue, ColumnTrait, DbErr, EntityTrait, Order,
    QueryFilter, QueryOrder, SqlErr, Uuid,
};
use error::{AppResult, Error};

use crate::{
    data::group::{
        decode_nonce, AddGroupMemberBody, AppShareGroup, AppShareGroupAsMember,
        AppShareGroupMember, AppShareGroupWithMembers, CreateGroupBody, GroupMemberWithKey,
        GroupsResponse, RenameGroupBody, SetMemberRoleBody,
    },
    repository::{nonce, Repository},
};
use validr::Validation;

const REPLAY_WINDOW_SECONDS: i64 = 300;

impl Repository<'_> {
    /// Create a new group owned by `caller`. Names are unique per owner
    /// (DB-enforced via `uniq_share_groups_owner_name`); the duplicate
    /// case maps to 409 so the client can re-prompt with a clean error.
    pub(crate) async fn create_group(
        &self,
        caller: &users::Model,
        body: CreateGroupBody,
    ) -> AppResult<AppShareGroup> {
        let validated = body.validate()?;
        let name = validated.name.unwrap();
        let now = chrono::Utc::now().timestamp();
        let id = Uuid::new_v4();

        let active = share_groups::ActiveModel {
            id: ActiveValue::Set(id),
            owner_id: ActiveValue::Set(caller.id),
            name: ActiveValue::Set(name),
            created_at: ActiveValue::Set(now),
        };

        match share_groups::Entity::insert(active)
            .exec_without_returning(&self.context.db)
            .await
        {
            Ok(_) => {}
            Err(err) => return Err(map_unique_violation(err)),
        }

        let row = share_groups::Entity::find_by_id(id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::InternalError("share_group_insert_lost".to_string()))?;
        Ok(row.into())
    }

    /// Drop a group. Owner-only (`can_delete_group`) — destructive and
    /// deliberately not delegated to co-owners. FK cascade clears
    /// `share_group_members`. A missing or non-owned group returns 404 so
    /// the surface can't probe group ids.
    pub(crate) async fn delete_group(
        &self,
        caller: &users::Model,
        group_id: Uuid,
    ) -> AppResult<()> {
        let role = group_role(&self.context.db, group_id, caller.id).await?;
        if !role.can_delete_group() {
            return Err(Error::NotFound("group_not_found".to_string()));
        }
        share_groups::Entity::delete_by_id(group_id)
            .exec(&self.context.db)
            .await?;
        Ok(())
    }

    /// Rename a group. Owner-only (`can_rename_group`) — co-owners manage the
    /// roster but the group's name is the owner's to set, mirroring delete.
    /// Re-checks the per-owner unique-name constraint, mapping a clash to
    /// 409 just like create.
    pub(crate) async fn rename_group(
        &self,
        caller: &users::Model,
        group_id: Uuid,
        body: RenameGroupBody,
    ) -> AppResult<AppShareGroup> {
        let validated = body.validate()?;
        let name = validated.name.unwrap();

        let role = group_role(&self.context.db, group_id, caller.id).await?;
        if matches!(role, GroupRole::None) {
            return Err(Error::NotFound("group_not_found".to_string()));
        }
        if !role.can_rename_group() {
            return Err(Error::Forbidden("not_group_owner".to_string()));
        }

        let group = share_groups::Entity::find_by_id(group_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("group_not_found".to_string()))?;

        match share_groups::Entity::update(share_groups::ActiveModel {
            id: ActiveValue::Unchanged(group_id),
            owner_id: ActiveValue::Unchanged(group.owner_id),
            name: ActiveValue::Set(name),
            created_at: ActiveValue::Unchanged(group.created_at),
        })
        .exec(&self.context.db)
        .await
        {
            Ok(row) => Ok(row.into()),
            Err(err) => Err(map_unique_violation(err)),
        }
    }

    /// List the caller's owned groups (with members + each member's group
    /// role) plus the groups they're a member of (with the owner's email
    /// and the caller's own group role, but no peer roster — peers in
    /// someone else's group are not the caller's data to enumerate).
    pub(crate) async fn list_groups(
        &self,
        caller: &users::Model,
    ) -> AppResult<GroupsResponse> {
        let owned_rows = share_groups::Entity::find()
            .filter(share_groups::Column::OwnerId.eq(caller.id))
            .order_by(share_groups::Column::CreatedAt, Order::Asc)
            .all(&self.context.db)
            .await?;

        let mut owned = Vec::with_capacity(owned_rows.len());
        for group in owned_rows {
            let members = self.member_rows(group.id).await?;
            owned.push(AppShareGroupWithMembers {
                id: group.id,
                owner_id: group.owner_id,
                name: group.name,
                created_at: group.created_at,
                members,
            });
        }

        let member_of_rows: Vec<(share_group_members::Model, Option<share_groups::Model>)> =
            share_group_members::Entity::find()
                .filter(share_group_members::Column::UserId.eq(caller.id))
                .find_also_related(share_groups::Entity)
                .all(&self.context.db)
                .await?;

        let owner_ids: Vec<Uuid> = member_of_rows
            .iter()
            .filter_map(|(_, group)| group.as_ref().map(|g| g.owner_id))
            .collect();
        let owners: HashMap<Uuid, String> = if owner_ids.is_empty() {
            HashMap::new()
        } else {
            users::Entity::find()
                .filter(users::Column::Id.is_in(owner_ids))
                .all(&self.context.db)
                .await?
                .into_iter()
                .map(|u| (u.id, u.email))
                .collect()
        };

        let mut member_of = Vec::with_capacity(member_of_rows.len());
        for (member, group) in member_of_rows {
            let Some(group) = group else { continue };
            // A user can in principle own a group they also have a row
            // in (a self-add edge case); the owner-of section already
            // covers that, so suppress the duplicate here.
            if group.owner_id == caller.id {
                continue;
            }
            let owner_email = owners.get(&group.owner_id).cloned().unwrap_or_default();
            member_of.push(AppShareGroupAsMember {
                id: group.id,
                owner_id: group.owner_id,
                owner_email,
                name: group.name,
                created_at: group.created_at,
                added_at: member.added_at,
                group_role: member.role,
            });
        }

        Ok(GroupsResponse { owned, member_of })
    }

    /// Add `member` to `group` at `group_role`. A group is a saved
    /// recipient selection with no file associations, so this is a plain
    /// roster insert — no key wrapping, no signatures, no cascade. The
    /// caller must `can_manage_group`, and a co-owner caller may not mint
    /// another co-owner (the privilege-escalation guard). Idempotent on the
    /// membership row (composite PK); a repeat add refreshes the stored
    /// role. Replay is bounded by the timestamp window and the
    /// per-`(caller, nonce)` dedup.
    pub(crate) async fn add_group_member(
        &self,
        caller: &users::Model,
        group_id: Uuid,
        body: AddGroupMemberBody,
    ) -> AppResult<()> {
        let validated = body.validate()?;
        let new_member_id = Uuid::parse_str(&validated.user_id.unwrap())
            .map_err(|_| Error::BadRequest("user_id_invalid".to_string()))?;
        let claimed_fingerprint = validated.pubkey_fingerprint.unwrap();
        let new_member_group_role = member_role_from_str(&validated.group_role.unwrap());
        let signed_timestamp = validated.timestamp.unwrap();
        let nonce = decode_nonce(&validated.nonce.unwrap())?;

        let caller_role = group_role(&self.context.db, group_id, caller.id).await?;
        if matches!(caller_role, GroupRole::None) {
            return Err(Error::NotFound("group_not_found".to_string()));
        }
        if !caller_role.can_manage_group() {
            return Err(Error::Forbidden("not_group_manager".to_string()));
        }
        // A co-owner cannot mint another co-owner — same guard as setting
        // a member to co-owner after the fact.
        if !caller_role.can_set_role(GroupRole::None, new_member_group_role) {
            return Err(Error::Forbidden("cannot_grant_equal_role".to_string()));
        }
        if new_member_id == caller.id {
            return Err(Error::BadRequest("cannot_add_self".to_string()));
        }

        let now = chrono::Utc::now().timestamp();
        if (now - signed_timestamp).abs() > REPLAY_WINDOW_SECONDS {
            return Err(Error::BadRequest("replay_timestamp_skew".to_string()));
        }
        if nonce::check_and_record(caller.id, nonce, now) {
            return Err(Error::Conflict("replay_nonce_seen".to_string()));
        }

        let member = users::Entity::find_by_id(new_member_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("user_not_found".to_string()))?;
        if !member.fingerprint.eq_ignore_ascii_case(&claimed_fingerprint) {
            return Err(Error::Conflict("recipient_pubkey_changed".to_string()));
        }

        let role_str = new_member_group_role
            .as_member_role_str()
            .unwrap_or("reader")
            .to_string();
        let already_member = share_group_members::Entity::find_by_id((group_id, new_member_id))
            .one(&self.context.db)
            .await?;
        if let Some(existing) = already_member {
            share_group_members::Entity::update(share_group_members::ActiveModel {
                group_id: ActiveValue::Unchanged(group_id),
                user_id: ActiveValue::Unchanged(new_member_id),
                added_at: ActiveValue::Unchanged(existing.added_at),
                role: ActiveValue::Set(role_str),
            })
            .exec(&self.context.db)
            .await?;
        } else {
            share_group_members::Entity::insert(share_group_members::ActiveModel {
                group_id: ActiveValue::Set(group_id),
                user_id: ActiveValue::Set(new_member_id),
                added_at: ActiveValue::Set(signed_timestamp),
                role: ActiveValue::Set(role_str),
            })
            .exec_without_returning(&self.context.db)
            .await?;
        }

        Ok(())
    }

    /// Set a member's group role. Requires `can_manage_group` plus the
    /// privilege-escalation guard (`can_set_role`): a co-owner may set
    /// reader/editor but never co-owner, and never act on the owner or
    /// another co-owner. Pure roster metadata — no file key moves, so no
    /// crypto payload.
    pub(crate) async fn set_member_role(
        &self,
        caller: &users::Model,
        group_id: Uuid,
        member_id: Uuid,
        body: SetMemberRoleBody,
    ) -> AppResult<()> {
        let validated = body.validate()?;
        let new_role = member_role_from_str(&validated.group_role.unwrap());

        let caller_role = group_role(&self.context.db, group_id, caller.id).await?;
        if matches!(caller_role, GroupRole::None) {
            return Err(Error::NotFound("group_not_found".to_string()));
        }
        if !caller_role.can_manage_group() {
            return Err(Error::Forbidden("not_group_manager".to_string()));
        }

        let target_role = group_role(&self.context.db, group_id, member_id).await?;
        if matches!(target_role, GroupRole::None) {
            return Err(Error::NotFound("member_not_found".to_string()));
        }
        // The owner has no member row to update, and demoting them is
        // never allowed; `can_set_role` already rejects acting on Owner.
        if !caller_role.can_set_role(target_role, new_role) {
            return Err(Error::Forbidden("cannot_set_role".to_string()));
        }

        let membership = share_group_members::Entity::find_by_id((group_id, member_id))
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("member_not_found".to_string()))?;
        share_group_members::Entity::update(share_group_members::ActiveModel {
            group_id: ActiveValue::Unchanged(group_id),
            user_id: ActiveValue::Unchanged(member_id),
            added_at: ActiveValue::Unchanged(membership.added_at),
            role: ActiveValue::Set(new_role.as_member_role_str().unwrap_or("reader").to_string()),
        })
        .exec(&self.context.db)
        .await?;
        Ok(())
    }

    /// Drop a member from a group. Requires `can_manage_group`, with one
    /// exception: a member may remove **themselves** (self-leave),
    /// mirroring the self-revoke exception on the file-share DELETE route.
    /// Co-owners cannot remove the owner. Idempotent.
    pub(crate) async fn remove_group_member(
        &self,
        caller: &users::Model,
        group_id: Uuid,
        member_id: Uuid,
    ) -> AppResult<()> {
        let caller_role = group_role(&self.context.db, group_id, caller.id).await?;
        if matches!(caller_role, GroupRole::None) {
            return Err(Error::NotFound("group_not_found".to_string()));
        }

        let is_self_remove = member_id == caller.id;
        if !is_self_remove {
            if !caller_role.can_manage_group() {
                return Err(Error::Forbidden("not_group_manager".to_string()));
            }
            // The owner can never be removed (they have no member row to
            // delete anyway); reject explicitly so a co-owner can't try.
            let target_role = group_role(&self.context.db, group_id, member_id).await?;
            if matches!(target_role, GroupRole::Owner) {
                return Err(Error::Forbidden("cannot_remove_owner".to_string()));
            }
        }

        share_group_members::Entity::delete_by_id((group_id, member_id))
            .exec(&self.context.db)
            .await?;
        Ok(())
    }

    /// The group's full recipient set — owner plus every member — each with
    /// the pubkey material the client needs to wrap a file key for a
    /// share-to-group fan-out. Visible to anyone in the group, since every
    /// member is a candidate recipient when another member shares to the
    /// group. A non-member gets 404 so the surface can't probe group ids.
    pub(crate) async fn group_members_roster(
        &self,
        caller: &users::Model,
        group_id: Uuid,
    ) -> AppResult<Vec<GroupMemberWithKey>> {
        let group = share_groups::Entity::find_by_id(group_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("group_not_found".to_string()))?;

        let caller_role = group_role(&self.context.db, group_id, caller.id).await?;
        if matches!(caller_role, GroupRole::None) {
            return Err(Error::NotFound("group_not_found".to_string()));
        }

        let member_rows = share_group_members::Entity::find()
            .filter(share_group_members::Column::GroupId.eq(group_id))
            .order_by(share_group_members::Column::AddedAt, Order::Asc)
            .all(&self.context.db)
            .await?;

        let mut user_ids: Vec<Uuid> = Vec::with_capacity(member_rows.len() + 1);
        user_ids.push(group.owner_id);
        user_ids.extend(member_rows.iter().map(|r| r.user_id));
        let users_by_id: HashMap<Uuid, users::Model> = users::Entity::find()
            .filter(users::Column::Id.is_in(user_ids))
            .all(&self.context.db)
            .await?
            .into_iter()
            .map(|u| (u.id, u))
            .collect();

        let mut roster = Vec::with_capacity(member_rows.len() + 1);
        if let Some(owner) = users_by_id.get(&group.owner_id) {
            roster.push(GroupMemberWithKey {
                user_id: owner.id,
                email: owner.email.clone(),
                pubkey: owner.pubkey.clone(),
                fingerprint: owner.fingerprint.clone(),
                group_role: "owner".to_string(),
            });
        }
        for row in member_rows {
            if let Some(user) = users_by_id.get(&row.user_id) {
                roster.push(GroupMemberWithKey {
                    user_id: user.id,
                    email: user.email.clone(),
                    pubkey: user.pubkey.clone(),
                    fingerprint: user.fingerprint.clone(),
                    group_role: row.role,
                });
            }
        }
        Ok(roster)
    }

    /// Member rows for the `list_groups` owned section — every member with
    /// their group role and join time, owner excluded (the owner is the
    /// caller and is already named on the group itself).
    async fn member_rows(&self, group_id: Uuid) -> AppResult<Vec<AppShareGroupMember>> {
        let rows = share_group_members::Entity::find()
            .filter(share_group_members::Column::GroupId.eq(group_id))
            .order_by(share_group_members::Column::AddedAt, Order::Asc)
            .all(&self.context.db)
            .await?;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let user_ids: Vec<Uuid> = rows.iter().map(|r| r.user_id).collect();
        let users_by_id: HashMap<Uuid, users::Model> = users::Entity::find()
            .filter(users::Column::Id.is_in(user_ids))
            .all(&self.context.db)
            .await?
            .into_iter()
            .map(|u| (u.id, u))
            .collect();
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                users_by_id.get(&row.user_id).map(|u| AppShareGroupMember {
                    user_id: row.user_id,
                    email: u.email.clone(),
                    fingerprint: u.fingerprint.clone(),
                    added_at: row.added_at,
                    group_role: row.role,
                })
            })
            .collect())
    }
}

fn map_unique_violation(err: DbErr) -> Error {
    // `sql_err()` normalises SQLite vs Postgres engine-specific error
    // shapes into the same `UniqueConstraintViolation` variant — the
    // duplicate name from migration 6's `uniq_share_groups_owner_name`
    // is the only unique index these writes can trip.
    if matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
        return Error::Conflict("group_name_taken".to_string());
    }
    Error::from(err)
}
