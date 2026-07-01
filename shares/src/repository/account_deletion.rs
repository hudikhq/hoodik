//! Pre-emit audit rows before a user row is deleted.
//!
//! After the engine-level FK CASCADE on `user_files.user_id` and
//! `user_files.shared_by_user_id` fires, the per-user_files-row context
//! is gone — there'd be no way to record the action history. The
//! caller (`admin::repository::users::delete`) drives this helper as
//! step 1 of the transaction, then performs the actual user delete; the
//! FK constraints on `share_events.sender_id` / `recipient_id` (SET
//! NULL) turn the attribution into "(deleted)" after the cascade fires
//! but leave the row in place with its hash-chain entry intact.

use entity::{
    user_files, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Uuid,
};
use error::AppResult;

use crate::{contracts::audit::NewAuditEvent, repository::audit};

/// Emit one `share_events` row per side-effect of deleting `user_id`,
/// before the actual `DELETE FROM users` runs. All three classes of row
/// are system-attributed (no signature) — there is no human actor whose
/// privkey could sign cleanup events.
pub async fn pre_emit_for_user_delete<C: ConnectionTrait>(
    db: &C,
    user_id: Uuid,
    now: i64,
) -> AppResult<()> {
    // Incoming shares — every row where the deleted user was a
    // recipient. Action `revoke` records that they lost access.
    let incoming: Vec<user_files::Model> = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(user_id))
        .filter(user_files::Column::IsOwner.eq(false))
        .all(db)
        .await?;
    for row in &incoming {
        audit::append_event(
            db,
            NewAuditEvent {
                sender_id: Some(user_id),
                recipient_id: Some(user_id),
                file_id: row.file_id,
                action_str: "revoke",
                share_role_before: role_to_static_str(&row.share_role),
                share_role_after: None,
                created_at: now,
                event_signature: None,
            },
        )
        .await?;
    }

    // Outgoing Co-owner / Owner-as-granter grants — every row the user
    // had vouched for. Action `shared_by_co_owner_revoked` mirrors the
    // in-life Co-owner cascade.
    let outgoing: Vec<user_files::Model> = user_files::Entity::find()
        .filter(user_files::Column::SharedByUserId.eq(user_id))
        .filter(user_files::Column::IsOwner.eq(false))
        .all(db)
        .await?;
    for row in &outgoing {
        audit::append_event(
            db,
            NewAuditEvent {
                sender_id: Some(user_id),
                recipient_id: Some(row.user_id),
                file_id: row.file_id,
                action_str: "shared_by_co_owner_revoked",
                share_role_before: role_to_static_str(&row.share_role),
                share_role_after: None,
                created_at: now,
                event_signature: None,
            },
        )
        .await?;
    }

    // Owned files — for each file the user owned, every recipient
    // loses access via the `files.id` cascade chain. One `revoke` row
    // per affected recipient so the audit log shows the consequences.
    let owned_file_ids: Vec<Uuid> = user_files::Entity::find()
        .select_only()
        .column(user_files::Column::FileId)
        .filter(user_files::Column::UserId.eq(user_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .into_tuple()
        .all(db)
        .await?;
    if !owned_file_ids.is_empty() {
        let recipient_rows: Vec<user_files::Model> = user_files::Entity::find()
            .filter(user_files::Column::FileId.is_in(owned_file_ids.clone()))
            .filter(user_files::Column::IsOwner.eq(false))
            .all(db)
            .await?;
        for row in &recipient_rows {
            audit::append_event(
                db,
                NewAuditEvent {
                    sender_id: Some(user_id),
                    recipient_id: Some(row.user_id),
                    file_id: row.file_id,
                    action_str: "revoke",
                    share_role_before: role_to_static_str(&row.share_role),
                    share_role_after: None,
                    created_at: now,
                    event_signature: None,
                },
            )
            .await?;
        }
    }

    Ok(())
}

fn role_to_static_str(role: &str) -> Option<&'static str> {
    match role {
        "reader" => Some("reader"),
        "editor" => Some("editor"),
        "co-owner" => Some("co-owner"),
        _ => None,
    }
}
