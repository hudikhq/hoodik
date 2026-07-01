use entity::{user_files, users, DbErr, FromQueryResult, QueryResult, Uuid};
use serde::{Deserialize, Serialize};

/// Server-side view of a single non-owner row on `user_files`. Returned by
/// `GET /api/shares/{file_id}` and as the per-entry response from
/// `POST /api/shares`. `shared_by_email` is `None` for owner-issued grants
/// (`shared_by_user_id` itself stays as the granter's UUID); the field is
/// populated only when the joined sender row was found.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppShare {
    pub file_id: Uuid,
    pub recipient_id: Uuid,
    pub recipient_email: String,
    pub recipient_pubkey_fingerprint: String,
    pub share_role: String,
    pub created_at: i64,
    pub shared_at: Option<i64>,
    pub shared_by_user_id: Option<Uuid>,
    pub shared_by_email: Option<String>,
}

impl FromQueryResult for AppShare {
    fn from_query_result(res: &QueryResult, _pre: &str) -> Result<Self, DbErr> {
        let row = user_files::Model::from_query_result(res, "uf")?;
        let recipient = users::Model::from_query_result(res, "recipient")?;
        let shared_by_email: Option<String> = res.try_get("sender", "email").ok();

        Ok(Self {
            file_id: row.file_id,
            recipient_id: recipient.id,
            recipient_email: recipient.email,
            recipient_pubkey_fingerprint: recipient.fingerprint,
            share_role: row.share_role,
            created_at: row.created_at,
            shared_at: row.shared_at,
            shared_by_user_id: row.shared_by_user_id,
            shared_by_email,
        })
    }
}
