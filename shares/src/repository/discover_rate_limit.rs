//! Per-caller token bucket for `/api/users/discover`: 20 requests/min/user,
//! sliding window, in-memory single-node only.
//!
//! Counter increments on every authenticated call regardless of outcome.
//! Hits and 404 misses share the same bucket — without this, the time-to-
//! response delta between an existing email and a non-existing one becomes
//! a free oracle. The bucket must not expose internal state through an
//! observable side effect.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use cached::{Cached, TimedSizedCache};
use entity::Uuid;

const WINDOW_SECONDS: i64 = 60;
const REQUESTS_PER_WINDOW: usize = 20;

/// Bounded queue of request timestamps for one caller. Trim happens on
/// every probe so a quiet caller's bucket eventually evicts via the
/// underlying `TimedSizedCache` lifespan without leaking memory.
type Bucket = VecDeque<i64>;

type Cache = Mutex<TimedSizedCache<Uuid, Bucket>>;

static DISCOVER_CACHE: OnceLock<Cache> = OnceLock::new();

fn cache() -> &'static Cache {
    DISCOVER_CACHE.get_or_init(|| {
        // 10_000 distinct callers per minute is comfortably above any
        // realistic working set; bucket lifespan matches the window so
        // an idle caller's bucket evicts cleanly.
        Mutex::new(TimedSizedCache::with_size_and_lifespan(10_000, WINDOW_SECONDS as u64))
    })
}

/// Record one attempt against `user_id`'s bucket. Returns `true` when the
/// caller has exceeded `REQUESTS_PER_WINDOW` within the last
/// `WINDOW_SECONDS`. The timestamp is appended unconditionally — every
/// call counts, hits and misses both — so attackers cannot enumerate by
/// burning only on the 404 path.
pub(crate) fn over_limit(user_id: Uuid, now: i64) -> bool {
    let mut cache = cache().lock().expect("discover rate-limit cache poisoned");
    let mut bucket = cache.cache_remove(&user_id).unwrap_or_default();
    let cutoff = now - WINDOW_SECONDS;
    while let Some(&front) = bucket.front() {
        if front < cutoff {
            bucket.pop_front();
        } else {
            break;
        }
    }
    bucket.push_back(now);
    let over = bucket.len() > REQUESTS_PER_WINDOW;
    cache.cache_set(user_id, bucket);
    over
}

/// Test-only escape hatch — integration suites that exercise the limit
/// repeatedly across scenarios need to reset between scenarios. Hidden
/// behind `#[cfg(any(test, feature = "test-support"))]` so production
/// builds never link it.
#[cfg(any(test, feature = "test-support"))]
pub fn reset_for_tests() {
    let mut cache = cache().lock().expect("discover rate-limit cache poisoned");
    cache.cache_clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each test uses a fresh UUID so the shared static cache doesn't
    // alias across parallel runs — no global clear required, no inter-
    // test races.

    #[test]
    fn first_twenty_requests_pass_then_twenty_first_trips() {
        let user = Uuid::new_v4();
        for _ in 0..REQUESTS_PER_WINDOW {
            assert!(!over_limit(user, 0));
        }
        assert!(over_limit(user, 0));
    }

    #[test]
    fn requests_outside_window_are_trimmed() {
        let user = Uuid::new_v4();
        for _ in 0..REQUESTS_PER_WINDOW {
            assert!(!over_limit(user, 0));
        }
        // A request `WINDOW_SECONDS + 1` later finds the bucket empty.
        assert!(!over_limit(user, WINDOW_SECONDS + 1));
    }

    #[test]
    fn separate_users_have_independent_buckets() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        for _ in 0..REQUESTS_PER_WINDOW {
            assert!(!over_limit(a, 0));
        }
        assert!(!over_limit(b, 0));
    }
}
