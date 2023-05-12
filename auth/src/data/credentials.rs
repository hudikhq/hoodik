use ::error::AppResult;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Credentials {
    pub email: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
}

impl Validation for Credentials {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(email),
            rule_email!(email),
            rule_required!(password),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![modifier_trim!(email), modifier_lowercase!(email)]
    }
}

impl Credentials {
    pub fn into_tuple(&self) -> AppResult<(String, String, Option<String>)> {
        let data = self.clone().validate()?;

        Ok((data.email.unwrap(), data.password.unwrap(), data.token))
    }
}
