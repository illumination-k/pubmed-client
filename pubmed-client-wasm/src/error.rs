use wasm_bindgen::prelude::*;

// ================================================================================================
// Error Conversion
// ================================================================================================

/// Convert `PubMedError` to a `JsValue` carrying a proper `js_sys::Error` with a
/// `type` property that consumers can switch on (`"ParseError"`, `"RateLimitExceeded"`, etc.).
///
/// The match is exhaustive — adding a new `PubMedError` variant will fail to compile
/// until an explicit mapping is added here.
pub(crate) fn to_js_err(err: pubmed_client::error::PubMedError) -> JsValue {
    use pubmed_client::error::PubMedError;
    let (error_type, message) = match &err {
        PubMedError::ParseError(_) => ("ParseError", err.to_string()),
        PubMedError::RequestError(_) => ("RequestError", err.to_string()),
        PubMedError::InvalidQuery(_) => ("InvalidQuery", err.to_string()),
        PubMedError::RateLimitExceeded => ("RateLimitExceeded", err.to_string()),
        PubMedError::ApiError { .. } => ("ApiError", err.to_string()),
        PubMedError::SearchLimitExceeded { .. } => ("SearchLimitExceeded", err.to_string()),
        PubMedError::HistorySessionError(_) => ("HistorySessionError", err.to_string()),
        PubMedError::WebEnvNotAvailable => ("WebEnvNotAvailable", err.to_string()),
    };
    let js_error = js_sys::Error::new(&message);
    let _ = js_sys::Reflect::set(
        &js_error,
        &JsValue::from_str("type"),
        &JsValue::from_str(error_type),
    );
    js_error.into()
}
