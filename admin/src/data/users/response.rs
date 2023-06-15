use crate::data::files::stats::Stats;
use serde::Serialize;

use super::user::User;

#[derive(Debug, Serialize)]
pub struct Response {
    /// The user.
    pub user: User,

    /// Stats for the user's files.
    pub stats: Vec<Stats>,
}
