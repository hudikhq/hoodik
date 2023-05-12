use ::error::AppResult;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Signature {
    pub fingerprint: Option<String>,
    pub signature: Option<String>,
}

impl Validation for Signature {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(fingerprint), rule_required!(signature)]
    }
}

impl Signature {
    pub fn into_tuple(&self) -> AppResult<(String, String)> {
        let data = self.clone().validate()?;

        Ok((data.fingerprint.unwrap(), data.signature.unwrap()))
    }
}
