use napi::bindgen_prelude::*;

// ================================================================================================
// Error Conversion
// ================================================================================================

/// Convert `PubMedError` to `napi::Error` with an exhaustive match.
///
/// No wildcard arm — adding a new variant to `PubMedError` will cause a
/// compile error here, forcing an explicit mapping decision.
pub(crate) fn to_napi_err(err: pubmed_client::error::PubMedError) -> Error {
    use pubmed_client::error::PubMedError;
    let reason = match err {
        PubMedError::ParseError(_) => err.to_string(),
        PubMedError::RequestError(_) => err.to_string(),
        PubMedError::InvalidQuery(_) => err.to_string(),
        PubMedError::RateLimitExceeded => err.to_string(),
        PubMedError::ApiError { .. } => err.to_string(),
        PubMedError::SearchLimitExceeded { .. } => err.to_string(),
        PubMedError::HistorySessionError(_) => err.to_string(),
        PubMedError::WebEnvNotAvailable => err.to_string(),
    };
    Error::from_reason(reason)
}
