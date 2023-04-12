//! Resumable upload struct that holds the database file information
//! and the owner record that holds the encrypted key for the file.
//!
//! Key is encrypted with users public key and it contains the AES key which was
//! used for actually encrypting the file data.

use ::error::AppResult;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meta {
    /// The chunk number that is being uploaded
    /// this is used for resumable uploads
    pub chunk: Option<i32>,
    /// Checksum of the currently uploading chunk
    /// this is used for verifying the integrity of the chunk
    pub checksum: Option<String>,
}

impl Validation for Meta {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(chunk),
            rule_required!(checksum),
            Rule::new("chunk", |obj: &Self, error| {
                if let Some(v) = obj.chunk {
                    if v < 0 {
                        error.add("min:0");
                    }
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![]
    }
}

impl Meta {
    pub fn into_tuple(self) -> AppResult<(i32, String)> {
        let data = self.validate()?;

        Ok((data.chunk.unwrap(), data.checksum.unwrap()))
    }
}
