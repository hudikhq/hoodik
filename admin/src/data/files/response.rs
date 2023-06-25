use crate::data::files::stats::Stats;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Response {
    /// Available space in bytes to non-privileged users in the file system containing the provided path.
    pub available_space: u64,

    /// Stats about all the file types hosted on the platform for all the users
    pub stats: Vec<Stats>,
}
