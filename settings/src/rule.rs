use serde::{Deserialize, Serialize};
use std::{fmt::Display, sync::Arc};

pub trait RuleValidate {
    /// Validate input against rule pattern.
    /// Should return true if input is valid.
    fn valid(&self, input: &str) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pattern: Arc<str>,
    test_start: bool,
    test_end: bool,
    test_contains: bool,
}

impl Rule {
    /// Create new whitelist rule.
    ///
    /// Whitelist rules are used to valid email addresses.
    ///
    /// # Examples
    /// ```rust
    /// use settings::rule::Rule;
    /// use settings::rule::RuleValidate as _;
    ///
    /// let rule = Rule::new("*@example.com".into());
    ///
    /// assert_eq!(rule.valid("test@example.com"), true);
    /// assert_eq!(rule.valid("test@example.net"), false);
    /// assert_eq!(rule.valid("else@example.org"), false);
    ///
    /// let rule = Rule::new("test@example.*".into());
    ///
    /// assert_eq!(rule.valid("test@example.com"), true);
    /// assert_eq!(rule.valid("test@example.net"), true);
    /// assert_eq!(rule.valid("else@example.org"), false);
    ///
    /// let rule = Rule::new("*example*".into());
    ///
    /// assert_eq!(rule.valid("test@example.com"), true);
    /// assert_eq!(rule.valid("test@example.net"), true);
    /// assert_eq!(rule.valid("else@example.org"), true);
    /// ```
    pub fn new(pattern: &str) -> Self {
        let test_start = pattern.starts_with('*');
        let test_end = pattern.ends_with('*');

        let pattern = pattern.replace('*', "");

        Self {
            pattern: pattern.into(),
            test_start: test_start && !test_end,
            test_end: !test_start && test_end,
            test_contains: test_start && test_end,
        }
    }

    /// Get domain pattern.
    fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Check if email ends with domain pattern.
    fn ends_with(&self, email: &str) -> bool {
        email.ends_with(self.pattern())
    }

    /// Check if email starts with domain pattern.
    fn starts_with(&self, email: &str) -> bool {
        email.starts_with(self.pattern())
    }

    /// Check if email contains domain pattern.
    fn contains(&self, email: &str) -> bool {
        email.contains(self.pattern())
    }

    /// Check if email is an exact match to domain pattern.
    fn exact_match(&self, email: &str) -> bool {
        email == self.pattern()
    }
}

impl RuleValidate for Rule {
    fn valid(&self, input: &str) -> bool {
        if self.test_start {
            return self.ends_with(input);
        }

        if self.test_end {
            return self.starts_with(input);
        }

        if self.test_contains {
            return self.contains(input);
        }

        self.exact_match(input)
    }
}

impl Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.test_start {
            return write!(f, "*{}", self.pattern);
        }

        if self.test_end {
            return write!(f, "{}*", self.pattern);
        }

        if self.test_contains {
            return write!(f, "*{}*", self.pattern);
        }

        write!(f, "{}", self.pattern)
    }
}

impl Serialize for Rule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for Rule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        Ok(Self::new(String::deserialize(deserializer)?.as_str()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_serialize_and_deserialize() {
        let rule = Rule::new("*@example.com".into());
        assert_eq!(rule.pattern(), "@example.com");

        let serialized = serde_json::to_string(&rule).unwrap();
        let deserialized: Rule = serde_json::from_str(&serialized).unwrap();

        assert_eq!(rule, deserialized);
    }

    #[test]
    fn test_starts_with() {
        let rule = Rule::new("*@example.com".into());
        assert_eq!(rule.pattern(), "@example.com");

        assert!(rule.test_start);
        assert!(rule.valid("test@example.com"));

        assert!(!rule.test_end);
        assert!(!rule.valid("test@example.net"));

        assert!(!rule.test_contains);
        assert!(!rule.valid("else@example.org"));
    }

    #[test]
    fn test_ends_with() {
        let rule = Rule::new("test@example.*".into());
        assert_eq!(rule.pattern(), "test@example.");

        assert!(!rule.test_start);
        assert!(rule.valid("test@example.com"));

        assert!(rule.test_end);
        assert!(rule.valid("test@example.net"));

        assert!(!rule.test_contains);
        assert!(!rule.valid("else@example.org"));
    }

    #[test]
    fn test_contains() {
        let rule = Rule::new("*@example.*".into());

        assert!(!rule.test_start);
        assert!(rule.valid("test@example.com"));

        assert!(!rule.test_end);
        assert!(rule.valid("test@example.net"));

        assert!(rule.test_contains);
        assert!(rule.valid("else@example.org"));
    }

    #[test]
    fn test_exact_match() {
        let rule = Rule::new("else@example.org".into());

        assert!(!rule.test_start);
        assert!(!rule.valid("test@example.com"));

        assert!(!rule.test_end);
        assert!(!rule.valid("test@example.net"));

        assert!(!rule.test_contains);
        assert!(rule.valid("else@example.org"));
    }
}
