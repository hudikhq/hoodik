use ::error::AppResult;
use serde::Deserialize;
use validr::*;

#[derive(Clone, Debug, Deserialize)]
pub struct Update {
    pub expires_at: Option<i64>,
}

impl Validation for Update {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![]
    }
}

impl Update {
    pub fn into_value(self) -> AppResult<Option<i64>> {
        let data = self.validate()?;

        Ok(data.expires_at)
    }
}
