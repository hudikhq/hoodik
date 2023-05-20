pub mod data;
pub mod routes;

pub(crate) mod actions;
pub(crate) mod auth;
pub(crate) mod contract;
pub(crate) mod jwt;
pub(crate) mod providers;

mod emails;

pub(crate) const REFRESH_PATH: &str = "/api/auth/refresh";

#[cfg(test)]
mod test;

#[cfg(feature = "mock")]
pub mod mock;
