use std::fmt::Display;

use num_traits::cast::{FromPrimitive, ToPrimitive};
use sea_orm::{prelude::BigDecimal, QueryResult, TryGetable};

/// Helper to handle numeric values that are incoming from the database
/// and can be in many forms (i32, i64, BigDecimal). This is used
/// to handle support for multiple different db backends.
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Numeric(f64);

impl From<Numeric> for f64 {
    fn from(n: Numeric) -> Self {
        n.0
    }
}

impl From<Numeric> for f32 {
    fn from(n: Numeric) -> Self {
        n.0 as f32
    }
}

impl From<Numeric> for i64 {
    fn from(n: Numeric) -> Self {
        n.0 as i64
    }
}

impl From<Numeric> for i32 {
    fn from(n: Numeric) -> Self {
        n.0 as i32
    }
}

impl From<Numeric> for BigDecimal {
    fn from(n: Numeric) -> Self {
        BigDecimal::from_f64(n.0).unwrap_or_default()
    }
}

impl Display for Numeric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.to_string().fmt(f)
    }
}

impl TryGetable for Numeric {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &QueryResult,
        idx: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        if let Ok(v) = <f64>::try_get_by(res, idx) {
            return Ok(Self(v));
        }

        if let Ok(v) = <i64>::try_get_by(res, idx) {
            return Ok(Self(v as f64));
        }
        if let Ok(v) = <f32>::try_get_by(res, idx) {
            return Ok(Self(v as f64));
        }

        if let Ok(v) = <i32>::try_get_by(res, idx) {
            return Ok(Self(v as f64));
        }

        let big_decimal = <BigDecimal>::try_get_by(res, idx)?;

        Ok(Self(big_decimal.to_f64().unwrap_or(0.0)))
    }
}
