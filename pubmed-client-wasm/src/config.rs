use pubmed_client::config::ClientConfig;
use wasm_bindgen::prelude::*;

/// JavaScript-friendly configuration for the PubMed client
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct WasmClientConfig {
    api_key: Option<String>,
    email: Option<String>,
    tool: Option<String>,
    rate_limit: Option<f64>,
    timeout_seconds: Option<u64>,
}

#[wasm_bindgen]
impl WasmClientConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    #[wasm_bindgen(setter)]
    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }

    #[wasm_bindgen(setter)]
    pub fn set_email(&mut self, email: String) {
        self.email = Some(email);
    }

    #[wasm_bindgen(setter)]
    pub fn set_tool(&mut self, tool: String) {
        self.tool = Some(tool);
    }

    #[wasm_bindgen(setter)]
    pub fn set_rate_limit(&mut self, rate_limit: f64) {
        self.rate_limit = Some(rate_limit);
    }

    #[wasm_bindgen(setter)]
    pub fn set_timeout_seconds(&mut self, timeout_seconds: u64) {
        self.timeout_seconds = Some(timeout_seconds);
    }
}

impl From<WasmClientConfig> for ClientConfig {
    fn from(wasm_config: WasmClientConfig) -> Self {
        let mut config = ClientConfig::new();

        if let Some(api_key) = wasm_config.api_key {
            config = config.with_api_key(&api_key);
        }

        if let Some(email) = wasm_config.email {
            config = config.with_email(&email);
        }

        if let Some(tool) = wasm_config.tool {
            config = config.with_tool(&tool);
        }

        if let Some(rate_limit) = wasm_config.rate_limit {
            config = config.with_rate_limit(rate_limit);
        }

        if let Some(timeout_seconds) = wasm_config.timeout_seconds {
            config = config.with_timeout_seconds(timeout_seconds);
        }

        config
    }
}
