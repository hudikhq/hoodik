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
    /// Tells us what checksum function was used to generate the checksum
    pub checksum_function: Option<String>,
    /// Secret key to encrypt the data when before saving it
    /// this is optional and it can be done on the client
    /// side so the key is never sent to the backend.
    ///
    /// But in some cases, it might be more efficient to do it
    /// on the backend, even if it is less secure.
    ///
    /// Obviously, if the data is already encrypted it will be
    /// encrypted again because we don't check on the backend
    /// if it was encrypted, so be warned..
    pub key_hex: Option<String>,
}

impl Validation for Meta {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(chunk),
            rule_required!(checksum),
            Rule::new("chunk", |obj: &Self, error| {
                if let Some(v) = obj.chunk {
                    if v < 0 {
                        error.add("min:0")
                    }
                } else {
                    error.add("required")
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![]
    }
}

impl Meta {
    pub fn into_tuple(self) -> AppResult<(i32, String, String, Option<String>)> {
        let data = self.validate()?;

        Ok((
            data.chunk.unwrap(),
            data.checksum.unwrap_or_default(),
            data.checksum_function
                .unwrap_or_else(|| "sha256".to_string()),
            data.key_hex,
        ))
    }
}
