use crate::rule::{Rule, RuleValidate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Whitelist {
    rules: Vec<Rule>,
}

impl RuleValidate for Whitelist {
    fn valid(&self, input: &str) -> bool {
        self.rules.iter().any(|rule| rule.valid(input))
    }
}
