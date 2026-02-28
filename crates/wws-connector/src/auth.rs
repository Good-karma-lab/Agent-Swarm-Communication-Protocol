//! RPC rate limiting — token bucket per agent DID.

use std::collections::HashMap;
use std::time::Instant;

/// Simple token bucket rate limiter per caller identity.
pub struct RateLimiter {
    buckets: HashMap<String, TokenBucket>,
    capacity: u32,
    refill_per_sec: f64,
}

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a rate limiter.
    /// - `capacity`: max burst tokens
    /// - `refill_per_sec`: steady-state rate (tokens per second)
    pub fn new(capacity: u32, refill_per_sec: f64) -> Self {
        Self { buckets: HashMap::new(), capacity, refill_per_sec }
    }

    /// Returns true if the request is allowed, false if rate-limited.
    pub fn check(&mut self, caller_id: &str) -> bool {
        let now = Instant::now();
        let bucket = self.buckets.entry(caller_id.to_string()).or_insert_with(|| {
            TokenBucket { tokens: self.capacity as f64, last_refill: now }
        });
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.refill_per_sec).min(self.capacity as f64);
        bucket.last_refill = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_capacity() {
        let mut rl = RateLimiter::new(5, 1.0);
        for _ in 0..5 {
            assert!(rl.check("agent1"), "should allow up to capacity");
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_capacity() {
        let mut rl = RateLimiter::new(3, 0.0); // refill=0 so no refill
        for _ in 0..3 { rl.check("agent1"); }
        assert!(!rl.check("agent1"), "should block when over capacity");
    }

    #[test]
    fn test_rate_limiter_separate_buckets() {
        let mut rl = RateLimiter::new(1, 0.0); // 1 token max, no refill
        assert!(rl.check("agent1"));
        assert!(!rl.check("agent1")); // agent1 exhausted
        assert!(rl.check("agent2")); // agent2 separate bucket
    }
}
