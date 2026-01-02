//! Internal time management module for cross-platform compatibility
//!
//! This module provides a simple time management API that works across
//! both native and WASM targets. On native, it uses tokio's time utilities.
//! On WASM, it uses gloo-timers for async sleep and js_sys::Date for time measurement.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration as StdDuration;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;

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
    time::sleep(StdDuration::from_millis(duration.as_millis())).await;
}

/// Sleep for the specified duration (WASM implementation)
///
/// Uses gloo-timers to provide actual async delays in WASM environments,
/// enabling proper rate limiting in browser contexts.
#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: Duration) {
    if duration.is_zero() {
        return;
    }
    // gloo-timers uses u32 for milliseconds, cap at u32::MAX
    let millis = duration.as_millis().min(u32::MAX as u64) as u32;
    TimeoutFuture::new(millis).await;
}

/// Simple instant measurement for rate limiting
///
/// This provides basic time measurement functionality for rate limiting.
/// On native targets, wraps std::time::Instant.
/// On WASM targets, uses js_sys::Date::now() for millisecond precision.
#[derive(Clone, Copy, Debug)]
pub struct Instant {
    /// Timestamp in milliseconds
    millis: f64,
}

impl Instant {
    /// Get the current instant
    ///
    /// On native targets, uses std::time::Instant.
    /// On WASM targets, uses js_sys::Date::now() for proper time measurement.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn now() -> Self {
        use std::time::Instant as StdInstant;
        // Use a thread-local origin point for consistent measurements
        thread_local! {
            static ORIGIN: StdInstant = StdInstant::now();
        }
        ORIGIN.with(|origin| Self {
            millis: origin.elapsed().as_secs_f64() * 1000.0,
        })
    }

    /// Get the current instant (WASM implementation)
    #[cfg(target_arch = "wasm32")]
    pub fn now() -> Self {
        Self {
            millis: js_sys::Date::now(),
        }
    }

    /// Calculate duration since another instant
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        let diff_millis = (self.millis - earlier.millis).max(0.0);
        Duration::from_millis(diff_millis as u64)
    }

    /// Calculate elapsed time since this instant
    pub fn elapsed(&self) -> Duration {
        Self::now().duration_since(*self)
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
        // Elapsed time should be very small (less than 1 second)
        assert!(duration.as_secs() < 1);
    }

    #[test]
    fn test_instant_duration_since() {
        let earlier = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let later = Instant::now();
        let duration = later.duration_since(earlier);
        // Should measure at least 40ms (allowing some tolerance)
        assert!(duration.as_millis() >= 40);
    }

    #[tokio::test]
    async fn test_sleep_functionality() {
        let duration = Duration::from_secs(0);
        sleep(duration).await; // Should return immediately
    }
}
