//! # Authentication by credentials data
use ::error::AppResult;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResendActivation {
    pub email: Option<String>,
}

impl Validation for ResendActivation {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(email), rule_email!(email)]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![modifier_trim!(email), modifier_lowercase!(email)]
    }
}

impl ResendActivation {
    pub fn into_value(&self) -> AppResult<String> {
        let data = self.clone().validate()?;

        Ok(data.email.unwrap())
    }
}
