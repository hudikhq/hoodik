//! Per-sender SHA-256 hash chain over `share_events`.
//! Each row's `this_event_hash` is
//! `SHA-256("hoodik-audit-v1\0" || prev_event_hash || der(AuditEventRowV1))`,
//! where `prev_event_hash` is the previous row's hash for the same
//! `sender_id` (or 32 zero bytes if first). System-cascade rows
//! (`sender_id = NULL`) form their own NULL-sender chain.

use cryptfns::asn1::{
    encode_audit_event_v1, AuditEventRowV1, ShareRoleEnum, AUDIT_EVENT_V1_PREFIX,
};
use entity::{
    share_events, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, Order, QueryFilter,
    QueryOrder, Uuid,
};
use error::{AppResult, Error};
use sha2::{Digest, Sha256};

use crate::contracts::audit::NewAuditEvent;

const PREV_HASH_LEN: usize = 32;

/// Append one audit row, recomputing the chain head for the row's
/// `sender_id`. Returns the newly inserted row so callers can surface the
/// hash chain in responses if they want to.
pub(crate) async fn append_event<C: ConnectionTrait>(
    db: &C,
    event: NewAuditEvent,
) -> AppResult<share_events::Model> {
    let prev_hash = previous_hash(db, event.sender_id).await?;
    let this_hash = chain_hash(&prev_hash, &event)?;

    // UUID v7 sorts by creation time, so the `ORDER BY id DESC` tiebreak
    // in `previous_hash` resolves correctly when two rows share the same
    // `created_at` second.
    let id = Uuid::now_v7();
    let active = share_events::ActiveModel {
        id: ActiveValue::Set(id),
        sender_id: ActiveValue::Set(event.sender_id),
        recipient_id: ActiveValue::Set(event.recipient_id),
        file_id: ActiveValue::Set(event.file_id),
        action: ActiveValue::Set(event.action_str.to_string()),
        share_role_before: ActiveValue::Set(event.share_role_before.map(str::to_string)),
        share_role_after: ActiveValue::Set(event.share_role_after.map(str::to_string)),
        created_at: ActiveValue::Set(event.created_at),
        prev_event_hash: ActiveValue::Set(if prev_hash == zero_hash() {
            None
        } else {
            Some(prev_hash.to_vec())
        }),
        this_event_hash: ActiveValue::Set(this_hash.to_vec()),
        sender_signature: ActiveValue::Set(decode_signature(event.event_signature.as_deref())?),
    };

    share_events::Entity::insert(active)
        .exec_without_returning(db)
        .await?;

    let row = share_events::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| Error::InternalError("share_events_insert_lost".to_string()))?;

    Ok(row)
}

fn decode_signature(s: Option<&str>) -> AppResult<Option<Vec<u8>>> {
    match s {
        Some(value) => {
            let bytes = cryptfns::base64::decode(value)?;
            Ok(Some(bytes))
        }
        None => Ok(None),
    }
}

async fn previous_hash<C: ConnectionTrait>(
    db: &C,
    sender_id: Option<Uuid>,
) -> AppResult<[u8; PREV_HASH_LEN]> {
    let mut query = share_events::Entity::find();
    query = match sender_id {
        Some(id) => query.filter(share_events::Column::SenderId.eq(id)),
        None => query.filter(share_events::Column::SenderId.is_null()),
    };

    let latest = query
        .order_by(share_events::Column::CreatedAt, Order::Desc)
        .order_by(share_events::Column::Id, Order::Desc)
        .one(db)
        .await?;

    match latest {
        Some(row) => {
            let mut out = [0u8; PREV_HASH_LEN];
            if row.this_event_hash.len() != PREV_HASH_LEN {
                return Err(Error::InternalError(format!(
                    "share_events.this_event_hash bad length: {}",
                    row.this_event_hash.len()
                )));
            }
            out.copy_from_slice(&row.this_event_hash);
            Ok(out)
        }
        None => Ok(zero_hash()),
    }
}

fn zero_hash() -> [u8; PREV_HASH_LEN] {
    [0u8; PREV_HASH_LEN]
}

fn chain_hash(
    prev_hash: &[u8; PREV_HASH_LEN],
    event: &NewAuditEvent,
) -> AppResult<[u8; PREV_HASH_LEN]> {
    let row = AuditEventRowV1 {
        sender_id: uuid_to_bytes(event.sender_id),
        recipient_id: uuid_to_bytes(event.recipient_id),
        file_id: event.file_id.into_bytes(),
        action: event.action_str.to_string(),
        share_role: event.share_role_after.and_then(role_str_to_enum),
        created_at: event.created_at,
    };
    let der = encode_audit_event_v1(&row).map_err(|e| Error::CryptoError(Box::new(e)))?;

    let mut hasher = Sha256::new();
    hasher.update(AUDIT_EVENT_V1_PREFIX);
    hasher.update(prev_hash);
    hasher.update(&der);
    let digest = hasher.finalize();

    let mut out = [0u8; PREV_HASH_LEN];
    out.copy_from_slice(&digest);
    Ok(out)
}

fn uuid_to_bytes(id: Option<Uuid>) -> [u8; 16] {
    id.map(|u| u.into_bytes()).unwrap_or([0u8; 16])
}

fn role_str_to_enum(role: &'static str) -> Option<ShareRoleEnum> {
    match role {
        "reader" => Some(ShareRoleEnum::Reader),
        "editor" => Some(ShareRoleEnum::Editor),
        "co-owner" => Some(ShareRoleEnum::CoOwner),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> NewAuditEvent {
        NewAuditEvent {
            sender_id: Some(Uuid::nil()),
            recipient_id: Some(Uuid::nil()),
            file_id: Uuid::nil(),
            action_str: "grant",
            share_role_before: None,
            share_role_after: Some("editor"),
            created_at: 1_700_000_000,
            event_signature: None,
        }
    }

    #[test]
    fn chain_hash_is_deterministic() {
        let prev = [7u8; PREV_HASH_LEN];
        let a = chain_hash(&prev, &fixture()).unwrap();
        let b = chain_hash(&prev, &fixture()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn chain_hash_changes_with_prev() {
        let a = chain_hash(&[0u8; PREV_HASH_LEN], &fixture()).unwrap();
        let b = chain_hash(&[1u8; PREV_HASH_LEN], &fixture()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn chain_hash_changes_with_role() {
        let prev = [0u8; PREV_HASH_LEN];
        let mut other = fixture();
        other.share_role_after = Some("reader");
        assert_ne!(
            chain_hash(&prev, &fixture()).unwrap(),
            chain_hash(&prev, &other).unwrap()
        );
    }
}
