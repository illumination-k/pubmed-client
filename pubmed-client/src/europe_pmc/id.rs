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

    /// Resolve the `(source, id)` pair a caller addressed.
    ///
    /// Europe PMC identifies every record by a source database plus an id, but
    /// requiring callers to spell both out for the common cases would be
    /// noise, so three forms are accepted:
    ///
    /// * a fully-qualified `"SOURCE/ID"` string (e.g. `"PPR/PPR123456"`),
    ///   which takes precedence over any `source` argument;
    /// * an explicit `source` plus a bare id;
    /// * a bare id alone — a `PMC`-prefixed id implies the `PMC` source,
    ///   anything else is treated as a PubMed (`MED`) record.
    ///
    /// Every language binding routes its own id arguments through this so the
    /// three forms mean the same thing on every surface.
    ///
    /// # Errors
    ///
    /// Returns an error if the id is blank, or if a qualified or `PMC`-sourced
    /// id is malformed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client::{EuropePmcId, EuropePmcSource};
    ///
    /// // A bare PMC id implies the PMC source.
    /// let id = EuropePmcId::resolve("PMC3258128", None)?;
    /// assert_eq!(id.to_string(), "PMC/PMC3258128");
    ///
    /// // A bare non-PMC id is treated as a PubMed record.
    /// let id = EuropePmcId::resolve("33515491", None)?;
    /// assert_eq!(id.to_string(), "MED/33515491");
    ///
    /// // A qualified id wins over the source argument.
    /// let id = EuropePmcId::resolve("PPR/PPR123456", Some("MED"))?;
    /// assert_eq!(id.source, EuropePmcSource::Ppr);
    /// # Ok::<(), pubmed_client::PubMedError>(())
    /// ```
    pub fn resolve(id: &str, source: Option<&str>) -> Result<Self> {
        let id = id.trim();
        if id.is_empty() {
            return Err(PubMedError::InvalidQuery(
                "Europe PMC id must not be empty".to_string(),
            ));
        }

        if id.contains('/') {
            return id.parse();
        }

        let source = match source {
            Some(source) if !source.trim().is_empty() => EuropePmcSource::from(source),
            _ if id.to_ascii_uppercase().starts_with("PMC") => EuropePmcSource::Pmc,
            _ => EuropePmcSource::Med,
        };

        if source == EuropePmcSource::Pmc {
            return Self::pmc(id);
        }

        Ok(Self::new(source, id))
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

    #[test]
    fn test_resolve_bare_pmc_id_defaults_to_pmc_source() {
        let id = EuropePmcId::resolve("PMC3258128", None).unwrap();
        assert_eq!(id.to_string(), "PMC/PMC3258128");
    }

    #[test]
    fn test_resolve_bare_numeric_id_defaults_to_med_source() {
        let id = EuropePmcId::resolve("33515491", None).unwrap();
        assert_eq!(id.to_string(), "MED/33515491");
    }

    #[test]
    fn test_resolve_explicit_pmc_source_normalizes_a_bare_number() {
        assert_eq!(
            EuropePmcId::resolve("3258128", Some("pmc"))
                .unwrap()
                .to_string(),
            "PMC/PMC3258128"
        );
    }

    #[test]
    fn test_resolve_qualified_id_wins_over_source_argument() {
        let id = EuropePmcId::resolve("PPR/PPR123456", Some("MED")).unwrap();
        assert_eq!(id.source, EuropePmcSource::Ppr);
    }

    #[test]
    fn test_resolve_blank_source_falls_back_to_the_bare_id_rule() {
        let id = EuropePmcId::resolve("PMC3258128", Some("   ")).unwrap();
        assert_eq!(id.source, EuropePmcSource::Pmc);
    }

    #[test]
    fn test_resolve_rejects_invalid_ids() {
        assert!(EuropePmcId::resolve("   ", None).is_err());
        assert!(EuropePmcId::resolve("MED/", None).is_err());
    }
}
