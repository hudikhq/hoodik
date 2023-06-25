use super::session::Session;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Paginated {
    pub sessions: Vec<Session>,
    pub total: u64,
}
