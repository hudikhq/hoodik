use entity::{sessions, users};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Authenticated {
    pub user: users::Model,
    pub session: sessions::Model,
}
