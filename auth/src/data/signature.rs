//! # Authentication by private key
use ::error::AppResult;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Signature {
    pub fingerprint: Option<String>,
    pub signature: Option<String>,
    /// Unix seconds signed into the login canonical together with `nonce`.
    /// Absent on clients predating the client-nonce scheme, which sign the
    /// deterministic minute bucket instead.
    pub timestamp: Option<i64>,
    /// Client-generated random nonce (lowercase hex) signed into the canonical.
    pub nonce: Option<String>,
}

impl Validation for Signature {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(fingerprint), rule_required!(signature)]
    }
}

impl Signature {
    pub fn into_tuple(&self) -> AppResult<(String, String)> {
        let data = self.clone().validate()?;

        Ok((data.fingerprint.unwrap(), data.signature.unwrap()))
    }
}
