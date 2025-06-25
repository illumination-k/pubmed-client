use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{debug, instrument, warn};

/// Rate limiter using token bucket algorithm for NCBI API compliance
///
/// NCBI E-utilities rate limits:
/// - 3 requests per second without API key
/// - 10 requests per second with API key
/// - Violations can result in IP blocking
#[derive(Clone)]
pub struct RateLimiter {
    bucket: Arc<Mutex<TokenBucket>>,
}

struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the specified rate
    ///
    /// # Arguments
    ///
    /// * `rate` - Maximum requests per second (e.g., 3.0 for NCBI without API key)
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::rate_limit::RateLimiter;
    ///
    /// // NCBI rate limit without API key
    /// let limiter = RateLimiter::new(3.0);
    ///
    /// // NCBI rate limit with API key
    /// let limiter_with_key = RateLimiter::new(10.0);
    /// ```
    pub fn new(rate: f64) -> Self {
        let capacity = rate.max(1.0); // Ensure minimum capacity
        Self {
            bucket: Arc::new(Mutex::new(TokenBucket {
                tokens: capacity,
                capacity,
                refill_rate: rate,
                last_refill: Instant::now(),
            })),
        }
    }

    /// Create rate limiter for NCBI API without API key (3 requests/second)
    pub fn ncbi_default() -> Self {
        Self::new(3.0)
    }

    /// Create rate limiter for NCBI API with API key (10 requests/second)
    pub fn ncbi_with_key() -> Self {
        Self::new(10.0)
    }

    /// Acquire a token, waiting if necessary to respect rate limits
    ///
    /// This method will block until a token is available, ensuring
    /// compliance with the configured rate limit.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client_rs::rate_limit::RateLimiter;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let limiter = RateLimiter::ncbi_default();
    ///
    ///     // This will respect the 3 requests/second limit
    ///     limiter.acquire().await;
    ///     // Make API call here
    ///
    ///     limiter.acquire().await;
    ///     // Make another API call here
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn acquire(&self) -> crate::Result<()> {
        let wait_time = {
            let mut bucket = self.bucket.lock().await;
            bucket.refill();

            if bucket.tokens >= 1.0 {
                bucket.tokens -= 1.0;
                debug!(remaining_tokens = %bucket.tokens, "Token acquired immediately");
                None
            } else {
                // Calculate wait time for next token
                let wait_duration = Duration::from_secs_f64(1.0 / bucket.refill_rate);
                debug!(
                    wait_duration_ms = wait_duration.as_millis(),
                    "Need to wait for token"
                );
                Some(wait_duration)
            }
        };

        if let Some(duration) = wait_time {
            debug!("Sleeping to respect rate limit");
            sleep(duration).await;

            // Try to acquire again after waiting
            let mut bucket = self.bucket.lock().await;
            bucket.refill();

            if bucket.tokens >= 1.0 {
                bucket.tokens -= 1.0;
                debug!(remaining_tokens = %bucket.tokens, "Token acquired after waiting");
            } else {
                warn!("Failed to acquire token after waiting - this should not happen");
                return Err(crate::error::PubMedError::RateLimitExceeded);
            }
        }

        Ok(())
    }

    /// Check if a token is available without blocking
    ///
    /// Returns `true` if a token is available and can be acquired immediately.
    /// This method does not consume a token.
    pub async fn check_available(&self) -> bool {
        let mut bucket = self.bucket.lock().await;
        bucket.refill();
        bucket.tokens >= 1.0
    }

    /// Get current token count (for testing and monitoring)
    pub async fn token_count(&self) -> f64 {
        let mut bucket = self.bucket.lock().await;
        bucket.refill();
        bucket.tokens
    }

    /// Get the configured rate limit (requests per second)
    pub async fn rate(&self) -> f64 {
        let bucket = self.bucket.lock().await;
        bucket.refill_rate
    }
}

impl TokenBucket {
    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let new_tokens = elapsed.as_secs_f64() * self.refill_rate;

        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::{Instant, sleep};

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(5.0);
        assert_eq!(limiter.rate().await, 5.0);
        assert_eq!(limiter.token_count().await, 5.0);
    }

    #[tokio::test]
    async fn test_ncbi_presets() {
        let default_limiter = RateLimiter::ncbi_default();
        assert_eq!(default_limiter.rate().await, 3.0);

        let key_limiter = RateLimiter::ncbi_with_key();
        assert_eq!(key_limiter.rate().await, 10.0);
    }

    #[tokio::test]
    async fn test_immediate_token_acquisition() {
        let limiter = RateLimiter::new(5.0);

        // Should be able to acquire tokens immediately up to capacity
        for _ in 0..5 {
            assert!(limiter.acquire().await.is_ok());
        }

        // No more tokens should be available immediately
        assert!(!limiter.check_available().await);
    }

    #[tokio::test]
    async fn test_token_refill() {
        let limiter = RateLimiter::new(10.0); // 10 tokens per second

        // Consume all tokens
        for _ in 0..10 {
            assert!(limiter.acquire().await.is_ok());
        }

        // Wait for refill (100ms should give us 1 token at 10/sec rate)
        sleep(Duration::from_millis(100)).await;

        // Should have at least 1 token available
        assert!(limiter.check_available().await);
        assert!(limiter.acquire().await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiting_timing() {
        let limiter = RateLimiter::new(2.0); // 2 requests per second

        let start = Instant::now();

        // Acquire 3 tokens - the third should require waiting
        limiter.acquire().await.unwrap();
        limiter.acquire().await.unwrap();
        limiter.acquire().await.unwrap();

        let elapsed = start.elapsed();

        // Should take at least 500ms for the third token (1/2 second)
        assert!(elapsed >= Duration::from_millis(450)); // Allow some tolerance
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let limiter = RateLimiter::new(5.0);
        let limiter_clone = limiter.clone();

        // Spawn concurrent tasks trying to acquire tokens
        let handle1 = tokio::spawn(async move {
            for _ in 0..3 {
                limiter.acquire().await.unwrap();
            }
        });

        let handle2 = tokio::spawn(async move {
            for _ in 0..3 {
                limiter_clone.acquire().await.unwrap();
            }
        });

        // Both should complete successfully
        assert!(handle1.await.is_ok());
        assert!(handle2.await.is_ok());
    }

    #[tokio::test]
    async fn test_minimum_capacity() {
        // Even with very low rate, should have minimum capacity of 1
        let limiter = RateLimiter::new(0.1);
        assert!(limiter.token_count().await >= 1.0);
        assert!(limiter.acquire().await.is_ok());
    }
}
