//! Whether this deployment can hand clients a URL straight to the storage
//! bucket.
//!
//! The answer is a deployment fact, so it is kept here with the rest of them,
//! but working it out means reaching the bucket over the network and standing
//! in for a browser while doing it. That lives in the `fs` crate, which calls
//! [`record`] once during startup. Everything else reads [`verdict`].

use std::sync::OnceLock;

/// The verdict, and what to tell the operator when it is negative.
#[derive(Debug, Clone, Default)]
pub struct DirectVerdict {
    pub enabled: bool,
    /// Why not, in the operator's terms. Empty when the feature was never
    /// asked for, which is not a fault worth reporting.
    pub blockers: Vec<String>,
}

static VERDICT: OnceLock<DirectVerdict> = OnceLock::new();

/// What startup decided, or a disabled verdict when nothing has run yet —
/// the safe answer for tests and for any path that starts a server without
/// probing.
pub fn verdict() -> &'static DirectVerdict {
    VERDICT.get_or_init(DirectVerdict::default)
}

/// Record the verdict. The first call wins; later ones are ignored, so a
/// process that somehow probes twice cannot flip the answer under a request
/// that is already deciding what to advertise.
pub fn record(verdict: DirectVerdict) {
    let _ = VERDICT.set(verdict);
}
