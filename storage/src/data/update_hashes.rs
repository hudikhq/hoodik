use ::error::AppResult;
use entity::{files::ActiveModel as ActiveModelFile, ActiveValue, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateHashes {
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub blake2b: Option<String>,
}

impl Validation for UpdateHashes {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            // SHA-256 is the integrity anchor; optional hashes may be omitted when the client
            // skipped computing them (e.g. performance experiments).
            rule_required!(sha256),
        ]
    }
}

impl UpdateHashes {
    pub fn into_active_model(self, id: Uuid) -> AppResult<ActiveModelFile> {
        let data = self.validate()?;

        Ok(ActiveModelFile {
            id: ActiveValue::Set(id),
            md5: match data.md5 {
                Some(v) => ActiveValue::Set(Some(v)),
                None => ActiveValue::NotSet,
            },
            sha1: match data.sha1 {
                Some(v) => ActiveValue::Set(Some(v)),
                None => ActiveValue::NotSet,
            },
            sha256: ActiveValue::Set(data.sha256),
            blake2b: match data.blake2b {
                Some(v) => ActiveValue::Set(Some(v)),
                None => ActiveValue::NotSet,
            },
            ..Default::default()
        })
    }
}
