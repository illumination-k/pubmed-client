//! Cross-platform time module for rate limiting and retry backoff.
//!
//! - **Native**: delegates to `std::time::Instant` and `tokio::time::sleep`.
//! - **WASM**: uses `js_sys::Date::now()` for time measurement and
//!   `setTimeout` (via `js_sys::Promise`) for async sleep.

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration as StdDuration, Instant as StdInstant};
#[cfg(not(target_arch = "wasm32"))]
use tokio::time as tokio_time;

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
    /// use pubmed_client::time::Duration;
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
    /// use pubmed_client::time::Duration;
    ///
    /// let duration = Duration::from_millis(1500);
    /// assert_eq!(duration.as_secs(), 1);
    /// assert_eq!(duration.as_millis(), 1500);
    /// ```
    pub fn from_millis(millis: u64) -> Self {
        Self { millis }
    }

    /// Get duration as seconds
    pub fn as_secs(&self) -> u64 {
        self.millis / 1000
    }

    /// Get duration as milliseconds
    pub fn as_millis(&self) -> u64 {
        self.millis
    }

    /// Get duration as seconds f64 (useful for rate calculations)
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

// ---------------------------------------------------------------------------
// sleep
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    if duration.is_zero() {
        return;
    }
    tokio_time::sleep(StdDuration::from_millis(duration.as_millis())).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: Duration) {
    use wasm_bindgen::prelude::*;

    if duration.is_zero() {
        return;
    }
    let ms = duration.as_millis() as f64;
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let global = js_sys::global();
        if let Ok(set_timeout) = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout")) {
            let set_timeout_fn: js_sys::Function = set_timeout.unchecked_into();
            let _ = set_timeout_fn.call2(&JsValue::undefined(), &resolve, &JsValue::from_f64(ms));
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

// ---------------------------------------------------------------------------
// Instant
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug)]
pub struct Instant {
    inner: StdInstant,
}

#[cfg(not(target_arch = "wasm32"))]
impl Instant {
    pub fn now() -> Self {
        Self {
            inner: StdInstant::now(),
        }
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        let std_dur = self.inner.duration_since(earlier.inner);
        Duration::from_millis(std_dur.as_millis() as u64)
    }

    pub fn elapsed(&self) -> Duration {
        let std_dur = self.inner.elapsed();
        Duration::from_millis(std_dur.as_millis() as u64)
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug)]
pub struct Instant {
    epoch_millis: f64,
}

#[cfg(target_arch = "wasm32")]
impl Instant {
    pub fn now() -> Self {
        Self {
            epoch_millis: js_sys::Date::now(),
        }
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        let diff = (self.epoch_millis - earlier.epoch_millis).max(0.0);
        Duration::from_millis(diff as u64)
    }

    pub fn elapsed(&self) -> Duration {
        let now = js_sys::Date::now();
        let diff = (now - self.epoch_millis).max(0.0);
        Duration::from_millis(diff as u64)
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
    fn test_instant_elapsed_is_non_negative() {
        let instant = Instant::now();
        let elapsed = instant.elapsed();
        assert!(elapsed.as_millis() < 1000);
    }

    #[tokio::test]
    async fn test_instant_duration_since() {
        let earlier = Instant::now();
        sleep(Duration::from_millis(50)).await;
        let later = Instant::now();
        let diff = later.duration_since(earlier);
        assert!(diff.as_millis() >= 30);
    }

    #[tokio::test]
    async fn test_sleep_functionality() {
        let duration = Duration::from_secs(0);
        sleep(duration).await;
    }

    #[tokio::test]
    async fn test_sleep_actually_waits() {
        let before = Instant::now();
        sleep(Duration::from_millis(100)).await;
        let elapsed = before.elapsed();
        assert!(elapsed.as_millis() >= 50);
    }
}
