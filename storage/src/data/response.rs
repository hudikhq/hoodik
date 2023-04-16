use serde::{Deserialize, Serialize};

use super::app_file::AppFile;

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub parent: Option<AppFile>,
    pub dir: Option<AppFile>,
    pub children: Vec<AppFile>,
}
