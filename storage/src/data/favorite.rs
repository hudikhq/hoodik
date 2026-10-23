use ::error::AppResult;
use entity::Uuid;
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetFavorite {
    pub favorite: Option<bool>,
}

impl Validation for SetFavorite {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_required!(favorite)]
    }
}

impl SetFavorite {
    pub fn into_value(self) -> AppResult<bool> {
        let data = self.validate()?;

        Ok(data.favorite.unwrap_or_default())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FavoriteMany {
    pub ids: Option<Vec<Uuid>>,
    pub favorite: Option<bool>,
}

impl Validation for FavoriteMany {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            Rule::new("ids", |obj: &FavoriteMany, error| {
                if let Some(ids) = obj.ids.as_ref() {
                    if ids.is_empty() {
                        error.add("required")
                    }
                } else {
                    error.add("required")
                }
            }),
            rule_required!(favorite),
        ]
    }
}

impl FavoriteMany {
    pub fn into_value(self) -> AppResult<(Vec<Uuid>, bool)> {
        let data = self.validate()?;

        Ok((
            data.ids.unwrap_or_default(),
            data.favorite.unwrap_or_default(),
        ))
    }
}
