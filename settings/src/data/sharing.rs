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
    /// Cipher identifier clients use to encrypt new files, advertised through
    /// `GET /api/capabilities`. Existing files are unaffected — each file
    /// decrypts with the cipher stored in its own `files.cipher` column.
    #[serde(default = "default_cipher")]
    default_cipher: String,
}

/// Settings files written before this field existed get the cipher that was
/// the hardcoded default at the time, so their behavior does not change.
fn default_cipher() -> String {
    "aegis128l".to_string()
}

impl Default for Sharing {
    fn default() -> Self {
        Self {
            enabled: true,
            default_cipher: default_cipher(),
        }
    }
}

impl Sharing {
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn default_cipher(&self) -> &str {
        &self.default_cipher
    }

    pub fn set_default_cipher(&mut self, cipher: String) {
        self.default_cipher = cipher;
    }
}

#[cfg(test)]
mod tests {
    use super::Sharing;

    #[test]
    fn settings_written_before_default_cipher_existed_keep_old_behavior() {
        let sharing: Sharing = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
        assert_eq!(sharing.default_cipher(), "aegis128l");
    }

    #[test]
    fn default_cipher_round_trips() {
        let mut sharing = Sharing::default();
        sharing.set_default_cipher("aegis256".to_string());
        let json = serde_json::to_string(&sharing).unwrap();
        let parsed: Sharing = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.default_cipher(), "aegis256");
    }
}
