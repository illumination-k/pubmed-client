use clap::ValueEnum;
use pubmed_client::{ClientConfig, PmcClient, PubMedClient};

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    #[value(alias = "txt")]
    Text,
    Json,
    Csv,
    Table,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Json => write!(f, "json"),
            Self::Csv => write!(f, "csv"),
            Self::Table => write!(f, "table"),
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum CitationFormat {
    #[value(alias = "bib")]
    Bibtex,
    Ris,
    #[value(alias = "csl")]
    CslJson,
    #[value(alias = "medline")]
    Nbib,
}

impl std::fmt::Display for CitationFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bibtex => write!(f, "bibtex"),
            Self::Ris => write!(f, "ris"),
            Self::CslJson => write!(f, "csl-json"),
            Self::Nbib => write!(f, "nbib"),
        }
    }
}

pub struct ClientContext<'a> {
    pub api_key: Option<&'a str>,
    pub email: Option<&'a str>,
    pub tool: &'a str,
}

impl ClientContext<'_> {
    pub fn build_config(&self) -> ClientConfig {
        let mut config = ClientConfig::new().with_tool(self.tool);
        if let Some(key) = self.api_key {
            config = config.with_api_key(key);
        }
        if let Some(email) = self.email {
            config = config.with_email(email);
        }
        config
    }

    pub fn pubmed_client(&self) -> PubMedClient {
        PubMedClient::with_config(self.build_config())
    }

    pub fn pmc_client(&self) -> PmcClient {
        PmcClient::with_config(self.build_config())
    }

    pub fn pmc_client_with_timeout(&self, timeout_seconds: Option<u64>) -> PmcClient {
        let mut config = self.build_config();
        if let Some(timeout) = timeout_seconds {
            config = config.with_timeout_seconds(timeout);
        }
        PmcClient::with_config(config)
    }
}
