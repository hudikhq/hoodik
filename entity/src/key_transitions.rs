use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "key_transitions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    #[sea_orm(unique)]
    pub old_fingerprint: String,
    pub old_key_spki: Vec<u8>,
    pub old_key_type: String,
    pub new_fingerprint: String,
    /// PEM of the identity and wrapping keys this hop rotated *to*, stored so a
    /// verifier re-encodes the transition canonical for any hop — not only the
    /// most recent one, whose new keys still happen to be the account's live
    /// `users.pubkey`/`users.wrapping_pubkey`. Without them an intermediate hop
    /// in a multi-rotation chain is unverifiable, because a rotation's wrapping
    /// key is generated independently (and is a hybrid X25519+ML-KEM container,
    /// not derivable from the identity key).
    pub new_identity_key_pem: String,
    pub new_wrapping_key_pem: String,
    pub old_signature: Vec<u8>,
    pub new_signature: Vec<u8>,
    pub issued_at: i64,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Users,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
