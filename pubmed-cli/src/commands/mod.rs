pub mod citmatch;
pub mod convert;
pub mod figures;
pub mod gquery;
pub mod markdown;
pub mod metadata;
pub mod search;
pub mod storage;

use anyhow::Result;
use pubmed_client::{ClientConfig, PmcClient, PubMedClient};

pub fn create_pmc_client(
    api_key: Option<&str>,
    email: Option<&str>,
    tool: &str,
) -> Result<PmcClient> {
    create_pmc_client_with_timeout(api_key, email, tool, None)
}

pub fn create_pmc_client_with_timeout(
    api_key: Option<&str>,
    email: Option<&str>,
    tool: &str,
    timeout_seconds: Option<u64>,
) -> Result<PmcClient> {
    let mut config = ClientConfig::new().with_tool(tool);

    if let Some(key) = api_key {
        config = config.with_api_key(key);
    }

    if let Some(email) = email {
        config = config.with_email(email);
    }

    if let Some(timeout) = timeout_seconds {
        config = config.with_timeout_seconds(timeout);
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
