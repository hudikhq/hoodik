//! # Transfer token request and response data
use entity::Uuid;
use ::error::AppResult;
use serde::{Deserialize, Serialize};
use validr::*;

/// Request body for creating a transfer token.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateTransferToken {
    pub file_id: Option<Uuid>,
    pub action: Option<String>,
}

impl Validation for CreateTransferToken {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(file_id),
            rule_required!(action),
            rule_in!(
                action,
                Into::<Vec<String>>::into(["upload".to_string(), "download".to_string()])
            ),
        ]
    }
}

impl CreateTransferToken {
    pub fn into_tuple(self) -> AppResult<(Uuid, String)> {
        let data = self.validate()?;
        Ok((data.file_id.unwrap(), data.action.unwrap()))
    }
}

/// Response body for a created transfer token.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransferTokenResponse {
    pub token: String,
    pub expires_at: i64,
    pub file_id: Uuid,
    pub action: String,
}
