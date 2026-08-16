//! Client handle lifecycle and configuration decoding.

use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::Arc;

use serde::Deserialize;

use pubmed_client::{Client, ClientConfig};

use crate::cancel::enter_runtime;
use crate::error::ShimError;
use crate::ffi::{borrow_str, clear_error, set_error};

/// Opaque client handle handed to Go as a raw pointer.
///
/// Created by [`pubmed_client_new`] and released by [`pubmed_client_free`].
pub struct PubmedClient {
    inner: Arc<Client>,
}

/// Borrow a client handle.
///
/// # Safety
///
/// `handle` must be null, or a pointer returned by [`pubmed_client_new`] that
/// has not yet been passed to [`pubmed_client_free`].
pub unsafe fn borrow_client(handle: *const PubmedClient) -> Result<Arc<Client>, ShimError> {
    let Some(client) = (unsafe { handle.as_ref() }) else {
        return Err(ShimError::invalid_argument(
            "client handle must not be null",
        ));
    };
    Ok(client.inner.clone())
}

/// Client configuration decoded from the JSON blob Go passes to
/// [`pubmed_client_new`]. Every field is optional; omitted values fall back to
/// the `pubmed-client` defaults.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ConfigDto {
    api_key: Option<String>,
    email: Option<String>,
    tool: Option<String>,
    rate_limit: Option<f64>,
    timeout_seconds: Option<u64>,
    user_agent: Option<String>,
    base_url: Option<String>,
    cache: bool,
}

impl ConfigDto {
    fn into_config(self) -> ClientConfig {
        let mut config = ClientConfig::new();
        if let Some(api_key) = self.api_key {
            config = config.with_api_key(api_key);
        }
        if let Some(email) = self.email {
            config = config.with_email(email);
        }
        if let Some(tool) = self.tool {
            config = config.with_tool(tool);
        }
        if let Some(rate_limit) = self.rate_limit {
            config = config.with_rate_limit(rate_limit);
        }
        if let Some(timeout_seconds) = self.timeout_seconds {
            config = config.with_timeout_seconds(timeout_seconds);
        }
        if let Some(user_agent) = self.user_agent {
            config = config.with_user_agent(user_agent);
        }
        if let Some(base_url) = self.base_url {
            config = config.with_base_url(base_url);
        }
        if self.cache {
            config = config.with_cache();
        }
        config
    }
}

/// Version of the underlying Rust crate, as a static NUL-terminated string.
///
/// The returned pointer is borrowed and must NOT be freed.
#[unsafe(no_mangle)]
pub extern "C" fn pubmed_client_version() -> *const c_char {
    const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr().cast()
}

/// Create a client from a JSON configuration blob (see `ConfigDto`).
///
/// Pass null for `config_json` to use the library defaults. Returns null and
/// sets `*out_err` on failure. The handle must be released with
/// [`pubmed_client_free`].
///
/// # Safety
///
/// `config_json` must be null or a valid NUL-terminated string, and `out_err`
/// must be null or point to a writable `*mut c_char`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_client_new(
    config_json: *const c_char,
    out_err: *mut *mut c_char,
) -> *mut PubmedClient {
    unsafe { clear_error(out_err) };

    let outcome = catch_unwind(AssertUnwindSafe(
        || -> Result<*mut PubmedClient, ShimError> {
            let config = if config_json.is_null() {
                ConfigDto::default()
            } else {
                let raw = unsafe { borrow_str(config_json, "config_json") }?;
                serde_json::from_str(raw).map_err(|e| {
                    ShimError::invalid_argument(format!("invalid client config: {e}"))
                })?
            };

            // Enter the runtime: the HTTP client builder expects a reactor context.
            let _guard = enter_runtime();
            let client = Client::with_config(config.into_config());
            Ok(Box::into_raw(Box::new(PubmedClient {
                inner: Arc::new(client),
            })))
        },
    ))
    .unwrap_or_else(|_| Err(ShimError::panicked("panic while creating client")));

    match outcome {
        Ok(handle) => handle,
        Err(error) => {
            unsafe { set_error(out_err, &error) };
            ptr::null_mut()
        }
    }
}

/// Release a handle returned by [`pubmed_client_new`]. Null is a no-op.
///
/// # Safety
///
/// `handle` must come from [`pubmed_client_new`] and must not be used or freed
/// again afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_client_free(handle: *mut PubmedClient) {
    if handle.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(handle) });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::ffi::pubmed_string_free;
    use std::ffi::CStr;

    /// Read and free an error envelope, returning its `kind`.
    fn error_kind(err: *mut c_char) -> String {
        assert!(!err.is_null(), "expected an error envelope");
        let raw = unsafe { CStr::from_ptr(err) }.to_string_lossy().to_string();
        unsafe { pubmed_string_free(err) };

        let parsed: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or_else(|_| panic!("not an envelope: {raw}"));
        parsed["kind"]
            .as_str()
            .unwrap_or_else(|| panic!("no kind in {raw}"))
            .to_string()
    }

    #[test]
    fn client_new_accepts_null_config() {
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(ptr::null(), &mut err) };
        assert!(!handle.is_null());
        assert!(err.is_null());
        unsafe { pubmed_client_free(handle) };
    }

    #[test]
    fn client_new_reports_invalid_config() {
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(c"{ not json }".as_ptr(), &mut err) };
        assert!(handle.is_null());
        assert_eq!(error_kind(err), "invalid_argument");
    }

    #[test]
    fn client_new_rejects_unknown_config_keys() {
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(c"{\"nope\": 1}".as_ptr(), &mut err) };
        assert!(handle.is_null());
        assert_eq!(error_kind(err), "invalid_argument");
    }

    #[test]
    fn client_new_applies_config() {
        let config = c"{\"api_key\":\"k\",\"email\":\"a@b.c\",\"rate_limit\":9.0,\"cache\":true}";
        let mut err: *mut c_char = ptr::null_mut();
        let handle = unsafe { pubmed_client_new(config.as_ptr(), &mut err) };
        assert!(!handle.is_null(), "unexpected error creating client");
        unsafe { pubmed_client_free(handle) };
    }

    #[test]
    fn borrowing_a_null_handle_is_an_invalid_argument() {
        let Err(error) = (unsafe { borrow_client(ptr::null()) }) else {
            panic!("null is not a handle");
        };
        assert_eq!(error.kind, ErrorKind::InvalidArgument);
    }

    #[test]
    fn version_is_the_crate_version() {
        let version = unsafe { CStr::from_ptr(pubmed_client_version()) };
        assert_eq!(version.to_string_lossy(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn free_tolerates_null() {
        unsafe { pubmed_client_free(ptr::null_mut()) };
    }
}
