use entity::{sessions, users, DbErr, FromQueryResult, QueryResult, Uuid};
use serde::Serialize;

/// Struct wrapper around the session entity.
/// Session within itself holds some data that might
/// be used to extract users information in certain conditions,
/// so we are using this wrapper struct to only display the essential
/// data to the administrators and not the whole session entity.
#[derive(Debug, Clone, Serialize)]
pub struct Session {
    /// The session's ID.
    pub id: Uuid,

    /// The session's user ID.
    pub user_id: Uuid,

    /// The users's email.
    pub email: String,

    /// The session's IP address.
    pub ip: String,

    /// The session's user agent.
    pub user_agent: String,

    /// The session's created date.
    pub created_at: i64,

    /// The session's last updated date.
    pub updated_at: i64,

    /// The expiration datetime of the session
    pub expires_at: i64,

    /// The expiration datetime of the session
    pub deleted_at: Option<i64>,
}

impl FromQueryResult for Session {
    fn from_query_result(res: &QueryResult, _pre: &str) -> Result<Self, DbErr> {
        let user = users::Model::from_query_result(res, "user")?;
        let session = sessions::Model::from_query_result(res, "session")?;

        Ok(Self::from((&user, &session)))
    }
}

impl From<(&users::Model, &sessions::Model)> for Session {
    fn from(source: (&users::Model, &sessions::Model)) -> Session {
        let (user, session) = source;

        Self {
            id: session.id,
            user_id: session.user_id,
            email: user.email.clone(),
            ip: session.ip.clone(),
            user_agent: session.user_agent.clone(),
            created_at: session.created_at,
            updated_at: session.updated_at,
            expires_at: session.expires_at,
            deleted_at: session.deleted_at,
        }
    }
}
