use ::error::AppResult;
use serde::{Deserialize, Serialize};
use util::validation::validate_password;
use validr::*;

/// Change password data
#[derive(Clone, Serialize, Deserialize)]
pub struct ChangePassword {
    /// Email of the user that is trying to change its password
    pub email: Option<String>,

    /// Two factor token if the user has tfa enabled
    pub token: Option<String>,

    /// New password for the user
    pub password: Option<String>,

    /// Signature of the current plaintext password with the users private key
    /// If the current_password is missing for the user, this is required to be present
    /// and it is used as proof that the user actually has access to the private key
    pub signature: Option<String>,

    /// Current password of the user if the user didn't provide signature
    /// This is required to be present if the user didn't provide signature
    /// because its the other way for the user to gain access to the private key
    /// and prove that he is the owner of the account.
    pub current_password: Option<String>,

    /// Private key encrypted again with the new password (this is required to be the same as the old one)
    pub encrypted_private_key: Option<String>,
}

impl Validation for ChangePassword {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(email),
            rule_email!(email),
            rule_required!(password),
            rule_required!(encrypted_private_key),
            Rule::new("password", |obj: &Self, error| {
                if let Some(v) = &obj.password {
                    if !validate_password(v) {
                        error.add("weak_password");
                    }
                }
            }),
            Rule::new("signature", |obj: &Self, error| {
                if obj.current_password.is_none() && obj.signature.is_none() {
                    error.add("required");
                }
            }),
            Rule::new("current_password", |obj: &Self, error| {
                if obj.signature.is_none() && obj.current_password.is_none() {
                    error.add("required");
                }
            }),
        ]
    }
}

pub(crate) type ChangePasswordData = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

impl ChangePassword {
    pub fn into_data(self) -> AppResult<ChangePasswordData> {
        let data = self.validate()?;

        Ok((
            data.email.unwrap(),
            data.password.unwrap(),
            data.encrypted_private_key.unwrap(),
            data.current_password,
            data.signature,
            data.token,
        ))
    }
}
