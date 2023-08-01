use entity::{numeric::Numeric, DbErr, FromQueryResult, QueryResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub mime: String,
    pub size: i64,
    pub count: i64,
}

impl FromQueryResult for Stats {
    fn from_query_result(res: &QueryResult, _pre: &str) -> Result<Self, DbErr> {
        let mime = res.try_get_by("mime")?;
        let size: Numeric = res.try_get_by("size")?;
        let count = res.try_get_by("count")?;

        Ok(Self {
            mime,
            size: size.into(),
            count,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub stats: Vec<Stats>,
    pub used_space: i64,
    pub quota: Option<u64>,
}
