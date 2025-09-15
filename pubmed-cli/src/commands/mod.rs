pub mod convert;
pub mod figures;
pub mod markdown;
pub mod search;
pub mod storage;

use anyhow::Result;
use pubmed_client_rs::{ClientConfig, PmcClient, PubMedClient};

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

pub fn create_pubmed_client(
    api_key: Option<&str>,
    email: Option<&str>,
    tool: &str,
) -> Result<PubMedClient> {
    let mut config = ClientConfig::new().with_tool(tool);

    if let Some(key) = api_key {
        config = config.with_api_key(key);
    }

    if let Some(email) = email {
        config = config.with_email(email);
    }

    let pubmed_client = PubMedClient::with_config(config);
    Ok(pubmed_client)
}
