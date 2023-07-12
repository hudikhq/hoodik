//! # Two Factor Data
use ::error::AppResult;
use serde::{Deserialize, Serialize};
use util::validation::validate_otp;
use validr::*;

#[derive(Serialize, Deserialize, Clone)]
pub struct Disable {
    pub token: Option<String>,
}

impl Validation for Disable {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(token)]
    }
}

impl Disable {
    pub fn into_value(&self) -> AppResult<Option<String>> {
        let data = self.clone().validate().unwrap();

        Ok(data.token)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Enable {
    pub token: Option<String>,
    pub secret: Option<String>,
}

impl Validation for Enable {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(token),
            rule_required!(secret),
            Rule::new("secret", |obj: &Self, error| {
                if let Some(v) = &obj.secret {
                    if !validate_otp(v, obj.token.as_ref()) {
                        error.add("invalid_otp_token");
                    }
                }
            }),
        ]
    }
}

impl Enable {
    pub fn into_value(&self) -> AppResult<Option<String>> {
        let data = self.clone().validate().unwrap();

        Ok(data.secret)
    }
}
