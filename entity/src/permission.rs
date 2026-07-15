//! The canonical permission helper. Every mutating route gates with one
//! of the `SharePermission` capability methods (`can_read`, `can_write`,
//! `can_reshare`, `can_fork`, `can_delete_file`); no ad-hoc role checks.
//! Lives in `entity/` to avoid a `shares → storage` crate cycle.

use error::AppResult;
use sea_orm::ConnectionTrait;

use crate::{user_files, ColumnTrait, EntityTrait, QueryFilter, Uuid};

/// Outcome of a `(file_id, user_id)` lookup against `user_files`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SharePermission {
    Owner,
    CoOwner,
    Editor,
    Reader,
    None,
}

impl SharePermission {
    pub fn can_read(self) -> bool {
        !matches!(self, Self::None)
    }

    pub fn can_write(self) -> bool {
        matches!(self, Self::Owner | Self::CoOwner | Self::Editor)
    }

    pub fn can_reshare(self) -> bool {
        matches!(self, Self::Owner | Self::CoOwner)
    }

    pub fn can_fork(self) -> bool {
        matches!(self, Self::Owner | Self::CoOwner)
    }

    pub fn can_delete_file(self) -> bool {
        matches!(self, Self::Owner)
    }

    /// Wire string the schema's CHECK constraint accepts for non-owner
    /// rows. Returns `None` for `Owner` and `None` because those values
    /// don't map to a `share_role` column value.
    pub fn as_share_role_str(self) -> Option<&'static str> {
        match self {
            Self::CoOwner => Some("co-owner"),
            Self::Editor => Some("editor"),
            Self::Reader => Some("reader"),
            Self::Owner | Self::None => None,
        }
    }
}

/// Resolve `(file_id, user_id)` to a `SharePermission` via one indexed
/// lookup. Owner rows return `Owner` regardless of their `share_role`
/// value (the convention is `'co-owner'` from migration 1; the helper
/// treats it as moot).
pub async fn permission(
    db: &impl ConnectionTrait,
    file_id: Uuid,
    user_id: Uuid,
) -> AppResult<SharePermission> {
    let row = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file_id))
        .filter(user_files::Column::UserId.eq(user_id))
        .one(db)
        .await?;

    Ok(map_row(row.as_ref()))
}

/// Batch variant for routes that act on many files at once (`delete_many`,
/// `move_many`). Returns a permission per requested id, with `None` for
/// ids that have no `user_files` row for the caller.
pub async fn permissions_for(
    db: &impl ConnectionTrait,
    file_ids: &[Uuid],
    user_id: Uuid,
) -> AppResult<std::collections::HashMap<Uuid, SharePermission>> {
    use std::collections::HashMap;

    if file_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(user_id))
        .filter(user_files::Column::FileId.is_in(file_ids.iter().copied()))
        .all(db)
        .await?;

    let mut by_file: HashMap<Uuid, SharePermission> = file_ids
        .iter()
        .map(|id| (*id, SharePermission::None))
        .collect();
    for row in rows {
        by_file.insert(row.file_id, map_row(Some(&row)));
    }
    Ok(by_file)
}

fn map_row(row: Option<&user_files::Model>) -> SharePermission {
    match row {
        None => SharePermission::None,
        Some(r) if r.is_owner => SharePermission::Owner,
        Some(r) => match r.share_role.as_str() {
            "co-owner" => SharePermission::CoOwner,
            "editor" => SharePermission::Editor,
            "reader" => SharePermission::Reader,
            // Defence in depth — a row that slipped past the CHECK
            // constraint via some unknown ingest path degrades to the
            // least-privileged tier rather than panicking. Logged at the
            // call site if the actual share_role surfaces.
            _ => SharePermission::Reader,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_matrix() {
        // Matrix from the three-tier role table.
        assert!(SharePermission::Owner.can_read());
        assert!(SharePermission::Owner.can_write());
        assert!(SharePermission::Owner.can_reshare());
        assert!(SharePermission::Owner.can_fork());
        assert!(SharePermission::Owner.can_delete_file());

        assert!(SharePermission::CoOwner.can_read());
        assert!(SharePermission::CoOwner.can_write());
        assert!(SharePermission::CoOwner.can_reshare());
        assert!(SharePermission::CoOwner.can_fork());
        assert!(!SharePermission::CoOwner.can_delete_file());

        assert!(SharePermission::Editor.can_read());
        assert!(SharePermission::Editor.can_write());
        assert!(!SharePermission::Editor.can_reshare());
        assert!(!SharePermission::Editor.can_fork());
        assert!(!SharePermission::Editor.can_delete_file());

        assert!(SharePermission::Reader.can_read());
        assert!(!SharePermission::Reader.can_write());
        assert!(!SharePermission::Reader.can_reshare());
        assert!(!SharePermission::Reader.can_fork());
        assert!(!SharePermission::Reader.can_delete_file());

        assert!(!SharePermission::None.can_read());
        assert!(!SharePermission::None.can_write());
        assert!(!SharePermission::None.can_reshare());
        assert!(!SharePermission::None.can_fork());
        assert!(!SharePermission::None.can_delete_file());
    }

    #[test]
    fn owner_row_with_any_share_role_resolves_to_owner() {
        for role in ["reader", "editor", "co-owner", "anything"] {
            let row = user_files::Model {
                id: Uuid::nil(),
                file_id: Uuid::nil(),
                user_id: Uuid::nil(),
                encrypted_key: String::new(),
                is_owner: true,
                created_at: 0,
                expires_at: None,
                share_role: role.to_string(),
                shared_at: None,
                shared_by_user_id: None,
                member_signature: None,
                member_signed_at: None,
            };
            assert_eq!(map_row(Some(&row)), SharePermission::Owner);
        }
    }

    #[test]
    fn non_owner_row_maps_share_role_to_enum() {
        let cases = [
            ("co-owner", SharePermission::CoOwner),
            ("editor", SharePermission::Editor),
            ("reader", SharePermission::Reader),
            ("unknown-role", SharePermission::Reader),
        ];
        for (role, expect) in cases {
            let row = user_files::Model {
                id: Uuid::nil(),
                file_id: Uuid::nil(),
                user_id: Uuid::nil(),
                encrypted_key: String::new(),
                is_owner: false,
                created_at: 0,
                expires_at: None,
                share_role: role.to_string(),
                shared_at: None,
                shared_by_user_id: None,
                member_signature: None,
                member_signed_at: None,
            };
            assert_eq!(map_row(Some(&row)), expect);
        }
    }

    #[test]
    fn missing_row_maps_to_none() {
        assert_eq!(map_row(None), SharePermission::None);
    }

    #[test]
    fn as_share_role_str_round_trip() {
        assert_eq!(SharePermission::Reader.as_share_role_str(), Some("reader"));
        assert_eq!(SharePermission::Editor.as_share_role_str(), Some("editor"));
        assert_eq!(
            SharePermission::CoOwner.as_share_role_str(),
            Some("co-owner")
        );
        assert!(SharePermission::Owner.as_share_role_str().is_none());
        assert!(SharePermission::None.as_share_role_str().is_none());
    }
}
