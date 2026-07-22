use serde::Deserialize;
use validr::*;

#[derive(Clone, Debug, Deserialize)]
pub struct Find {
    pub with_expired: Option<bool>,
    /// Withhold `encrypted_thumbnail` from the rows and report only
    /// `has_thumbnail`. Absent means full rows — the compatible default
    /// for older clients.
    pub compact: Option<bool>,
}

impl Validation for Find {
    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![Modifier::new("with_expired", |obj: &mut Self| {
            if obj.with_expired.is_none() {
                obj.with_expired = Some(false);
            }
        })]
    }
}
