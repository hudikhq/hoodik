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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharingCapabilities {
    pub enabled: bool,
    pub roles: Vec<String>,
}

impl Capabilities {
    pub fn for_enabled(enabled: bool) -> Self {
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
        }
    }
}
