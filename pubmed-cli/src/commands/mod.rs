pub mod figures;

use anyhow::Result;
use pubmed_client_rs::{ClientConfig, PmcClient};

pub fn create_pmc_client(
    api_key: Option<&str>,
    email: Option<&str>,
    tool: &str,
) -> Result<PmcClient> {
    let mut config = ClientConfig::new().with_tool(tool);

    if let Some(key) = api_key {
        config = config.with_api_key(key);
    }

    if let Some(email) = email {
        config = config.with_email(email);
    }

    let pmc_client = PmcClient::with_config(config);
    Ok(pmc_client)
}
