pub mod data;
pub mod routes;

pub(crate) mod actions;
pub(crate) mod auth;
pub(crate) mod contracts;
pub(crate) mod jwt;
pub(crate) mod providers;
pub(crate) mod rate_limit;

pub(crate) const REFRESH_PATH: &str = "/api/auth/refresh";

#[cfg(test)]
mod test;

#[cfg(feature = "mock")]
pub mod mock;

/// Test-only hooks into the login rate limiter, hidden behind `mock` so they
/// never link into production binaries. Integration suites drive them to clear
/// window state and to advance the clock without sleeping.
#[cfg(feature = "mock")]
pub mod test_support {
    pub use crate::rate_limit::reset_for_tests as reset_auth_rate_limit;

    pub fn auth_rate_limit_check(
        identity: Option<&str>,
        ip: &str,
        now: i64,
    ) -> error::AppResult<()> {
        crate::rate_limit::check(identity, ip, now)
    }

    pub fn auth_rate_limit_charge_failure(identity: Option<&str>, ip: &str, now: i64) {
        crate::rate_limit::charge_failure(identity, ip, now)
    }
}
