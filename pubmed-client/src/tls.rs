//! Process-wide rustls setup for the `rustls-tls` feature.

/// Install ring as the process-wide rustls crypto provider.
///
/// `rustls-tls` selects reqwest's `rustls-no-provider` feature rather than its
/// `rustls` feature, so the aws-lc-rs backend is not pulled in — aws-lc-sys
/// builds C through cmake, which fails on windows-msvc runners. That leaves
/// reqwest without a provider, and it panics with "No provider set" unless one
/// is installed process-wide before the first client is built.
///
/// Idempotent, and safe when another crate has already installed a provider:
/// `install_default` only errors in that case, and any working provider is fine.
#[cfg(all(feature = "rustls-tls", not(target_arch = "wasm32")))]
pub(crate) fn install_default_crypto_provider() {
    use rustls::crypto::ring::default_provider;
    use std::sync::Once;

    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = default_provider().install_default();
    });
}

/// No-op when TLS is handled by native-tls or by the browser (wasm).
#[cfg(not(all(feature = "rustls-tls", not(target_arch = "wasm32"))))]
pub(crate) fn install_default_crypto_provider() {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use crate::europe_pmc::EuropePmcClient;
    use crate::pmc::PmcClient;
    use crate::pubmed::PubMedClient;

    /// Building a client must not panic.
    ///
    /// Under `rustls-tls` reqwest has no built-in provider, so it panics with
    /// "No provider set" if `install_default_crypto_provider` stops running
    /// before the builder. That is invisible at compile time — only
    /// constructing a client catches it.
    #[test]
    fn building_a_client_does_not_panic() {
        let _ = PmcClient::new();
        let _ = PubMedClient::new();
        let _ = EuropePmcClient::new();
    }
}
