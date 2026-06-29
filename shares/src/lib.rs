pub mod data;
pub mod routes;

pub(crate) mod contracts;
pub(crate) mod repository;

/// Pre-flight cascade used by `admin::repository::users::delete` to
/// record audit rows before the DB engine-level CASCADE drops the
/// user's `user_files` rows. The shares crate owns
/// this so admin can call it without taking a direct dependency on
/// audit chain primitives.
pub use repository::account_deletion::pre_emit_for_user_delete;

/// Test-only state-reset hooks. Hidden behind `test-support` so they
/// never link into production binaries.
#[cfg(feature = "test-support")]
pub mod test_support {
    pub fn reset_discover_rate_limit() {
        crate::repository::discover_rate_limit::reset_for_tests();
    }
}
