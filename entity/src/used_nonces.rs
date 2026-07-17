use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Signature-login nonces that have already authenticated, keeping a captured
/// login request from being replayed. A nonce is only accepted while its
/// signed timestamp is fresh (or, for legacy clients, within its minute
/// bucket), so rows are dead once that window closes and are purged on the
/// next login.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "used_nonces")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub fingerprint: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub nonce: String,
    pub expires_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
