use ::error::AppResult;
use chrono::Utc;
use entity::{links::ActiveModel, ActiveValue, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLink {
    /// Id of the file that will be shared.
    pub file_id: Option<String>,

    /// Signature that the user created when the link was initially created.
    pub signature: Option<String>,

    /// Name of the file encrypted with the link key.
    pub encrypted_name: Option<String>,

    /// Link AES key encrypted with the user's public RSA key.
    pub encrypted_link_key: Option<String>,

    /// If the file has a thumbnail it is encrypted with the link key.
    pub encrypted_thumbnail: Option<String>,

    /// AES key for the file encrypted with a link AES key.
    pub encrypted_file_key: Option<String>,

    /// Optional date when the link will expire.
    pub expires_at: Option<String>,
}

impl Validation for CreateLink {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(file_id),
            rule_required!(encrypted_name),
            rule_required!(encrypted_link_key),
            rule_required!(encrypted_file_key),
            Rule::new("expires_at", |obj: &Self, error| {
                if let Some(v) = obj.expires_at.as_deref() {
                    if util::datetime::parse_into_naive_datetime(v, Some("expires_at")).is_err() {
                        error.add("invalid_date")
                    }
                }
            }),
        ]
    }
}

impl CreateLink {
    pub fn into_active_model(self, user_id: Uuid) -> AppResult<(ActiveModel, String, Uuid)> {
        let data = self.validate()?;

        let expires_at = match data.expires_at.as_deref() {
            Some(v) => Some(util::datetime::parse_into_naive_datetime(
                v,
                Some("expires_at"),
            )?),
            None => None,
        };

        let file_id = match data.file_id.as_deref() {
            Some(v) => Uuid::parse_str(v)?,
            None => {
                return Err("file_id is required".into());
            }
        };

        Ok((
            ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                user_id: ActiveValue::Set(user_id),
                file_id: ActiveValue::Set(file_id),
                signature: ActiveValue::Set(data.signature.clone().unwrap()),
                downloads: ActiveValue::Set(0),
                encrypted_name: ActiveValue::Set(data.encrypted_name.unwrap()),
                encrypted_link_key: ActiveValue::Set(data.encrypted_link_key.unwrap()),
                encrypted_thumbnail: ActiveValue::Set(data.encrypted_thumbnail),
                encrypted_file_key: ActiveValue::Set(data.encrypted_file_key),
                created_at: ActiveValue::Set(Utc::now().naive_utc()),
                expires_at: ActiveValue::Set(expires_at),
            },
            data.signature.unwrap(),
            file_id,
        ))
    }
}
