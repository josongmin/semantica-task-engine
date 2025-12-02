//! Rate Limiter (Token Bucket Algorithm)
//!
//! Prevents DoS attacks by limiting requests per second.
//! Uses atomic operations to avoid lock contention under high load.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Rate limiter using token bucket algorithm with atomic operations
pub struct RateLimiter {
    state: Arc<AtomicState>,
    max_tokens: u32,
    refill_rate: u32, // tokens per second
}

struct AtomicState {
    // Pack tokens (u32) and last_refill_ms (u32) into u64
    // Upper 32 bits: tokens * 1000 (fixed-point)
    // Lower 32 bits: last_refill timestamp (milliseconds since creation)
    packed: AtomicU64,
    creation_time: Instant,
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
        let tokens_fixed = (max_tokens as u64) << 32;
        Self {
            state: Arc::new(AtomicState {
                packed: AtomicU64::new(tokens_fixed),
                creation_time: Instant::now(),
            }),
            max_tokens,
            refill_rate,
        }
    }

    /// Check if request is allowed (consumes 1 token)
    ///
    /// Returns true if allowed, false if rate limited
    ///
    /// Uses atomic CAS loop to avoid lock contention
    pub async fn check(&self) -> bool {
        // CAS loop to update tokens atomically
        loop {
            let packed = self.state.packed.load(Ordering::Acquire);
            let tokens = (packed >> 32) as u32;
            let last_refill_ms = (packed & 0xFFFFFFFF) as u32;

            // Calculate elapsed time
            let now = Instant::now();
            let elapsed_ms = now
                .duration_since(self.state.creation_time)
                .as_millis() as u32;
            let delta_ms = elapsed_ms.saturating_sub(last_refill_ms);

            // Refill tokens
            let tokens_to_add = (delta_ms as u64 * self.refill_rate as u64) / 1000;
            let new_tokens = ((tokens as u64 + tokens_to_add).min(self.max_tokens as u64)) as u32;

            // Try to consume 1 token
            if new_tokens >= 1 {
                let consumed_tokens = new_tokens - 1;
                let new_packed = ((consumed_tokens as u64) << 32) | (elapsed_ms as u64);

                // CAS: update if no other thread modified it
                match self.state.packed.compare_exchange(
                    packed,
                    new_packed,
                    Ordering::Release,
                    Ordering::Acquire,
                ) {
                    Ok(_) => return true,
                    Err(_) => continue, // Retry
                }
            } else {
                // Not enough tokens, but still update timestamp
                let new_packed = ((new_tokens as u64) << 32) | (elapsed_ms as u64);
                let _ = self.state.packed.compare_exchange(
                    packed,
                    new_packed,
                    Ordering::Release,
                    Ordering::Acquire,
                );
                return false;
            }
        }
    }

    /// Get remaining tokens (for monitoring)
    #[allow(dead_code)] // Used for metrics in Phase 4
    pub async fn remaining(&self) -> f64 {
        let packed = self.state.packed.load(Ordering::Acquire);
        let tokens = (packed >> 32) as u32;
        tokens as f64
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

    #[tokio::test]
    async fn test_rate_limiter_concurrent() {
        use std::sync::Arc;
        
        let limiter = Arc::new(RateLimiter::new(100, 50)); // 50 req/sec, burst 100
        
        // Spawn 10 concurrent tasks, each trying 20 requests
        let mut handles = vec![];
        for _ in 0..10 {
            let limiter_clone = Arc::clone(&limiter);
            let handle = tokio::spawn(async move {
                let mut allowed = 0;
                for _ in 0..20 {
                    if limiter_clone.check().await {
                        allowed += 1;
                    }
                }
                allowed
            });
            handles.push(handle);
        }
        
        // Collect results
        let mut total_allowed = 0;
        for handle in handles {
            total_allowed += handle.await.unwrap();
        }
        
        // Total requests = 200, but only 100 should be allowed (burst limit)
        assert!(
            total_allowed <= 100,
            "Expected at most 100 allowed, got {}",
            total_allowed
        );
        assert!(
            total_allowed >= 90,
            "Expected at least 90 allowed (some tolerance), got {}",
            total_allowed
        );
    }
}

