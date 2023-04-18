use serde::{Deserialize, Serialize};

use super::app_file::AppFile;

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    /// List of directory we are in and all of the ones before up until root
    pub parents: Vec<AppFile>,

    /// List of files in the current (last) directory in the list above
    pub children: Vec<AppFile>,
}
