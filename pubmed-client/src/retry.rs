//! Retry logic with exponential backoff for handling transient network failures
//!
//! This module provides a configurable retry mechanism with exponential backoff
//! and jitter to handle temporary network issues when communicating with NCBI APIs.

use std::{fmt::Display, future::Future};

use crate::time::{sleep, Duration};
use rand::Rng;
use tracing::{debug, warn};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: usize,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Base for exponential backoff (typically 2.0)
    pub backoff_base: f64,
    /// Whether to add jitter to retry delays
    pub use_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_base: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration with custom settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set the maximum delay
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Disable jitter
    pub fn without_jitter(mut self) -> Self {
        self.use_jitter = false;
        self
    }

    /// Calculate delay for a given retry attempt
    fn calculate_delay(&self, attempt: usize) -> Duration {
        let base_delay = self.initial_delay.as_millis() as f64;
        let exponential_delay = base_delay * self.backoff_base.powi(attempt as i32);
        let capped_delay = exponential_delay.min(self.max_delay.as_millis() as f64);

        let final_delay = if self.use_jitter {
            // Add jitter: random value between 0.5 and 1.5 of the calculated delay
            let mut rng = rand::thread_rng();
            let jitter_factor = rng.gen_range(0.5..1.5);
            capped_delay * jitter_factor
        } else {
            capped_delay
        };

        Duration::from_millis(final_delay as u64)
    }
}

/// Trait for errors that can be retried
pub trait RetryableError {
    /// Returns true if the error is transient and the operation should be retried
    fn is_retryable(&self) -> bool;

    /// Returns a human-readable description of why the error is/isn't retryable
    fn retry_reason(&self) -> &str {
        if self.is_retryable() {
            "Transient error, will retry"
        } else {
            "Non-transient error, will not retry"
        }
    }
}

/// Execute an operation with retry logic
///
/// # Arguments
///
/// * `operation` - A closure that returns a future with the operation to retry
/// * `config` - Retry configuration
/// * `operation_name` - A descriptive name for logging
///
/// # Returns
///
/// Returns the result of the operation, or the last error if all retries failed
pub async fn with_retry<F, Fut, T, E>(
    mut operation: F,
    config: &RetryConfig,
    operation_name: &str,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: RetryableError + Display,
{
    let mut attempt = 0;
    let mut last_error = None;

    while attempt <= config.max_retries {
        debug!(
            operation = operation_name,
            attempt = attempt,
            max_retries = config.max_retries,
            "Attempting operation"
        );

        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt,
                        "Operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(error) => {
                debug!(
                    operation = operation_name,
                    error = %error,
                    is_retryable = error.is_retryable(),
                    reason = error.retry_reason(),
                    "Error encountered"
                );

                if !error.is_retryable() {
                    debug!(
                        operation = operation_name,
                        error = %error,
                        reason = error.retry_reason(),
                        "Non-retryable error encountered"
                    );
                    return Err(error);
                }

                last_error = Some(error);

                if attempt < config.max_retries {
                    let delay = config.calculate_delay(attempt);
                    debug!(
                        operation = operation_name,
                        attempt = attempt + 1,
                        max_retries = config.max_retries,
                        delay_ms = delay.as_millis(),
                        error = %last_error.as_ref().unwrap(),
                        "Retryable error encountered, will retry after delay"
                    );
                    sleep(delay).await;
                } else {
                    warn!(
                        operation = operation_name,
                        attempts = attempt + 1,
                        error = %last_error.as_ref().unwrap(),
                        "Max retries exceeded, operation failed"
                    );
                }
            }
        }

        attempt += 1;
    }

    // This should never be reached due to the loop logic, but just in case
    Err(last_error.unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, thiserror::Error)]
    enum TestError {
        #[error("Retryable error")]
        Retryable,
        #[error("Non-retryable error")]
        NonRetryable,
    }

    impl RetryableError for TestError {
        fn is_retryable(&self) -> bool {
            matches!(self, TestError::Retryable)
        }
    }

    #[tokio::test]
    async fn test_successful_operation() {
        let config = RetryConfig::new().without_jitter();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = with_retry(
            || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Ok::<_, TestError>(42) }
            },
            &config,
            "test_operation",
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Called only once
    }

    #[tokio::test]
    async fn test_retry_then_success() {
        let config = RetryConfig::new()
            .with_max_retries(3)
            .with_initial_delay(Duration::from_millis(10))
            .without_jitter();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = with_retry(
            || {
                let count = counter_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if count < 2 {
                        Err(TestError::Retryable)
                    } else {
                        Ok(42)
                    }
                }
            },
            &config,
            "test_operation",
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Failed twice, succeeded on third
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let config = RetryConfig::new().with_max_retries(3);
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = with_retry(
            || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Err::<i32, _>(TestError::NonRetryable) }
            },
            &config,
            "test_operation",
        )
        .await;

        assert!(matches!(result, Err(TestError::NonRetryable)));
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Called only once
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let config = RetryConfig::new()
            .with_max_retries(2)
            .with_initial_delay(Duration::from_millis(10))
            .without_jitter();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = with_retry(
            || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Err::<i32, _>(TestError::Retryable) }
            },
            &config,
            "test_operation",
        )
        .await;

        assert!(matches!(result, Err(TestError::Retryable)));
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Initial attempt + 2 retries
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let config = RetryConfig::new()
            .with_initial_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(30))
            .without_jitter();

        // Test exponential growth
        assert_eq!(config.calculate_delay(0), Duration::from_secs(1));
        assert_eq!(config.calculate_delay(1), Duration::from_secs(2));
        assert_eq!(config.calculate_delay(2), Duration::from_secs(4));
        assert_eq!(config.calculate_delay(3), Duration::from_secs(8));
        assert_eq!(config.calculate_delay(4), Duration::from_secs(16));

        // Test max delay cap
        assert_eq!(config.calculate_delay(5), Duration::from_secs(30)); // Would be 32, capped at 30
        assert_eq!(config.calculate_delay(10), Duration::from_secs(30)); // Still capped
    }

    #[test]
    fn test_jitter() {
        let config = RetryConfig::new().with_initial_delay(Duration::from_secs(1));

        // With jitter, delays should vary
        let delay1 = config.calculate_delay(1);
        let delay2 = config.calculate_delay(1);

        // Both should be between 1-3 seconds (2 seconds * 0.5-1.5 jitter)
        assert!(delay1.as_millis() >= 1000);
        assert!(delay1.as_millis() <= 3000);
        assert!(delay2.as_millis() >= 1000);
        assert!(delay2.as_millis() <= 3000);

        // They're unlikely to be exactly the same with jitter
        // (though theoretically possible, so we don't assert inequality)
    }
}
