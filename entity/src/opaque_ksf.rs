use sea_orm::entity::prelude::*;

/// Model over the OPAQUE key-stretching-function columns of the `users` table.
/// It is a second entity on the same table rather than fields on `users`
/// because those columns are read only during login start and written only at
/// migration; folding them into `users` would force every exhaustive
/// `users::ActiveModel` literal across the codebase to change. `login/start`
/// reads a migrated account's stored parameters through this model so the
/// client runs the matching KSF; migration writes the ones it used.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub ksf_algorithm: String,
    pub ksf_m_cost: i32,
    pub ksf_t_cost: i32,
    pub ksf_p_cost: i32,
    pub opaque_protocol_version: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
