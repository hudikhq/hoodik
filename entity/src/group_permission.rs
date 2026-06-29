//! The canonical group-membership permission helper, parallel to
//! [`crate::permission`]. Every group-mutating route gates with one of the
//! `GroupRole` capability methods; no ad-hoc owner-id checks.
//!
//! This role is a **different axis** from the file-level
//! [`crate::permission::SharePermission`]. `GroupRole` answers "what may
//! this user do to the *group*" (view it, share files into it, manage its
//! roster). `SharePermission` answers "what may this user do to a shared
//! *file*". They reuse the words reader/editor/co-owner but never the same
//! column: group roles live in `share_group_members.role`, file roles in
//! `user_files.share_role`. A group co-owner manages the group; a file
//! co-owner reshares the file. The two are independent.

use error::AppResult;
use sea_orm::ConnectionTrait;

use crate::{
    share_group_members, share_groups, ColumnTrait, EntityTrait, QueryFilter, Uuid,
};

/// A caller's effective standing in a group. The group owner is always
/// `Owner` (the owner has no `share_group_members` row); every other
/// participant maps from their stored `share_group_members.role`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GroupRole {
    Owner,
    CoOwner,
    Editor,
    Reader,
    None,
}

impl GroupRole {
    /// May see the group exists and that they belong to it.
    pub fn can_view_group(self) -> bool {
        !matches!(self, Self::None)
    }

    /// May manage the roster: add/remove members, set member roles.
    pub fn can_manage_group(self) -> bool {
        matches!(self, Self::Owner | Self::CoOwner)
    }

    /// May rename the group. Owner-only — the group's identity is the
    /// owner's to set; co-owners manage the roster, not the group itself.
    /// Asymmetric with management for the same reason as `can_delete_group`.
    pub fn can_rename_group(self) -> bool {
        matches!(self, Self::Owner)
    }

    /// May delete the group. Owner-only — destructive and asymmetric with
    /// management, mirroring the file owner being the only role that
    /// `can_delete_file`.
    pub fn can_delete_group(self) -> bool {
        matches!(self, Self::Owner)
    }

    /// May the caller move a member's role from `target_current` to
    /// `target_new`?
    ///
    /// The owner may set any role, including co-owner. A co-owner may set
    /// reader/editor but never co-owner (no granting a peer at their own
    /// level), and never act on the owner or another co-owner (no demoting
    /// a peer or superior). This mirrors the file-share `cannot_grant_equal_role`
    /// guard. A reader/editor/non-member can set nothing.
    pub fn can_set_role(self, target_current: GroupRole, target_new: GroupRole) -> bool {
        match self {
            Self::Owner => true,
            Self::CoOwner => {
                !matches!(target_new, Self::CoOwner)
                    && !matches!(target_current, Self::Owner | Self::CoOwner)
            }
            _ => false,
        }
    }

    /// Wire/storage string for a member row's role. `Owner` and `None`
    /// have no `share_group_members.role` value and return `None`.
    pub fn as_member_role_str(self) -> Option<&'static str> {
        match self {
            Self::CoOwner => Some("co-owner"),
            Self::Editor => Some("editor"),
            Self::Reader => Some("reader"),
            Self::Owner | Self::None => None,
        }
    }
}

/// Map a `share_group_members.role` string to a non-owner `GroupRole`.
/// An unrecognised value degrades to the least-privileged tier rather
/// than panicking — the schema CHECK keeps the set closed, this is
/// defence in depth.
pub fn member_role_from_str(role: &str) -> GroupRole {
    match role {
        "co-owner" => GroupRole::CoOwner,
        "editor" => GroupRole::Editor,
        _ => GroupRole::Reader,
    }
}

/// Resolve `(group_id, user_id)` to a [`GroupRole`] in one indexed
/// lookup of the group row plus, for non-owners, the membership row.
pub async fn group_role(
    db: &impl ConnectionTrait,
    group_id: Uuid,
    user_id: Uuid,
) -> AppResult<GroupRole> {
    let group = share_groups::Entity::find_by_id(group_id).one(db).await?;
    let Some(group) = group else {
        return Ok(GroupRole::None);
    };
    if group.owner_id == user_id {
        return Ok(GroupRole::Owner);
    }
    let member = share_group_members::Entity::find_by_id((group_id, user_id))
        .one(db)
        .await?;
    Ok(member
        .map(|m| member_role_from_str(&m.role))
        .unwrap_or(GroupRole::None))
}

