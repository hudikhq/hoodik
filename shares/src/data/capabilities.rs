use serde::{Deserialize, Serialize};

/// Public capability advertisement returned by `GET /api/capabilities`.
/// Clients gate UI on `sharing.enabled` and fail closed on a missing or
/// erroring response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capabilities {
    pub sharing: SharingCapabilities,
    pub editable_folders: bool,
    pub share_groups: bool,
    pub audit_log: bool,
    pub fork: bool,
    pub default_cipher: String,

    /// Whether chunks can be read and written straight from the storage
    /// bucket instead of through this server.
    ///
    /// A property of how this instance is deployed, not of its version: two
    /// servers on the same release differ by environment and by whether
    /// their bucket answers a CORS preflight. Clients must read it here
    /// rather than inferring it from a version number, and treat its
    /// absence as `false` — which is what an older server, or an errored
    /// response, amounts to.
    #[serde(default)]
    pub direct_transfer: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingCapabilities {
    pub enabled: bool,
    pub roles: Vec<String>,
}

impl Capabilities {
    pub fn for_enabled(enabled: bool, default_cipher: String, direct_transfer: bool) -> Self {
        Self {
            sharing: SharingCapabilities {
                enabled,
                roles: vec![
                    "reader".to_string(),
                    "editor".to_string(),
                    "co-owner".to_string(),
                ],
            },
            editable_folders: true,
            share_groups: true,
            audit_log: true,
            fork: true,
            default_cipher,
            direct_transfer,
        }
    }
}
