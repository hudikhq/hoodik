//! Locales the server can render outbound email in. Clients offer the same
//! set; the API rejects anything else so a stored locale always has email
//! templates behind it.

pub const SUPPORTED: [&str; 4] = ["en", "fr", "de", "hr"];

pub fn is_supported(locale: &str) -> bool {
    SUPPORTED.contains(&locale)
}

/// Resolve a stored user locale to one the email templates cover,
/// defaulting to English for unset or stale values.
pub fn resolve(locale: Option<&str>) -> &'static str {
    match locale {
        Some(l) => SUPPORTED
            .iter()
            .find(|s| **s == l)
            .copied()
            .unwrap_or("en"),
        None => "en",
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn resolves_supported_and_defaults_unknown() {
        assert_eq!(resolve(Some("hr")), "hr");
        assert_eq!(resolve(Some("xx")), "en");
        assert_eq!(resolve(None), "en");
        assert!(is_supported("fr"));
        assert!(!is_supported("pt"));
    }
}
