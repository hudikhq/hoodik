use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Update {
    pub role: Option<String>,
}

impl Validation for Update {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_in!(
            role,
            vec!["admin".to_string(), "user".to_string()]
        )]
    }
}
