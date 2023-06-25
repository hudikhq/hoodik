use ::error::AppResult;
use serde::Deserialize;
use validr::*;

#[derive(Clone, Debug, Deserialize)]
pub struct Update {
    pub expires_at: Option<String>,
}

impl Validation for Update {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![Rule::new("expires_at", |obj: &Self, error| {
            if let Some(v) = obj.expires_at.as_deref() {
                if util::datetime::parse_into_naive_datetime(v, Some("expires_at")).is_err() {
                    error.add("invalid_date")
                }
            }
        })]
    }
}

impl Update {
    pub fn into_value(self) -> AppResult<Option<i64>> {
        let data = self.validate()?;

        let expires_at = match data.expires_at.as_deref() {
            Some(v) => {
                Some(util::datetime::parse_into_naive_datetime(v, Some("expires_at"))?.timestamp())
            }
            None => None,
        };

        Ok(expires_at)
    }
}
