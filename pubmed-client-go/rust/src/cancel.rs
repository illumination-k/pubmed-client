//! Caller-driven cancellation.
//!
//! Go's `context.Context` is the reason this exists. A boundary call blocks the
//! calling thread for the whole request, so without a way to interrupt it a
//! cancelled context could only be *reported* after the fact — the HTTP request
//! would keep running to completion. Instead Go allocates a token, hands it to
//! the call, and fires it from a watchdog goroutine when the context is done;
//! the request future is dropped at the next await point and the call returns
//! [`ErrorKind::Cancelled`](crate::error::ErrorKind::Cancelled) promptly.
//!
//! A token is single-use and owned by the caller: create with
//! [`pubmed_cancel_new`], fire with [`pubmed_cancel_trigger`], release with
//! [`pubmed_cancel_free`]. Firing is safe from any thread, and safe after the
//! call has already returned; freeing while a call still holds the token is
//! not, so Go joins its watchdog before freeing.

use std::future::Future;
use std::ptr;
use std::sync::OnceLock;

use tokio::runtime::Runtime;
use tokio::sync::watch;

use crate::error::{ShimError, ShimResult};

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Get or create the process-wide Tokio runtime used to block on async calls.
///
/// A single shared runtime keeps connection pools and the NCBI rate limiter
/// alive across calls, mirroring the Python and R bindings.
#[allow(clippy::expect_used)]
pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("failed to create Tokio runtime"))
}

/// Opaque cancellation token handed to Go as a raw pointer.
pub struct PubmedCancel {
    /// `true` once the token has been fired. A watch channel (rather than a
    /// bare flag plus `Notify`) makes the "fired before anyone waited" case
    /// race-free: a late subscriber still observes the current value.
    fired: watch::Sender<bool>,
}

impl PubmedCancel {
    /// Wait until the token fires. Never returns otherwise.
    async fn cancelled(&self) {
        let mut receiver = self.fired.subscribe();
        loop {
            // Scoped so the borrow guard is not held across the await below.
            if *receiver.borrow_and_update() {
                return;
            }
            if receiver.changed().await.is_err() {
                // The sender lives as long as the token, which the caller must
                // keep alive for the duration of the call, so this is
                // unreachable in practice. Park rather than report a spurious
                // cancellation.
                std::future::pending::<()>().await;
            }
        }
    }
}

/// Create a cancellation token. Never returns null.
///
/// The token must be released with [`pubmed_cancel_free`].
#[unsafe(no_mangle)]
pub extern "C" fn pubmed_cancel_new() -> *mut PubmedCancel {
    let (fired, _) = watch::channel(false);
    Box::into_raw(Box::new(PubmedCancel { fired }))
}

/// Fire a token, cancelling any call currently using it. Null is a no-op, and
/// firing more than once is harmless.
///
/// # Safety
///
/// `cancel` must be null or a live pointer from [`pubmed_cancel_new`]. It is
/// safe to call this from a different thread than the one running the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_cancel_trigger(cancel: *const PubmedCancel) {
    let Some(token) = (unsafe { cancel.as_ref() }) else {
        return;
    };
    // `send_replace`, not `send`: a token fired before any call subscribed has
    // no receivers, and `send` treats that as a closed channel and leaves the
    // value untouched — the cancellation would then be silently lost.
    let _previously_fired = token.fired.send_replace(true);
}

/// Release a token from [`pubmed_cancel_new`]. Null is a no-op.
///
/// # Safety
///
/// `cancel` must come from [`pubmed_cancel_new`], must not be used or freed
/// again afterwards, and must not be freed while a call is still using it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_cancel_free(cancel: *mut PubmedCancel) {
    if cancel.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(cancel) });
}

