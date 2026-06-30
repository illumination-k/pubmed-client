//! Europe PMC full-text (JATS XML) retrieval.

use tracing::{info, instrument};

use pubmed_parser::ParseError;
use pubmed_parser::pmc::PmcArticle;
use pubmed_parser::pmc::parser::parse_pmc_xml;

use crate::error::Result;

use super::client::EuropePmcClient;
use super::id::EuropePmcId;

impl EuropePmcClient {
    /// Fetch and parse the full text of a Europe PMC record into a [`PmcArticle`].
    ///
    /// Europe PMC serves full text as JATS XML, which is parsed by the same
    /// parser used for NCBI PMC. Parsing into a [`PmcArticle`] requires a PMC id,
    /// so this method only supports `PMC`-sourced records; for other sources use
    /// [`EuropePmcClient::fetch_full_text_xml`] to get the raw JATS instead.
    ///
    /// Results are cached when a cache is configured (key `epmc-ft:<source>:<id>`).
    ///
    /// # Errors
    ///
    /// * [`ParseError::PmcNotAvailable`] — the record is not PMC-sourced.
    /// * [`crate::PubMedError::ApiError`] — the HTTP request failed.
    #[instrument(skip(self), fields(id = %id))]
    pub async fn fetch_full_text(&self, id: &EuropePmcId) -> Result<PmcArticle> {
        let Some(pmcid) = id.pmcid() else {
            return Err(ParseError::PmcNotAvailable { id: id.to_string() }.into());
        };

        let cache_key = format!("epmc-ft:{}:{}", id.source, id.id);
        if let Some(cache) = &self.cache
            && let Some(cached) = cache.get(&cache_key).await
        {
            info!(id = %id, "Cache hit for Europe PMC full text");
            return Ok(cached);
        }

        let xml = self.fetch_full_text_xml(id).await?;
        let article = parse_pmc_xml(&xml, &pmcid)?;

        if let Some(cache) = &self.cache {
            cache.insert(cache_key, article.clone()).await;
        }

        Ok(article)
    }

    /// Fetch the raw JATS XML full text for a Europe PMC record.
    ///
    /// Works for any source that has full text available. Returns the response
    /// body verbatim (`/{source}/{id}/fullTextXML`).
    #[instrument(skip(self), fields(id = %id))]
    pub async fn fetch_full_text_xml(&self, id: &EuropePmcId) -> Result<String> {
        let endpoint = format!("{}/{}/fullTextXML", id.source, id.id);
        let response = self
            .executor()
            .get_endpoint(&self.base_url, &endpoint, &[])
            .await?;
        Ok(response.text().await?)
    }
}
