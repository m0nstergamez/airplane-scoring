//! Rate limits: token buckets in memory, one per key (a pilot, an IP
//! address). A bucket holds up to `burst` tokens and refills at a steady
//! rate; each request takes one. Nothing here is written anywhere: when
//! the machine stops (Fly stops it when nobody plays), every bucket
//! starts full again, which only ever favours honest players.

use std::collections::HashMap;
use std::sync::Mutex;

const HOUR_MS: f64 = 60.0 * 60.0 * 1000.0;

/// At most this many keys (about 100 bytes each). Full of keys that are
/// all in use (many addresses at once: an attack), a new key is refused
/// until the hourly cleanup frees some. Honest use needs a few hundred.
const MAX_KEYS: usize = 100_000;
/// `Retry-After` for a key refused because the map is full.
const FULL_RETRY_S: u64 = 60;

pub struct Limiter {
    burst: f64,
    /// Tokens per millisecond.
    refill: f64,
    buckets: Mutex<HashMap<String, Bucket>>,
}

struct Bucket {
    tokens: f64,
    at_ms: i64,
}

impl Limiter {
    /// `burst` requests at once, then `per_hour` an hour.
    pub fn new(burst: u32, per_hour: u32) -> Self {
        Self {
            burst: f64::from(burst),
            refill: f64::from(per_hour) / HOUR_MS,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    fn buckets(&self) -> std::sync::MutexGuard<'_, HashMap<String, Bucket>> {
        self.buckets.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Take a token for `key` at `now_ms`. `Err(seconds)`: none left,
    /// the next one comes in that many seconds (for `Retry-After`).
    pub fn take(&self, key: &str, now_ms: i64) -> Result<(), u64> {
        let mut buckets = self.buckets();
        if buckets.len() >= MAX_KEYS && !buckets.contains_key(key) {
            return Err(FULL_RETRY_S);
        }
        let bucket = buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: self.burst,
            at_ms: now_ms,
        });
        let passed = (now_ms - bucket.at_ms).max(0) as f64;
        bucket.tokens = (bucket.tokens + passed * self.refill).min(self.burst);
        bucket.at_ms = now_ms;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            let wait_ms = (1.0 - bucket.tokens) / self.refill;
            Err((wait_ms / 1000.0).ceil().max(1.0) as u64)
        }
    }

    /// Undo a `take` that went through: the request was refused for
    /// another reason, so it should not count against `key`.
    pub fn give_back(&self, key: &str) {
        if let Some(b) = self.buckets().get_mut(key) {
            b.tokens = (b.tokens + 1.0).min(self.burst);
        }
    }

    /// Drop the buckets that are full again by `now_ms`: the same as no
    /// bucket, so the map only holds keys seen within about an hour.
    pub fn forget_idle(&self, now_ms: i64) {
        let (burst, refill) = (self.burst, self.refill);
        self.buckets().retain(|_, b| {
            let passed = (now_ms - b.at_ms).max(0) as f64;
            b.tokens + passed * refill < burst
        });
    }

    /// How many keys it holds now (for tests and the hourly log line).
    pub fn keys(&self) -> usize {
        self.buckets().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINUTE: i64 = 60 * 1000;

    #[test]
    fn a_burst_then_the_hourly_rate() {
        let limit = Limiter::new(3, 60); // one a minute
        for _ in 0..3 {
            assert_eq!(limit.take("a", 0), Ok(()));
        }
        assert_eq!(limit.take("a", 0), Err(60), "the next in a minute");
        assert_eq!(limit.take("a", 30 * 1000), Err(30));
        assert_eq!(limit.take("a", MINUTE), Ok(()));
        assert!(limit.take("a", MINUTE).is_err());
    }

    #[test]
    fn every_key_has_its_own_bucket() {
        let limit = Limiter::new(1, 1);
        assert_eq!(limit.take("a", 0), Ok(()));
        assert!(limit.take("a", 0).is_err());
        assert_eq!(limit.take("b", 0), Ok(()));
    }

    #[test]
    fn a_bucket_never_holds_more_than_the_burst() {
        let limit = Limiter::new(2, 60);
        let day = 24 * 60 * MINUTE;
        assert_eq!(limit.take("a", day), Ok(()));
        assert_eq!(limit.take("a", day), Ok(()));
        assert!(
            limit.take("a", day).is_err(),
            "a quiet day saves up nothing"
        );
    }

    #[test]
    fn a_clock_going_back_takes_no_tokens_away_and_gives_none() {
        let limit = Limiter::new(1, 60);
        assert_eq!(limit.take("a", 10 * MINUTE), Ok(()));
        assert!(limit.take("a", 0).is_err());
    }

    #[test]
    fn a_token_given_back_can_be_taken_again() {
        let limit = Limiter::new(1, 1);
        assert_eq!(limit.take("a", 0), Ok(()));
        limit.give_back("a");
        assert_eq!(limit.take("a", 0), Ok(()));
        limit.give_back("a");
        limit.give_back("a");
        assert_eq!(limit.take("a", 0), Ok(()));
        assert!(limit.take("a", 0).is_err(), "never more than the burst");
    }

    #[test]
    fn the_map_of_keys_cannot_grow_without_bound() {
        let limit = Limiter::new(1, 60);
        for i in 0..MAX_KEYS {
            limit.take(&i.to_string(), 0).unwrap();
        }
        assert_eq!(limit.take("one more", 0), Err(FULL_RETRY_S));
        assert!(limit.take("7", 0).is_err(), "a known key: its own bucket");
        limit.forget_idle(2 * 60 * 1000);
        assert_eq!(limit.take("one more", 2 * 60 * 1000), Ok(()));
    }

    #[test]
    fn idle_keys_are_forgotten_once_full_again() {
        let limit = Limiter::new(2, 60);
        limit.take("a", 0).unwrap();
        limit.take("b", 0).unwrap();
        limit.take("b", MINUTE / 2).unwrap();
        limit.forget_idle(MINUTE);
        assert_eq!(limit.keys(), 1, "a is full again, b is not");
        limit.forget_idle(3 * MINUTE);
        assert_eq!(limit.keys(), 0);
    }
}