/// Effective group role for every `user_id` in `member_ids` within a
/// single group, in two queries (the group row + one `IN` over the
/// membership rows). Use for list/batch endpoints to avoid an N+1.
/// Ids absent from the membership table resolve to `None`; the owner
/// (if present in `member_ids`) resolves to `Owner`.
pub async fn group_roles_for(
    db: &impl ConnectionTrait,
    group_id: Uuid,
    member_ids: &[Uuid],
) -> AppResult<std::collections::HashMap<Uuid, GroupRole>> {
    use std::collections::HashMap;

    let mut out: HashMap<Uuid, GroupRole> =
        member_ids.iter().map(|id| (*id, GroupRole::None)).collect();
    if member_ids.is_empty() {
        return Ok(out);
    }

    let owner_id = share_groups::Entity::find_by_id(group_id)
        .one(db)
        .await?
        .map(|g| g.owner_id);

    let rows = share_group_members::Entity::find()
        .filter(share_group_members::Column::GroupId.eq(group_id))
        .filter(share_group_members::Column::UserId.is_in(member_ids.iter().copied()))
        .all(db)
        .await?;
    for row in rows {
        out.insert(row.user_id, member_role_from_str(&row.role));
    }
    if let Some(owner_id) = owner_id {
        if out.contains_key(&owner_id) {
            out.insert(owner_id, GroupRole::Owner);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_matrix() {
        assert!(GroupRole::Owner.can_view_group());
        assert!(GroupRole::Owner.can_manage_group());
        assert!(GroupRole::Owner.can_rename_group());
        assert!(GroupRole::Owner.can_delete_group());

        assert!(GroupRole::CoOwner.can_view_group());
        assert!(GroupRole::CoOwner.can_manage_group());
        assert!(!GroupRole::CoOwner.can_rename_group());
        assert!(!GroupRole::CoOwner.can_delete_group());

        assert!(GroupRole::Editor.can_view_group());
        assert!(!GroupRole::Editor.can_manage_group());
        assert!(!GroupRole::Editor.can_rename_group());
        assert!(!GroupRole::Editor.can_delete_group());

        assert!(GroupRole::Reader.can_view_group());
        assert!(!GroupRole::Reader.can_manage_group());
        assert!(!GroupRole::Reader.can_rename_group());
        assert!(!GroupRole::Reader.can_delete_group());

        assert!(!GroupRole::None.can_view_group());
        assert!(!GroupRole::None.can_manage_group());
        assert!(!GroupRole::None.can_rename_group());
        assert!(!GroupRole::None.can_delete_group());
    }

    #[test]
    fn owner_may_set_any_role() {
        for new in [GroupRole::Reader, GroupRole::Editor, GroupRole::CoOwner] {
            for current in [GroupRole::Reader, GroupRole::Editor, GroupRole::CoOwner] {
                assert!(GroupRole::Owner.can_set_role(current, new));
            }
        }
    }

    #[test]
    fn co_owner_cannot_grant_co_owner() {
        assert!(!GroupRole::CoOwner.can_set_role(GroupRole::Reader, GroupRole::CoOwner));
        assert!(!GroupRole::CoOwner.can_set_role(GroupRole::Editor, GroupRole::CoOwner));
    }

    #[test]
    fn co_owner_cannot_demote_owner_or_peer() {
        assert!(!GroupRole::CoOwner.can_set_role(GroupRole::Owner, GroupRole::Reader));
        assert!(!GroupRole::CoOwner.can_set_role(GroupRole::CoOwner, GroupRole::Reader));
    }

    #[test]
    fn co_owner_may_set_reader_editor_on_lesser_members() {
        assert!(GroupRole::CoOwner.can_set_role(GroupRole::Reader, GroupRole::Editor));
        assert!(GroupRole::CoOwner.can_set_role(GroupRole::Editor, GroupRole::Reader));
    }

    #[test]
    fn lesser_roles_cannot_set_anything() {
        for role in [GroupRole::Editor, GroupRole::Reader, GroupRole::None] {
            assert!(!role.can_set_role(GroupRole::Reader, GroupRole::Editor));
        }
    }

    #[test]
    fn member_role_from_str_maps_and_degrades() {
        assert_eq!(member_role_from_str("co-owner"), GroupRole::CoOwner);
        assert_eq!(member_role_from_str("editor"), GroupRole::Editor);
        assert_eq!(member_role_from_str("reader"), GroupRole::Reader);
        assert_eq!(member_role_from_str("nonsense"), GroupRole::Reader);
    }

    #[test]
    fn as_member_role_str_round_trip() {
        assert_eq!(GroupRole::Reader.as_member_role_str(), Some("reader"));
        assert_eq!(GroupRole::Editor.as_member_role_str(), Some("editor"));
        assert_eq!(GroupRole::CoOwner.as_member_role_str(), Some("co-owner"));
        assert!(GroupRole::Owner.as_member_role_str().is_none());
        assert!(GroupRole::None.as_member_role_str().is_none());
    }
}
