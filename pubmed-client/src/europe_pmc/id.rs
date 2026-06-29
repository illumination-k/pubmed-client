//! Source/identifier addressing for Europe PMC records.
//!
//! Europe PMC addresses every record by a `(source, id)` pair, e.g.
//! `MED/12345`, `PMC/PMC3258128`, or `PPR/PPR123456`. These types provide a
//! typed, validated way to construct those addresses for the REST API.

use std::fmt;
use std::str::FromStr;

use crate::common::PmcId;
use crate::error::{PubMedError, Result};

/// A Europe PMC source database.
///
/// The known variants cover the commonly used databases; any unrecognized code
/// is preserved in [`EuropePmcSource::Other`] so new sources never break
/// parsing or addressing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EuropePmcSource {
    /// PubMed / MEDLINE (`MED`).
    Med,
    /// PubMed Central (`PMC`).
    Pmc,
    /// Preprints (`PPR`).
    Ppr,
    /// Agricola (`AGR`).
    Agr,
    /// Chinese Biological Abstracts (`CBA`).
    Cba,
    /// Patents (`PAT`).
    Pat,
    /// NHS Evidence / ETHoS / other recognized-but-uncommon, or any code not
    /// otherwise modelled. Stores the raw uppercase source code.
    Other(String),
}

impl EuropePmcSource {
    /// Return the uppercase source code used by the REST API (e.g. `"MED"`).
    pub fn as_str(&self) -> &str {
        match self {
            EuropePmcSource::Med => "MED",
            EuropePmcSource::Pmc => "PMC",
            EuropePmcSource::Ppr => "PPR",
            EuropePmcSource::Agr => "AGR",
            EuropePmcSource::Cba => "CBA",
            EuropePmcSource::Pat => "PAT",
            EuropePmcSource::Other(code) => code,
        }
    }
}

impl fmt::Display for EuropePmcSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EuropePmcSource {
    // Parsing is in fact infallible (unknown codes map to `Other`), but the
    // crate `Result` alias fixes the error type to `PubMedError`, so we use it
    // for consistency and to satisfy the `absolute_paths` lint.
    type Err = PubMedError;

    fn from_str(s: &str) -> Result<Self> {
        let upper = s.trim().to_ascii_uppercase();
        Ok(match upper.as_str() {
            "MED" => EuropePmcSource::Med,
            "PMC" => EuropePmcSource::Pmc,
            "PPR" => EuropePmcSource::Ppr,
            "AGR" => EuropePmcSource::Agr,
            "CBA" => EuropePmcSource::Cba,
            "PAT" => EuropePmcSource::Pat,
            _ => EuropePmcSource::Other(upper),
        })
    }
}

impl From<&str> for EuropePmcSource {
    fn from(s: &str) -> Self {
        // FromStr never returns Err for a source code.
        s.parse()
            .unwrap_or_else(|_| EuropePmcSource::Other(s.trim().to_ascii_uppercase()))
    }
}

/// A fully-qualified Europe PMC record address: a `(source, id)` pair.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EuropePmcId {
    /// The source database.
    pub source: EuropePmcSource,
    /// The record identifier within that source (e.g. a PMID for `MED`, or a
    /// `PMCnnn` id for `PMC`).
    pub id: String,
}

impl EuropePmcId {
    /// Construct an address from an explicit source and id.
    pub fn new(source: EuropePmcSource, id: impl Into<String>) -> Self {
        Self {
            source,
            id: id.into(),
        }
    }

    /// Construct a `PMC`-sourced address, normalizing the id to `PMCnnn` form.
    ///
    /// Accepts ids with or without the `PMC` prefix.
    ///
    /// # Errors
    ///
    /// Returns an error if the id is not a valid PMC id.
    pub fn pmc(id: &str) -> Result<Self> {
        let pmc_id = PmcId::parse(id)?;
        Ok(Self {
            source: EuropePmcSource::Pmc,
            id: pmc_id.as_str(),
        })
    }

    /// Construct a `MED`-sourced (PubMed) address from a PMID.
    pub fn med(pmid: impl Into<String>) -> Self {
        Self {
            source: EuropePmcSource::Med,
            id: pmid.into(),
        }
    }

    /// Return the PMC id (`PMCnnn`) for this address if it is PMC-sourced.
    pub(crate) fn pmcid(&self) -> Option<String> {
        match self.source {
            EuropePmcSource::Pmc => Some(self.id.clone()),
            _ => None,
        }
    }
}

impl fmt::Display for EuropePmcId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.source, self.id)
    }
}

impl FromStr for EuropePmcId {
    type Err = PubMedError;

    /// Parse a `"SOURCE/ID"` string, e.g. `"PMC/PMC3258128"` or `"MED/12345"`.
    fn from_str(s: &str) -> Result<Self> {
        let (source, id) = s.trim().split_once('/').ok_or_else(|| {
            PubMedError::InvalidQuery(format!(
                "invalid Europe PMC id {s:?}: expected \"SOURCE/ID\" form"
            ))
        })?;
        if id.is_empty() {
            return Err(PubMedError::InvalidQuery(format!(
                "invalid Europe PMC id {s:?}: empty record id"
            )));
        }
        Ok(Self {
            source: EuropePmcSource::from(source),
            id: id.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_roundtrip() {
        assert_eq!(EuropePmcSource::Med.as_str(), "MED");
        assert_eq!(
            "pmc".parse::<EuropePmcSource>().unwrap(),
            EuropePmcSource::Pmc
        );
        assert_eq!(
            "xyz".parse::<EuropePmcSource>().unwrap(),
            EuropePmcSource::Other("XYZ".to_string())
        );
    }

    #[test]
    fn test_pmc_normalizes() {
        let id = EuropePmcId::pmc("3258128").unwrap();
        assert_eq!(id.source, EuropePmcSource::Pmc);
        assert_eq!(id.id, "PMC3258128");
        assert_eq!(id.to_string(), "PMC/PMC3258128");
        assert_eq!(id.pmcid().as_deref(), Some("PMC3258128"));
    }

    #[test]
    fn test_med_has_no_pmcid() {
        let id = EuropePmcId::med("12345");
        assert_eq!(id.to_string(), "MED/12345");
        assert!(id.pmcid().is_none());
    }

    #[test]
    fn test_parse_from_str() {
        let id: EuropePmcId = "PMC/PMC3258128".parse().unwrap();
        assert_eq!(id.source, EuropePmcSource::Pmc);
        assert_eq!(id.id, "PMC3258128");

        let med: EuropePmcId = "MED/12345".parse().unwrap();
        assert_eq!(med.source, EuropePmcSource::Med);

        assert!("nodelimiter".parse::<EuropePmcId>().is_err());
        assert!("PMC/".parse::<EuropePmcId>().is_err());
    }
}
