use crate::common::PmcId;
use crate::error::{ParseError, Result};
use crate::request::RequestExecutor;

pub(crate) fn normalize_pmcid(pmcid: &str) -> String {
    PmcId::parse(pmcid)
        .map(|id| id.as_str())
        .unwrap_or_else(|_| {
            if pmcid.starts_with("PMC") {
                pmcid.to_string()
            } else {
                format!("PMC{pmcid}")
            }
        })
}

pub(crate) async fn fetch_pmc_xml(
    executor: &RequestExecutor<'_>,
    base_url: &str,
    pmcid: &str,
) -> Result<String> {
    let pmc_id = PmcId::parse(pmcid)?;
    let normalized_pmcid = pmc_id.as_str();
    let numeric_part = pmc_id.numeric_part();

    let id = format!("PMC{numeric_part}");
    let response = executor
        .get_endpoint(
            base_url,
            "efetch.fcgi",
            &[("db", "pmc"), ("id", id.as_str()), ("retmode", "xml")],
        )
        .await?;

    let xml_content = response.text().await?;

    if xml_content.contains("<ERROR>") {
        return Err(ParseError::PmcNotAvailable {
            id: normalized_pmcid,
        }
        .into());
    }

    Ok(xml_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_pmcid() {
        assert_eq!(normalize_pmcid("1234567"), "PMC1234567");
        assert_eq!(normalize_pmcid("PMC1234567"), "PMC1234567");
    }
}
