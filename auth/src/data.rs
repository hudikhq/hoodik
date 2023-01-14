use chrono::Utc;
use serde::{Deserialize, Serialize};
use store::{user::ActiveModel, ActiveValue};
use util::{
    password::hash,
    validation::{validate_otp, validate_password},
};
use validr::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub email: Option<String>,
    pub password: Option<String>,
    pub secret: Option<String>,
    pub token: Option<String>,
}

impl Validation for CreateUser {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(email),
            rule_email!(email),
            rule_required!(password),
            Rule::new("password", |obj: &Self, error| {
                if let Some(v) = &obj.password {
                    if validate_password(v) {
                        error.add("weak_password");
                    }
                }
            }),
            Rule::new("secret", |obj: &Self, error| {
                if let Some(v) = &obj.secret {
                    if validate_otp(v, obj.token.as_ref()) {
                        error.add("invalid_otp_token");
                    }
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![modifier_trim!(email), modifier_lowercase!(email)]
    }
}

impl CreateUser {
    pub fn into_active_model(self) -> ActiveModel {
        ActiveModel {
            id: ActiveValue::NotSet,
            email: ActiveValue::Set(self.email.unwrap()),
            password: ActiveValue::Set(hash(&self.password.unwrap())),
            secret: self
                .secret
                .map(ActiveValue::Set)
                .unwrap_or_else(|| ActiveValue::NotSet),
            created_at: ActiveValue::Set(Utc::now().naive_utc()),
            updated_at: ActiveValue::Set(Utc::now().naive_utc()),
        }
    }
}
