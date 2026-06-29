use entity::Uuid;

/// All fields required to append a single `share_events` row through the
/// per-sender hash chain. `event_signature` is `None` for system-cascade
/// rows (account-deletion fan-out, Co-owner revoke cascade) where there
/// is no human actor whose privkey could sign.
#[derive(Debug, Clone)]
pub(crate) struct NewAuditEvent {
    pub sender_id: Option<Uuid>,
    pub recipient_id: Option<Uuid>,
    pub file_id: Uuid,
    pub action_str: &'static str,
    pub share_role_before: Option<&'static str>,
    pub share_role_after: Option<&'static str>,
    pub created_at: i64,
    /// Already-verified base64 signature. Stored verbatim on the row.
    pub event_signature: Option<String>,
}
