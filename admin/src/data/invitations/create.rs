use ::error::AppResult;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Create {
    pub email: Option<String>,
    pub message: Option<String>,
    pub expires_at: Option<String>,
}

impl Validation for Create {
    fn rules(&self) -> Vec<validr::Rule<Self>> {
        vec![
            rule_required!(email),
            rule_email!(email),
            Rule::new("expires_at", |obj: &Self, error| {
                if let Some(v) = obj.expires_at.as_deref() {
                    if util::datetime::parse_into_naive_datetime(v, Some("expires_at")).is_err() {
                        error.add("invalid_date")
                    }
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![Modifier::new("email", |obj: &mut Self| {
            if let Some(s) = obj.email.as_deref() {
                obj.email = Some(s.to_lowercase());
            }
        })]
    }
}

impl Create {
    pub fn into_values(self) -> AppResult<(String, Option<String>, i64)> {
        let data = self.validate()?;

        let expires_at = match data.expires_at.as_deref() {
            Some(v) => util::datetime::parse_into_naive_datetime(v, Some("expires_at"))?,
            None => Utc::now().naive_utc() + chrono::Duration::days(7),
        };

        Ok((data.email.unwrap(), data.message, expires_at.timestamp()))
    }
}
