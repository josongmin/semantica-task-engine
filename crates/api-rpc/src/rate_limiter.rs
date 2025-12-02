//! Simple Rate Limiter (Token Bucket Algorithm)
//!
//! Prevents DoS attacks by limiting requests per second.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    tokens: Arc<Mutex<TokenBucket>>,
    max_tokens: u32,
    refill_rate: u32, // tokens per second
}

struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    /// * `max_tokens` - Maximum burst size
    /// * `refill_rate` - Tokens added per second
    ///
    /// # Example
    /// Allow 100 requests/sec with burst of 200:
    /// `RateLimiter::new(200, 100)`
    pub fn new(max_tokens: u32, refill_rate: u32) -> Self {
        Self {
            tokens: Arc::new(Mutex::new(TokenBucket {
                tokens: max_tokens as f64,
                last_refill: Instant::now(),
            })),
            max_tokens,
            refill_rate,
        }
    }

    /// Check if request is allowed (consumes 1 token)
    ///
    /// Returns true if allowed, false if rate limited
    pub async fn check(&self) -> bool {
        let mut bucket = self.tokens.lock().await;

        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        let tokens_to_add = elapsed * self.refill_rate as f64;

        bucket.tokens = (bucket.tokens + tokens_to_add).min(self.max_tokens as f64);
        bucket.last_refill = now;

        // Try to consume 1 token
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get remaining tokens (for monitoring)
    #[allow(dead_code)] // Used for metrics in Phase 4
    pub async fn remaining(&self) -> f64 {
        let bucket = self.tokens.lock().await;
        bucket.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(10, 10);

        // Should allow 10 requests
        for _ in 0..10 {
            assert!(limiter.check().await);
        }

        // 11th should be denied
        assert!(!limiter.check().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_refills() {
        let limiter = RateLimiter::new(5, 10); // 10 tokens/sec

        // Consume all tokens
        for _ in 0..5 {
            assert!(limiter.check().await);
        }
        assert!(!limiter.check().await);

        // Wait 1 second for refill
        sleep(Duration::from_secs(1)).await;

        // Should have ~10 tokens now
        assert!(limiter.check().await);
    }
}

