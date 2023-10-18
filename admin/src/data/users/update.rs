use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Update {
    pub role: Option<String>,
    pub quota: Option<i64>,
}

impl Validation for Update {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_in!(
            role,
            Into::<Vec<String>>::into(["admin".to_string(), "user".to_string()])
        )]
    }
}
