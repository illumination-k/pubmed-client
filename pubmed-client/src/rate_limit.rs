//! Rate limiting implementation for NCBI API compliance
//!
//! This module provides rate limiting functionality that respects NCBI E-utilities guidelines.
//! Uses a unified implementation that works across both native and WASM targets.

use crate::time::{sleep, Duration, Instant};
use std::sync::Arc;
use std::sync::Mutex;
use tracing::{debug, instrument};

/// NCBI E-utilities rate limits:
/// - 3 requests per second without API key
/// - 10 requests per second with API key
/// - Violations can result in IP blocking
///
/// Token bucket rate limiter for NCBI API compliance
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
    /// * `rate` - Maximum requests per second (e.g., 3.0 for NCBI default)
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::RateLimiter;
    ///
    /// // Create rate limiter for NCBI API without key (3 req/sec)
    /// let limiter_default = RateLimiter::new(3.0);
    ///
    /// // Create rate limiter for NCBI API with key (10 req/sec)
    /// let limiter_with_key = RateLimiter::new(10.0);
    /// ```
    pub fn new(rate: f64) -> Self {
        let capacity = rate.max(1.0); // Ensure minimum capacity
        let now = Instant::now();
        Self {
            bucket: Arc::new(Mutex::new(TokenBucket {
                tokens: capacity,
                capacity,
                refill_rate: rate,
                last_refill: now,
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
    /// This method implements a token bucket algorithm with the following behavior:
    /// 1. Check if tokens are available in the bucket
    /// 2. If available, consume one token and return immediately
    /// 3. If not available, wait for the appropriate interval
    /// 4. Refill the bucket and consume one token
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pubmed_client_rs::RateLimiter;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let limiter = RateLimiter::ncbi_default();
    ///
    ///     // This will respect the 3 req/sec limit
    ///     for i in 0..5 {
    ///         limiter.acquire().await?;
    ///         println!("Making API call {}", i + 1);
    ///         // Make your API call here
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn acquire(&self) -> crate::Result<()> {
        let should_wait = {
            let mut bucket = self.bucket.lock().unwrap();
            self.refill_bucket(&mut bucket);

            if bucket.tokens >= 1.0 {
                bucket.tokens -= 1.0;
                debug!(remaining_tokens = %bucket.tokens, "Token acquired immediately");
                false
            } else {
                debug!("No tokens available, need to wait");
                true
            }
        };

        if should_wait {
            // Calculate wait time based on rate
            let wait_duration = Duration::from_secs(1).as_secs_f64() / self.rate();
            let wait_duration = Duration::from_millis((wait_duration * 1000.0) as u64);

            debug!(
                wait_duration_ms = wait_duration.as_millis(),
                "Waiting for rate limit"
            );
            sleep(wait_duration).await;

            // After waiting, refill bucket and consume token
            let mut bucket = self.bucket.lock().unwrap();
            self.refill_bucket(&mut bucket);
            bucket.tokens = bucket.tokens.min(bucket.capacity);
            if bucket.tokens >= 1.0 {
                bucket.tokens -= 1.0;
                debug!(remaining_tokens = %bucket.tokens, "Token acquired after waiting");
            }
        }

        Ok(())
    }

    /// Check if a token is available without blocking
    ///
    /// Returns `true` if a token is available and can be acquired immediately.
    /// This method does not consume a token.
    pub fn check_available(&self) -> bool {
        let mut bucket = self.bucket.lock().unwrap();
        self.refill_bucket(&mut bucket);
        bucket.tokens >= 1.0
    }

    /// Get current token count (for testing and monitoring)
    pub fn token_count(&self) -> f64 {
        let mut bucket = self.bucket.lock().unwrap();
        self.refill_bucket(&mut bucket);
        bucket.tokens
    }

    /// Get the configured rate limit (requests per second)
    pub fn rate(&self) -> f64 {
        let bucket = self.bucket.lock().unwrap();
        bucket.refill_rate
    }

    /// Refill the token bucket based on elapsed time
    fn refill_bucket(&self, bucket: &mut TokenBucket) {
        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last_refill);

        // Calculate tokens to add based on elapsed time
        let tokens_to_add = elapsed.as_secs_f64() * bucket.refill_rate;
        bucket.tokens = (bucket.tokens + tokens_to_add).min(bucket.capacity);

        bucket.last_refill = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_functionality() {
        let limiter = RateLimiter::new(5.0);

        // Should be able to acquire tokens
        limiter.acquire().await.unwrap();

        // Check rate
        let rate = limiter.rate();
        assert!((rate - 5.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_check_available() {
        let limiter = RateLimiter::new(2.0);

        // Should have tokens available initially
        assert!(limiter.check_available());
    }

    #[tokio::test]
    async fn test_ncbi_presets() {
        let default_limiter = RateLimiter::ncbi_default();
        let with_key_limiter = RateLimiter::ncbi_with_key();

        assert!((default_limiter.rate() - 3.0).abs() < 0.1);
        assert!((with_key_limiter.rate() - 10.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_rate_limiting_basic() {
        let limiter = RateLimiter::new(1.0); // 1 request per second

        // Should be able to acquire tokens
        limiter.acquire().await.unwrap();
        limiter.acquire().await.unwrap(); // This should involve a wait

        // Rate limiter should still work
        let tokens = limiter.token_count();
        assert!(tokens >= 0.0);
    }
}
