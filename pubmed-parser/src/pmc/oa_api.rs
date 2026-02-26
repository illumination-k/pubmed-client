//! PMC Open Access API client
//!
//! This module provides functionality to check if a PMC article is in the OA subset
//! using the NCBI OA Web Service API.
//!
//! The OA subset contains articles with programmatic access to full-text XML.
//! Not all PMC articles are in the OA subset - some publishers restrict programmatic access
//! even though the article may be viewable on the PMC website.

use crate::common::PmcId;
use crate::error::{ParseError, Result};
use crate::pmc::models::OaSubsetInfo;
use quick_xml::de::from_str;
use serde::Deserialize;
use tracing::debug;

// ================================================================================================
// OA API Response Structs (for quick-xml deserialization)
// ================================================================================================

/// Root element of OA API response
#[derive(Debug, Deserialize)]
#[serde(rename = "OA")]
struct OaResponse {
    #[serde(rename = "error")]
    error: Option<OaError>,
    #[serde(rename = "records")]
    records: Option<OaRecords>,
}

/// Error element in OA API response
#[derive(Debug, Deserialize)]
struct OaError {
    #[serde(rename = "@code")]
    code: Option<String>,
    #[serde(rename = "$text")]
    message: String,
}

/// Records container in OA API response
#[derive(Debug, Deserialize)]
struct OaRecords {
    #[serde(rename = "record", default)]
    record: Vec<OaRecord>,
}

/// Individual record in OA API response
#[derive(Debug, Deserialize)]
struct OaRecord {
    #[serde(rename = "@id")]
    _id: Option<String>,
    #[serde(rename = "@citation")]
    citation: Option<String>,
    #[serde(rename = "@license")]
    license: Option<String>,
    #[serde(rename = "@retracted")]
    retracted: Option<String>,
    #[serde(rename = "link")]
    link: Option<OaLink>,
}

/// Link element in OA record
#[derive(Debug, Deserialize)]
struct OaLink {
    #[serde(rename = "@format")]
    format: Option<String>,
    #[serde(rename = "@updated")]
    updated: Option<String>,
    #[serde(rename = "@href")]
    href: Option<String>,
}

// ================================================================================================
// Public API
// ================================================================================================

/// OA API base URL
const OA_API_BASE_URL: &str = "https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi";

/// Build the OA API URL for a given PMC ID
pub fn build_oa_api_url(pmcid: &str) -> Result<String> {
    let pmc_id = PmcId::parse(pmcid)?;
    Ok(format!("{}?id={}", OA_API_BASE_URL, pmc_id.as_str()))
}

/// Parse OA API XML response
///
/// # Arguments
///
/// * `xml` - Raw XML response from OA API
/// * `pmcid` - PMC ID for error reporting
///
/// # Returns
///
/// Returns `Result<OaSubsetInfo>` containing detailed information about OA availability
pub fn parse_oa_response(xml: &str, pmcid: &str) -> Result<OaSubsetInfo> {
    let oa_response: OaResponse = from_str(xml).map_err(|e| {
        debug!(pmcid = %pmcid, error = %e, "Failed to parse OA API response");
        ParseError::XmlError(format!("Failed to parse OA API response: {e}"))
    })?;

    // Check for error response
    if let Some(error) = oa_response.error {
        return Ok(OaSubsetInfo::not_available(
            pmcid.to_string(),
            error.code.unwrap_or_else(|| "unknown".to_string()),
            error.message,
        ));
    }

    // Check for records
    if let Some(records) = oa_response.records {
        if let Some(record) = records.record.into_iter().next() {
            let mut info = OaSubsetInfo::available(pmcid.to_string());

            info.citation = record.citation;
            info.license = record.license;
            info.retracted = record.retracted.is_some_and(|r| r == "yes");

            if let Some(link) = record.link {
                info.download_format = link.format;
                info.updated = link.updated;
                info.download_link = link.href;
            }

            return Ok(info);
        }
    }

    // No error and no records - unexpected format
    debug!(pmcid = %pmcid, "OA API response has no error and no records");
    Ok(OaSubsetInfo::not_available(
        pmcid.to_string(),
        "parseError".to_string(),
        "OA API response has no error and no records".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_oa_api_url() {
        let url = build_oa_api_url("PMC7906746").unwrap();
        assert_eq!(
            url,
            "https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id=PMC7906746"
        );

        let url = build_oa_api_url("7906746").unwrap();
        assert_eq!(
            url,
            "https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id=PMC7906746"
        );
    }

    #[test]
    fn test_parse_oa_response_not_open_access() {
        let xml = r#"<OA><responseDate>2026-01-02 10:45:24</responseDate><request>https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id=PMC8550608</request><error code="idIsNotOpenAccess">identifier 'PMC8550608' is not Open Access</error></OA>"#;

        let result = parse_oa_response(xml, "PMC8550608").unwrap();

        assert!(!result.is_oa_subset);
        assert_eq!(result.pmcid, "PMC8550608");
        assert_eq!(result.error_code, Some("idIsNotOpenAccess".to_string()));
        assert!(result
            .error_message
            .as_ref()
            .unwrap()
            .contains("is not Open Access"));
        assert!(result.download_link.is_none());
    }

    #[test]
    fn test_parse_oa_response_open_access() {
        let xml = r#"<OA><responseDate>2026-01-02 10:45:39</responseDate><request id="PMC7906746">https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi?id=PMC7906746</request><records returned-count="1" total-count="1"><record id="PMC7906746" citation="Lancet. 2021 Jan 27 6-12 February; 397(10273):452-455" license="none" retracted="no"><link format="tgz" updated="2022-12-16 07:10:15" href="ftp://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_package/f1/69/PMC7906746.tar.gz" /></record></records></OA>"#;

        let result = parse_oa_response(xml, "PMC7906746").unwrap();

        assert!(result.is_oa_subset);
        assert_eq!(result.pmcid, "PMC7906746");
        assert_eq!(
            result.citation,
            Some("Lancet. 2021 Jan 27 6-12 February; 397(10273):452-455".to_string())
        );
        assert_eq!(result.license, Some("none".to_string()));
        assert!(!result.retracted);
        assert_eq!(result.download_format, Some("tgz".to_string()));
        assert_eq!(result.updated, Some("2022-12-16 07:10:15".to_string()));
        assert_eq!(
            result.download_link,
            Some(
                "ftp://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_package/f1/69/PMC7906746.tar.gz".to_string()
            )
        );
        assert!(result.error_code.is_none());
    }

    #[test]
    fn test_parse_oa_response_retracted() {
        let xml = r#"<OA><records><record id="PMC1234567" citation="Test" license="cc-by" retracted="yes"><link format="tgz" href="ftp://test.com/file.tar.gz" /></record></records></OA>"#;

        let result = parse_oa_response(xml, "PMC1234567").unwrap();

        assert!(result.is_oa_subset);
        assert!(result.retracted);
        assert_eq!(result.license, Some("cc-by".to_string()));
    }
}
