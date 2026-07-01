mod blacklist;
mod sharing;
mod users;
mod whitelist;

pub use blacklist::Blacklist;
pub use sharing::Sharing;
pub use users::Users;
pub use whitelist::Whitelist;

use error::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Data {
    pub users: Users,
    #[serde(default)]
    pub sharing: Sharing,
}

impl Data {
    pub fn to_vec(&self) -> AppResult<Vec<u8>> {
        let data = serde_json::to_string_pretty(self)?;

        Ok(data.as_bytes().to_vec())
    }
}
