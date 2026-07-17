//! Sliding-window lockout for the surfaces where a password or key is guessed
//! online: `login`, `login/finish`, `signature`, and `change-password`. Only
//! *failed* authentications are charged, following OWASP guidance — a wrong
//! password or bad signature counts against the budget, a correct one does not.
//!
//! Charging failures only is also what keeps the limiter from leaking account
//! existence. An attacker probing whether an address is registered never has
//! the password, so every probe fails and accrues identically whether the
//! account exists or not: there is no delta to read. Charging successes would
//! instead feed the victim's own logins into a window the attacker can watch
//! drain, disclosing that the account exists and is in active use — and would
//! let any user lock themselves out with correct passwords. Enumeration is
//! defended by uniform responses, not by counting the honest path.
//!
//! Two windows guard each attempt. They are read before authentication and the
//! request is refused (429) when either is already full:
//!   - identity (email or key fingerprint): 10 failures / 5 min. The primary
//!     guard — focused guessing of one account trips here whatever the source.
//!   - source IP: 100 failures / 5 min. A coarse backstop for the case the
//!     identity window cannot see: one host spraying a single password across
//!     many accounts. Set well above any burst of genuine failures a shared
//!     office NAT — or a reverse proxy forwarding no client IP, collapsing
//!     every user onto one address — would produce.
//!
//! `login/start` (OPAQUE) is deliberately not charged here. It carries no secret
//! to guess and always succeeds — an unknown email gets a decoy — so a
//! failures-only limiter has nothing to count, and charging its successes would
//! reopen the enumeration oracle above. Its residual cost is self-capping rather
//! than a guessing surface: the handler purges every expired login-state row on
//! each call (60s TTL), so `opaque_login_sessions` never grows past one window's
//! inserts and drains when idle; the server-side OPRF is a cheap blind eval (the
//! expensive KSF runs on the client); and the branch that writes a row at all
//! needs a known migrated email. The migration-method it discloses is an
//! accepted transition-window necessity. Should single-source hammering of that
//! endpoint ever need a ceiling, the fit is a generous per-IP window charged
//! unconditionally there — or reverse-proxy flood control, where it belongs.
//!
//! In-memory and process-local: the windows live in a static and are lost on
//! restart, the trade-off the discover limiter also makes. Hoodik runs one
//! server process, so a shared cross-node store is scope it does not yet have.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

use error::{AppResult, Error};

const WINDOW_SECONDS: i64 = 300;
const IDENTITY_ATTEMPTS: usize = 10;
const IP_ATTEMPTS: usize = 100;

/// Ceiling on distinct tracked keys. Crossing it sweeps out windows whose most
/// recent failure has already aged out, reclaiming the long tail of one-off
/// attacker IPs without a background task.
const MAX_TRACKED_KEYS: usize = 50_000;

type Window = VecDeque<i64>;

fn windows() -> &'static Mutex<HashMap<String, Window>> {
    static WINDOWS: OnceLock<Mutex<HashMap<String, Window>>> = OnceLock::new();
    WINDOWS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_full(windows: &HashMap<String, Window>, key: &str, threshold: usize, cutoff: i64) -> bool {
    windows
        .get(key)
        .is_some_and(|w| w.iter().filter(|&&t| t >= cutoff).count() >= threshold)
}

fn record(windows: &mut HashMap<String, Window>, key: String, now: i64, cutoff: i64) {
    let window = windows.entry(key).or_default();
    while window.front().is_some_and(|&t| t < cutoff) {
        window.pop_front();
    }
    window.push_back(now);
}

/// Refuse with [`Error::TooManyRequests`] when the identity or source-IP window
/// is already full. Read-only: a mere check never counts as an attempt, so the
/// honest login path leaves no trace and cannot be used to drain a budget.
pub(crate) fn check(identity: Option<&str>, ip: &str, now: i64) -> AppResult<()> {
    let cutoff = now - WINDOW_SECONDS;
    let windows = windows().lock().expect("auth rate-limit windows poisoned");

    let ip_full = is_full(&windows, &format!("ip:{ip}"), IP_ATTEMPTS, cutoff);
    let identity_full =
        identity.is_some_and(|id| is_full(&windows, &format!("id:{id}"), IDENTITY_ATTEMPTS, cutoff));

    if ip_full || identity_full {
        Err(Error::TooManyRequests("too_many_attempts".to_string()))
    } else {
        Ok(())
    }
}

/// Charge one failed attempt to the identity and source-IP windows. Call only
/// after authentication has actually failed.
pub(crate) fn charge_failure(identity: Option<&str>, ip: &str, now: i64) {
    let cutoff = now - WINDOW_SECONDS;
    let mut windows = windows().lock().expect("auth rate-limit windows poisoned");

    record(&mut windows, format!("ip:{ip}"), now, cutoff);
    if let Some(id) = identity {
        record(&mut windows, format!("id:{id}"), now, cutoff);
    }

    if windows.len() > MAX_TRACKED_KEYS {
        windows.retain(|_, w| w.back().is_some_and(|&t| t >= cutoff));
    }
}

#[cfg(any(test, feature = "mock"))]
pub fn reset_for_tests() {
    windows()
        .lock()
        .expect("auth rate-limit windows poisoned")
        .clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use entity::Uuid;

    // Unique keys per test so the shared static never aliases under the
    // parallel runner; no global reset, no inter-test races.
    fn unique() -> (String, String) {
        (Uuid::new_v4().to_string(), Uuid::new_v4().to_string())
    }

    #[test]
    fn refuses_once_the_identity_window_is_full() {
        let (id, ip) = unique();
        for _ in 0..IDENTITY_ATTEMPTS {
            assert!(check(Some(&id), &ip, 0).is_ok());
            charge_failure(Some(&id), &ip, 0);
        }
        assert!(check(Some(&id), &ip, 0).is_err());
    }

    #[test]
    fn successes_never_accumulate() {
        let (id, ip) = unique();
        // check() is the only call the success path makes; on its own it must
        // never fill a window no matter how often a user logs in.
        for _ in 0..(IDENTITY_ATTEMPTS * 5) {
            assert!(check(Some(&id), &ip, 0).is_ok());
        }
    }

    #[test]
    fn window_slides_forward() {
        let (id, ip) = unique();
        for _ in 0..IDENTITY_ATTEMPTS {
            charge_failure(Some(&id), &ip, 0);
        }
        assert!(check(Some(&id), &ip, 0).is_err());
        assert!(check(Some(&id), &ip, WINDOW_SECONDS + 1).is_ok());
    }

    #[test]
    fn ip_window_trips_independent_of_identity() {
        let ip = Uuid::new_v4().to_string();
        // Each failure names a different identity, so no identity window fills —
        // only the shared IP window does.
        for _ in 0..IP_ATTEMPTS {
            charge_failure(Some(&Uuid::new_v4().to_string()), &ip, 0);
        }
        assert!(check(None, &ip, 0).is_err());
        let (fresh_id, fresh_ip) = unique();
        assert!(check(Some(&fresh_id), &fresh_ip, 0).is_ok());
    }
}
