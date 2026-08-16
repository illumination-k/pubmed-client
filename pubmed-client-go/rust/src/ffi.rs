//! Boundary plumbing shared by every exported function: argument borrowing,
//! result/error ownership, and panic containment.

use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use serde::Serialize;

use crate::error::{ShimError, ShimResult};

/// Copy a Rust string into a freshly allocated C string owned by the caller.
fn into_c_string(value: String) -> ShimResult<*mut c_char> {
    CString::new(value)
        .map(CString::into_raw)
        .map_err(|_| ShimError::internal("result contained an interior NUL byte"))
}

/// Serialize `value` as JSON for transport across the boundary.
pub fn to_json<T: Serialize>(value: &T) -> ShimResult<String> {
    serde_json::to_string(value)
        .map_err(|e| ShimError::internal(format!("failed to serialize response: {e}")))
}

/// Borrow a NUL-terminated C string as a `&str`.
///
/// # Safety
///
/// `value` must be null, or point to a valid NUL-terminated string that stays
/// alive for as long as the returned reference is used.
pub unsafe fn borrow_str<'a>(value: *const c_char, name: &str) -> ShimResult<&'a str> {
    if value.is_null() {
        return Err(ShimError::invalid_argument(format!(
            "{name} must not be null"
        )));
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map_err(|_| ShimError::invalid_argument(format!("{name} must be valid UTF-8")))
}

/// Borrow an optional NUL-terminated C string, mapping null to `None`.
///
/// # Safety
///
/// See [`borrow_str`].
pub unsafe fn borrow_opt_str<'a>(value: *const c_char, name: &str) -> ShimResult<Option<&'a str>> {
    if value.is_null() {
        return Ok(None);
    }
    unsafe { borrow_str(value, name) }.map(Some)
}

/// Parse a JSON argument, reporting failures as invalid-argument rather than as
/// a parse failure of the NCBI response.
///
/// # Safety
///
/// See [`borrow_str`].
pub unsafe fn parse_json_arg<T: serde::de::DeserializeOwned>(
    value: *const c_char,
    name: &str,
) -> ShimResult<T> {
    let raw = unsafe { borrow_str(value, name) }?;
    serde_json::from_str(raw)
        .map_err(|e| ShimError::invalid_argument(format!("invalid {name}: {e}")))
}

/// Reset `out_err` to null so a stale message can never be mistaken for a fresh
/// one.
///
/// # Safety
///
/// `out_err` must be null or point to a writable `*mut c_char`.
pub unsafe fn clear_error(out_err: *mut *mut c_char) {
    if !out_err.is_null() {
        unsafe { *out_err = ptr::null_mut() };
    }
}

/// Store an owned error envelope in `out_err`.
///
/// # Safety
///
/// `out_err` must be null or point to a writable `*mut c_char`.
pub unsafe fn set_error(out_err: *mut *mut c_char, error: &ShimError) {
    if out_err.is_null() {
        return;
    }
    let owned = CString::new(error.to_envelope())
        .unwrap_or_else(|_| c"error contained an interior NUL byte".into());
    unsafe { *out_err = owned.into_raw() };
}

/// Run `body` under the FFI error convention: a freshly allocated C string on
/// success, or null with `*out_err` holding the error envelope on failure.
///
/// Panics are caught and converted into errors so they never unwind into Go —
/// an `extern "C"` function that unwinds would abort the whole process.
///
/// Crate-private: it writes through `out_err` without being an `unsafe fn`,
/// which is only sound because every caller is an `unsafe extern "C"` function
/// that already promises the pointer is null or writable.
pub(crate) fn guard<F>(out_err: *mut *mut c_char, body: F) -> *mut c_char
where
    F: FnOnce() -> ShimResult<String>,
{
    unsafe { clear_error(out_err) };

    let outcome = catch_unwind(AssertUnwindSafe(body))
        .unwrap_or_else(|_| Err(ShimError::panicked("panic inside pubmed-client")));

    match outcome.and_then(into_c_string) {
        Ok(value) => value,
        Err(error) => {
            unsafe { set_error(out_err, &error) };
            ptr::null_mut()
        }
    }
}

/// Release a string returned by any call function or written to an `out_err`.
/// Null is a no-op.
///
/// # Safety
///
/// `value` must be a pointer produced by this library and not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pubmed_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    drop(unsafe { CString::from_raw(value) });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    /// Read and free a `*mut c_char` produced by this module.
    fn take(value: *mut c_char) -> String {
        assert!(!value.is_null());
        let owned = unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .to_string();
        unsafe { pubmed_string_free(value) };
        owned
    }

    #[test]
    fn guard_returns_the_body_result_and_clears_out_err() {
        let mut err: *mut c_char = ptr::null_mut();
        let result = guard(&mut err, || Ok("hello".to_string()));
        assert!(err.is_null());
        assert_eq!(take(result), "hello");
    }

    #[test]
    fn guard_reports_errors_as_an_envelope() {
        let mut err: *mut c_char = ptr::null_mut();
        let result = guard(&mut err, || Err(ShimError::invalid_argument("bad")));
        assert!(result.is_null());
        assert_eq!(take(err), r#"{"kind":"invalid_argument","message":"bad"}"#);
    }

    #[test]
    fn guard_converts_panics_into_errors() {
        let mut err: *mut c_char = ptr::null_mut();
        let result = guard(&mut err, || panic!("boom"));
        assert!(result.is_null());
        assert!(take(err).contains("panic"));
    }

    #[test]
    fn guard_clears_a_stale_error_before_running() {
        let mut err: *mut c_char = CString::new("stale").expect("no NUL").into_raw();
        let result = guard(&mut err, || Ok("fresh".to_string()));
        assert!(err.is_null(), "a stale message survived a successful call");
        assert_eq!(take(result), "fresh");
    }

    #[test]
    fn optional_strings_map_null_to_none() {
        assert_eq!(
            unsafe { borrow_opt_str(ptr::null(), "sort") }.expect("null is valid"),
            None
        );
        assert_eq!(
            unsafe { borrow_opt_str(c"pub_date".as_ptr(), "sort") }.expect("valid"),
            Some("pub_date")
        );
    }

    #[test]
    fn json_arguments_fail_as_invalid_argument() {
        let error = unsafe { parse_json_arg::<Vec<String>>(c"[1]".as_ptr(), "pmids_json") }
            .expect_err("an int array is not a string array");
        assert_eq!(error.kind, ErrorKind::InvalidArgument);
        assert!(error.message.contains("pmids_json"), "{}", error.message);
    }

    #[test]
    fn free_tolerates_null() {
        unsafe { pubmed_string_free(ptr::null_mut()) };
    }
}
