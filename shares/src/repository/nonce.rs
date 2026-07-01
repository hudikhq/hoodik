//! In-memory nonce dedup, scoped to a single node. Each entry survives
//! 10 minutes — twice the ±5-minute replay window — so a request whose
//! timestamp passes the window check is guaranteed to find its nonce in
//! the cache if a sibling request used the same nonce.
use std::sync::{Mutex, OnceLock};

use cached::{Cached, TimedSizedCache};
use entity::Uuid;

type Cache = Mutex<TimedSizedCache<(Uuid, [u8; 16]), i64>>;

static NONCE_CACHE: OnceLock<Cache> = OnceLock::new();

fn cache() -> &'static Cache {
    NONCE_CACHE
        .get_or_init(|| Mutex::new(TimedSizedCache::with_size_and_lifespan(100_000, 600)))
}

/// True iff the `(sender, nonce)` pair was seen within the 10-minute
/// window. On hit, the existing entry is left in place — caller already
/// rejected the duplicate, so refreshing the TTL is unnecessary.
pub(crate) fn check_and_record(sender_id: Uuid, nonce: [u8; 16], now: i64) -> bool {
    let mut cache = cache().lock().expect("nonce cache mutex poisoned");
    if cache.cache_get(&(sender_id, nonce)).is_some() {
        return true;
    }
    cache.cache_set((sender_id, nonce), now);
    false
}

#[cfg(test)]
pub(crate) fn clear_for_tests() {
    let mut cache = cache().lock().expect("nonce cache mutex poisoned");
    cache.cache_clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_insert_returns_false_duplicate_returns_true() {
        clear_for_tests();
        let user = Uuid::new_v4();
        let nonce = [9u8; 16];
        assert!(!check_and_record(user, nonce, 0));
        assert!(check_and_record(user, nonce, 0));
    }

    #[test]
    fn different_sender_can_reuse_same_nonce() {
        clear_for_tests();
        let nonce = [3u8; 16];
        assert!(!check_and_record(Uuid::new_v4(), nonce, 0));
        assert!(!check_and_record(Uuid::new_v4(), nonce, 0));
    }
}
