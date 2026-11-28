//! Shared helpers for MCP tool implementations

use pubmed_client::{Figure, PmcArticle, PubMedError, Section, Table};
use rmcp::model::{ErrorCode, ErrorData};
use std::borrow::Cow;
use std::fmt::Display;

use super::as_of::not_available;

pub fn internal_error(msg: impl Display) -> ErrorData {
    ErrorData {
        code: ErrorCode(-32603),
        message: Cow::from(msg.to_string()),
        data: None,
    }
}

pub fn invalid_params(msg: impl Display) -> ErrorData {
    ErrorData {
        code: ErrorCode(-32602),
        message: Cow::from(msg.to_string()),
        data: None,
    }
}

pub fn normalize_pmc_id(pmc_id: &str) -> String {
    if pmc_id.starts_with("PMC") {
        pmc_id.to_string()
    } else {
        format!("PMC{}", pmc_id)
    }
}

/// Fetch a PMC article for the full-text tools.
///
/// Under `--as-of` the article is refused unless its PubMed record is
/// visible, with the same error whether the record is hidden, has no PubMed
/// record to date it by, or PMC does not have the article at all.
pub async fn fetch_pmc_article(
    server: &super::PubMedServer,
    pmc_id: &str,
) -> Result<PmcArticle, ErrorData> {
    const CONTEXT: &str = "Failed to fetch PMC article";
    let fetched = server.client.pmc.fetch_full_text(pmc_id).await;

    let Some(as_of) = server.as_of else {
        return fetched.map_err(|e| internal_error(format!("{CONTEXT}: {e}")));
    };
    match fetched {
        Ok(article) if server.pmc_article_is_visible(as_of, &article).await? => Ok(article),
        Ok(_) | Err(PubMedError::ParseError(_)) => Err(not_available(CONTEXT, pmc_id)),
        Err(e) => Err(internal_error(format!("{CONTEXT}: {e}"))),
    }
}

pub fn collect_figures(sections: &[Section]) -> Vec<&Figure> {
    let mut figures = Vec::new();
    for section in sections {
        figures.extend(section.figures.iter());
        figures.extend(collect_figures(&section.subsections));
    }
    figures
}

pub fn collect_tables(sections: &[Section]) -> Vec<&Table> {
    let mut tables = Vec::new();
    for section in sections {
        tables.extend(section.tables.iter());
        tables.extend(collect_tables(&section.subsections));
    }
    tables
}
