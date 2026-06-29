//! Thin wrappers around `entity::permission::permission` that map the
//! `SharePermission` outcome to the storage-route HTTP-error contract.
//! Every mutating storage route gates with one of
//! these helpers rather than re-deriving the mapping inline.

use entity::{
    permission::{permission, SharePermission},
    ConnectionTrait, Uuid,
};
use error::{AppResult, Error};

/// Read access: Owner / Co-owner / Editor / Reader → ok, None → 404.
/// Read paths return 404 (not 403) on missing access
/// so the file's existence does not leak.
pub(crate) async fn require_read<C: ConnectionTrait>(
    db: &C,
    file_id: Uuid,
    user_id: Uuid,
) -> AppResult<SharePermission> {
    let perm = permission(db, file_id, user_id).await?;
    if !perm.can_read() {
        return Err(Error::NotFound("file_not_found".to_string()));
    }
    Ok(perm)
}

/// Write access: Owner / Co-owner / Editor → ok. Reader → 403
/// `forbidden_read_only`. None → 404. Used by chunk-push, replace-
/// content, rename, move, hashes, version-restore.
pub(crate) async fn require_write<C: ConnectionTrait>(
    db: &C,
    file_id: Uuid,
    user_id: Uuid,
) -> AppResult<SharePermission> {
    let perm = permission(db, file_id, user_id).await?;
    match perm {
        SharePermission::Owner | SharePermission::CoOwner | SharePermission::Editor => Ok(perm),
        SharePermission::Reader => Err(Error::Forbidden("forbidden_read_only".to_string())),
        SharePermission::None => Err(Error::NotFound("file_not_found".to_string())),
    }
}

/// Owner-only mutations: set-editable, delete (cascade), delete-version,
/// purge-all-history, name-hash lookup. Anything that touches the file's
/// fundamental identity or layout. Non-owner → 403 `forbidden_not_owner`;
/// None → 404.
pub(crate) async fn require_owner<C: ConnectionTrait>(
    db: &C,
    file_id: Uuid,
    user_id: Uuid,
) -> AppResult<()> {
    let perm = permission(db, file_id, user_id).await?;
    match perm {
        SharePermission::Owner => Ok(()),
        SharePermission::None => Err(Error::NotFound("file_not_found".to_string())),
        _ => Err(Error::Forbidden("forbidden_not_owner".to_string())),
    }
}

