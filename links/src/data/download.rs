use ::error::AppResult;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Download {
    pub link_key: Option<String>,
}

impl Validation for Download {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(link_key)]
    }
}

impl Download {
    pub fn into_value(self) -> AppResult<Vec<u8>> {
        let data = self.validate()?;

        Ok(cryptfns::hex::decode(data.link_key.unwrap())?)
    }
}
