use crate::rule::{Rule, RuleValidate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Blacklist {
    rules: Vec<Rule>,
}

impl RuleValidate for Blacklist {
    /// Check if the email matches any of the blacklist rules.
    fn valid(&self, input: &str) -> bool {
        !self.rules.iter().any(|rule| rule.valid(input))
    }
}
