use entity::{sessions, users, DbErr, FromQueryResult, QueryResult, Uuid};
use serde::Serialize;

use crate::data::sessions::session::Session;

#[derive(Debug, Serialize)]
pub struct User {
    pub id: Uuid,
    pub role: Option<String>,
    pub email: String,
    pub secret: bool,
    pub quota: Option<i64>,
    pub pubkey: String,
    pub fingerprint: String,
    pub email_verified_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_session: Option<Session>,
}

impl FromQueryResult for User {
    fn from_query_result(res: &QueryResult, _pre: &str) -> Result<Self, DbErr> {
        let user = users::Model::from_query_result(res, "user")?;
        let session = sessions::Model::from_query_result(res, "session").ok();

        let last_session = session.map(|session| Session::from((&user, &session)));

        Ok(Self {
            id: user.id,
            role: user.role,
            email: user.email,
            secret: user.secret.is_some(),
            quota: user.quota,
            pubkey: user.pubkey,
            fingerprint: user.fingerprint,
            email_verified_at: user.email_verified_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_session,
        })
    }
}
