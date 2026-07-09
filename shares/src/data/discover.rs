use entity::Uuid;
use serde::{Deserialize, Serialize};

/// Query parameter shape for `GET /api/users/discover`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiscoverQuery {
    pub email: Option<String>,
}

/// Response body — never include `encrypted_private_key`, `role`,
/// `quota`, or any flag beyond what's needed to compute a fingerprint and
/// wrap a key.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredUser {
    pub user_id: Uuid,
    pub email: String,
    pub pubkey: String,
    pub key_type: String,
    pub wrapping_pubkey: Option<String>,
    pub fingerprint: String,
}
