use serde::{Deserialize, Serialize};

/// Admin kill switch for the sharing surface. Default is enabled;
/// flipping to `false` makes every `/api/shares/...`
/// endpoint return `503 feature_disabled` and the public
/// `/api/capabilities` advertise `sharing.enabled = false` so clients
/// hide the UI fail-closed. Existing share rows are preserved across
/// toggles — the switch hides UI, it does not drop data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sharing {
    enabled: bool,
}

impl Default for Sharing {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Sharing {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
