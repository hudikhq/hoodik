use crate::data::files::stats::Stats;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Response {
    /// The user.
    pub user: entity::users::Model,

    /// Stats for the user's files.
    pub stats: Vec<Stats>,
}