/// Block the calling thread on `future`, aborting early if `cancel` fires.
///
/// Passing a null `cancel` runs the future to completion, which is what Go does
/// for a context that can never be cancelled (`context.Background()`).
///
/// # Safety
///
/// `cancel` must be null or a live pointer from [`pubmed_cancel_new`] that
/// outlives this call.
pub unsafe fn block_on<F, T, E>(cancel: *const PubmedCancel, future: F) -> ShimResult<T>
where
    F: Future<Output = Result<T, E>>,
    ShimError: From<E>,
{
    let token = unsafe { cancel.as_ref() };

    runtime().block_on(async move {
        match token {
            None => future.await.map_err(ShimError::from),
            Some(token) => {
                tokio::select! {
                    // Biased so a token that fired before the call started wins
                    // deterministically instead of racing the first poll.
                    biased;
                    () = token.cancelled() => Err(ShimError::cancelled()),
                    result = future => result.map_err(ShimError::from),
                }
            }
        }
    })
}

/// Block the calling thread on an infallible `future`, aborting early if
/// `cancel` fires.
///
/// # Safety
///
/// See [`block_on`].
pub unsafe fn block_on_infallible<F, T>(cancel: *const PubmedCancel, future: F) -> ShimResult<T>
where
    F: Future<Output = T>,
{
    unsafe { block_on(cancel, async move { Ok::<T, ShimError>(future.await) }) }
}

/// Enter the runtime context, which the reqwest client builder requires.
pub fn enter_runtime() -> tokio::runtime::EnterGuard<'static> {
    runtime().enter()
}

/// A token that is guaranteed never to fire, for calls made without one.
pub const NO_CANCEL: *const PubmedCancel = ptr::null();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use std::time::Duration;

    #[test]
    fn a_null_token_runs_the_future_to_completion() {
        let result: ShimResult<u8> =
            unsafe { block_on(NO_CANCEL, async { Ok::<u8, ShimError>(7) }) };
        assert_eq!(result.expect("no cancellation"), 7);
    }

    #[test]
    fn an_unfired_token_runs_the_future_to_completion() {
        let token = pubmed_cancel_new();
        let result: ShimResult<u8> = unsafe { block_on(token, async { Ok::<u8, ShimError>(7) }) };
        assert_eq!(result.expect("token never fired"), 7);
        unsafe { pubmed_cancel_free(token) };
    }

    #[test]
    fn a_token_fired_beforehand_cancels_immediately() {
        let token = pubmed_cancel_new();
        unsafe { pubmed_cancel_trigger(token) };

        let result: ShimResult<u8> = unsafe {
            block_on(token, async {
                // Would outlast the test if cancellation did not win.
                tokio::time::sleep(Duration::from_secs(30)).await;
                Ok::<u8, ShimError>(7)
            })
        };

        let error = result.expect_err("the token was already fired");
        assert_eq!(error.kind, ErrorKind::Cancelled);
        unsafe { pubmed_cancel_free(token) };
    }

    #[test]
    fn a_token_fired_during_the_call_cancels_it() {
        let token = pubmed_cancel_new();

        // Mirrors Go's watchdog goroutine: another thread fires while the
        // caller is parked inside `block_on`.
        let address = token as usize;
        let watchdog = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            unsafe { pubmed_cancel_trigger(address as *const PubmedCancel) };
        });

        let result: ShimResult<u8> = unsafe {
            block_on(token, async {
                tokio::time::sleep(Duration::from_secs(30)).await;
                Ok::<u8, ShimError>(7)
            })
        };

        watchdog.join().expect("watchdog panicked");
        assert_eq!(
            result.expect_err("the token fired mid-call").kind,
            ErrorKind::Cancelled
        );
        unsafe { pubmed_cancel_free(token) };
    }

    #[test]
    fn triggering_tolerates_null_and_repeats() {
        unsafe { pubmed_cancel_trigger(ptr::null()) };

        let token = pubmed_cancel_new();
        unsafe { pubmed_cancel_trigger(token) };
        unsafe { pubmed_cancel_trigger(token) };
        unsafe { pubmed_cancel_free(token) };
    }

    #[test]
    fn free_tolerates_null() {
        unsafe { pubmed_cancel_free(ptr::null_mut()) };
    }
}
