//! Configuration module for Python bindings
//!
//! This module provides Python wrappers for client configuration.

use pubmed_client::config::ClientConfig;
use pyo3::prelude::*;
use pyo3_stub_gen_derive::{gen_stub_pyclass, gen_stub_pymethods};

// ================================================================================================
// Configuration
// ================================================================================================

/// Python wrapper for ClientConfig
///
/// Configuration for PubMed and PMC clients.
///
/// Examples:
///     >>> config = ClientConfig()
///     >>> config.with_api_key("your_api_key").with_email("you@example.com")
///     >>> client = Client.with_config(config)
#[gen_stub_pyclass]
#[pyclass(name = "ClientConfig")]
#[derive(Clone)]
pub struct PyClientConfig {
    pub inner: ClientConfig,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyClientConfig {
    /// Create a new configuration with default settings
    #[new]
    fn new() -> Self {
        PyClientConfig {
            inner: ClientConfig::new(),
        }
    }

    /// Set the NCBI API key for increased rate limits (10 req/sec instead of 3)
    fn with_api_key(mut slf: PyRefMut<Self>, api_key: String) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_api_key(&api_key);
        slf
    }

    /// Set the email address for identification (recommended by NCBI)
    fn with_email(mut slf: PyRefMut<Self>, email: String) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_email(&email);
        slf
    }

    /// Set the tool name for identification (default: "pubmed-client-py")
    fn with_tool(mut slf: PyRefMut<Self>, tool: String) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_tool(&tool);
        slf
    }

    /// Set custom rate limit in requests per second
    fn with_rate_limit(mut slf: PyRefMut<Self>, rate_limit: f64) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_rate_limit(rate_limit);
        slf
    }

    /// Set HTTP request timeout in seconds
    fn with_timeout_seconds(mut slf: PyRefMut<Self>, timeout_seconds: u64) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_timeout_seconds(timeout_seconds);
        slf
    }

    /// Enable default response caching
    fn with_cache(mut slf: PyRefMut<Self>) -> PyRefMut<Self> {
        slf.inner = slf.inner.clone().with_cache();
        slf
    }

    fn __repr__(&self) -> String {
        "ClientConfig(...)".to_string()
    }
}
