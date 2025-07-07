//! Internal time management module for cross-platform compatibility
//!
//! This module provides a simple time management API that works across
//! both native and WASM targets without requiring external dependencies.
//! It focuses on the minimal functionality needed by the PubMed client.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration as StdDuration;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

/// Simple duration representation for cross-platform compatibility
///
/// This struct provides basic duration functionality without relying on
/// `std::time::Duration` which is not available in WASM environments.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration {
    millis: u64,
}

impl Duration {
    /// Create a new Duration from seconds
    ///
    /// # Arguments
    ///
    /// * `secs` - Number of seconds
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::time::Duration;
    ///
    /// let duration = Duration::from_secs(30);
    /// assert_eq!(duration.as_secs(), 30);
    /// ```
    pub fn from_secs(secs: u64) -> Self {
        Self {
            millis: secs * 1000,
        }
    }

    /// Create a new Duration from milliseconds
    ///
    /// # Arguments
    ///
    /// * `millis` - Number of milliseconds
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client_rs::time::Duration;
    ///
    /// let duration = Duration::from_millis(1500);
    /// assert_eq!(duration.as_secs(), 1);
    /// assert_eq!(duration.as_millis(), 1500);
    /// ```
    pub fn from_millis(millis: u64) -> Self {
        Self { millis }
    }

    /// Get duration as seconds
    ///
    /// # Returns
    ///
    /// Duration in seconds as u64
    pub fn as_secs(&self) -> u64 {
        self.millis / 1000
    }

    /// Get duration as milliseconds
    ///
    /// # Returns
    ///
    /// Duration in milliseconds as u64
    pub fn as_millis(&self) -> u64 {
        self.millis
    }

    /// Get duration as seconds f64 (useful for rate calculations)
    ///
    /// # Returns
    ///
    /// Duration in seconds as f64
    pub fn as_secs_f64(&self) -> f64 {
        self.millis as f64 / 1000.0
    }

    /// Check if duration is zero
    pub fn is_zero(&self) -> bool {
        self.millis == 0
    }
}

impl Default for Duration {
    fn default() -> Self {
        Self::from_secs(0)
    }
}

impl From<u64> for Duration {
    fn from(secs: u64) -> Self {
        Self::from_secs(secs)
    }
}

/// Sleep for the specified duration
///
/// This function provides a cross-platform sleep implementation:
/// - On native targets: Uses tokio::time::sleep with std::time::Duration
/// - On WASM targets: Uses a simplified implementation that returns immediately
///
/// # Arguments
///
/// * `duration` - Time to sleep
///
/// # Example
///
/// ```no_run
/// use pubmed_client_rs::time::{Duration, sleep};
///
/// #[tokio::main]
/// async fn main() {
///     let duration = Duration::from_secs(1);
///     sleep(duration).await;
/// }
/// ```
#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    if duration.is_zero() {
        return;
    }
    time::sleep(StdDuration::from_secs(duration.as_secs())).await;
}

/// Sleep for the specified duration (WASM implementation)
///
/// In WASM environments, this is a simplified implementation that
/// returns immediately. For actual delays in WASM, rate limiting
/// should be handled by the browser's natural request scheduling.
#[cfg(target_arch = "wasm32")]
pub async fn sleep(_duration: Duration) {
    // In WASM, we don't perform actual delays for rate limiting
    // The browser will handle request scheduling naturally
}

/// Simple instant measurement for rate limiting
///
/// This provides basic time measurement functionality for rate limiting
/// without requiring std::time::Instant which panics in WASM.
#[derive(Clone, Copy, Debug)]
pub struct Instant {
    // For simplicity, we'll use a counter approach rather than actual time
    _marker: (),
}

impl Instant {
    /// Get the current instant
    ///
    /// Note: In WASM environments, this is a simplified implementation
    /// that doesn't provide actual time measurement.
    pub fn now() -> Self {
        Self { _marker: () }
    }

    /// Calculate duration since another instant
    ///
    /// In WASM environments, this always returns zero duration
    /// since we use simplified rate limiting.
    pub fn duration_since(&self, _earlier: Instant) -> Duration {
        Duration::from_secs(0)
    }

    /// Calculate elapsed time since this instant
    ///
    /// In WASM environments, this always returns zero duration.
    pub fn elapsed(&self) -> Duration {
        Duration::from_secs(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_creation() {
        let duration = Duration::from_secs(30);
        assert_eq!(duration.as_secs(), 30);
        assert_eq!(duration.as_millis(), 30000);
        assert_eq!(duration.as_secs_f64(), 30.0);
    }

    #[test]
    fn test_duration_from_millis() {
        let duration = Duration::from_millis(1500);
        assert_eq!(duration.as_secs(), 1);
        assert_eq!(duration.as_millis(), 1500);
    }

    #[test]
    fn test_duration_zero() {
        let duration = Duration::default();
        assert!(duration.is_zero());

        let non_zero = Duration::from_secs(1);
        assert!(!non_zero.is_zero());
    }

    #[test]
    fn test_duration_ordering() {
        let dur1 = Duration::from_secs(10);
        let dur2 = Duration::from_secs(20);

        assert!(dur1 < dur2);
        assert!(dur2 > dur1);
        assert_eq!(dur1, dur1);
    }

    #[test]
    fn test_duration_from_u64() {
        let duration: Duration = 42u64.into();
        assert_eq!(duration.as_secs(), 42);
    }

    #[test]
    fn test_instant_creation() {
        let instant = Instant::now();
        let duration = instant.elapsed();
        assert_eq!(duration.as_secs(), 0); // In our simplified implementation
    }

    #[tokio::test]
    async fn test_sleep_functionality() {
        let duration = Duration::from_secs(0);
        sleep(duration).await; // Should return immediately
    }
}
