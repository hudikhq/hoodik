use serde::{Deserialize, Serialize};

/// Optional POST body for a public-link content download. `link_key` is
/// accepted for back-compat with older clients but is never used — the
/// recipient decrypts client-side with the fragment key, so the server only
/// ever streams ciphertext.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Download {
    pub link_key: Option<String>,
}
