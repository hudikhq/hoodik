pub mod file_tokens;
pub mod files;
pub mod invitations;
pub mod links;
pub mod paginated;
pub mod prelude;
pub mod sessions;
pub mod tokens;
pub mod user_actions;
pub mod user_files;
pub mod users;

pub mod join;
pub mod sort;

pub mod numeric;

pub use prelude::*;

pub use sea_orm::prelude::BigDecimal;
pub use sea_orm::{
    entity::prelude::Uuid,
    entity::{ActiveModelTrait, ColumnTrait, EntityTrait, RelationTrait},
    sea_query::{
        Alias, Expr, IntoCondition, Query, SelectStatement, SimpleExpr, SubQueryOper,
        SubQueryStatement, UnionType,
    },
    ActiveValue, Condition, ConnectionTrait, DbBackend, DbConn, DbErr, EntityOrSelect,
    FromQueryResult, Identity, JoinType, JsonValue, ModelTrait, Order, PaginatorTrait, QueryFilter,
    QueryOrder, QueryResult, QuerySelect, QueryTrait, Select, SelectTwo, Statement,
    TransactionTrait, TryGetableMany, Value,
};

/// Helper to convert `Option<String>` to `Option<Uuid>`
pub fn option_string_to_uuid(i: Option<String>) -> Option<Uuid> {
    match i {
        Some(s) => Uuid::parse_str(s.as_str()).ok(),
        None => None,
    }
}

/// Convert `ActiveValue` to `Option<Uuid>`
pub fn active_value_to_uuid(i: ActiveValue<Uuid>) -> Option<Uuid> {
    value_to_uuid(i.into_value()?)
}

/// Helper to convert `Value` to `Option<Uuid>`
pub fn value_to_uuid<T: Into<Value>>(i: T) -> Option<Uuid> {
    let v = i.into();

    match v {
        Value::Uuid(u) => u.map(|u| *u),
        _ => None,
    }
}

#[cfg(feature = "mock")]
pub use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult, Transaction};

#[cfg(feature = "mock")]
pub mod mock;
