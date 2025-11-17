//! ID validation and cleaning utilities for PubMed and PMC identifiers
//!
//! This module provides strongly-typed, validated ID types for PubMed IDs (PMIDs)
//! and PubMed Central IDs (PMC IDs).

use crate::error::{PubMedError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A validated PubMed ID (PMID)
///
/// PMIDs are numeric identifiers for articles in the PubMed database.
/// This type ensures that the ID is valid and provides methods for
/// parsing, cleaning, and converting between different representations.
///
/// # Examples
///
/// ```
/// use pubmed_client_rs::common::PubMedId;
///
/// // Parse from string
/// let pmid = PubMedId::parse("31978945").unwrap();
/// assert_eq!(pmid.as_u32(), 31978945);
/// assert_eq!(pmid.as_str(), "31978945");
///
/// // Parse with whitespace (automatically cleaned)
/// let pmid = PubMedId::parse("  31978945  ").unwrap();
/// assert_eq!(pmid.as_u32(), 31978945);
///
/// // From u32
/// let pmid = PubMedId::from_u32(31978945);
/// assert_eq!(pmid.to_string(), "31978945");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PubMedId {
    value: u32,
}

impl PubMedId {
    /// Parse a PMID from a string
    ///
    /// The input is automatically trimmed of whitespace.
    ///
    /// # Errors
    ///
    /// Returns `PubMedError::InvalidPmid` if:
    /// - The string is empty after trimming
    /// - The string contains non-numeric characters
    /// - The number is zero
    /// - The number is too large to fit in a u32
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PubMedId;
    ///
    /// let pmid = PubMedId::parse("31978945").unwrap();
    /// assert_eq!(pmid.as_u32(), 31978945);
    ///
    /// // With whitespace
    /// let pmid = PubMedId::parse("  31978945  ").unwrap();
    /// assert_eq!(pmid.as_u32(), 31978945);
    ///
    /// // Invalid cases
    /// assert!(PubMedId::parse("").is_err());
    /// assert!(PubMedId::parse("abc").is_err());
    /// assert!(PubMedId::parse("0").is_err());
    /// assert!(PubMedId::parse("-123").is_err());
    /// ```
    pub fn parse(s: &str) -> Result<Self> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err(PubMedError::InvalidPmid {
                pmid: s.to_string(),
            });
        }

        // Parse as u32
        let value = trimmed
            .parse::<u32>()
            .map_err(|_| PubMedError::InvalidPmid {
                pmid: s.to_string(),
            })?;

        // PMIDs should be positive (non-zero)
        if value == 0 {
            return Err(PubMedError::InvalidPmid {
                pmid: s.to_string(),
            });
        }

        Ok(Self { value })
    }

    /// Create a PubMedId from a u32 value
    ///
    /// # Panics
    ///
    /// Panics if the value is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PubMedId;
    ///
    /// let pmid = PubMedId::from_u32(31978945);
    /// assert_eq!(pmid.as_u32(), 31978945);
    /// ```
    pub fn from_u32(value: u32) -> Self {
        assert!(value > 0, "PMID must be greater than zero");
        Self { value }
    }

    /// Try to create a PubMedId from a u32 value
    ///
    /// # Errors
    ///
    /// Returns `PubMedError::InvalidPmid` if the value is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PubMedId;
    ///
    /// let pmid = PubMedId::try_from_u32(31978945).unwrap();
    /// assert_eq!(pmid.as_u32(), 31978945);
    ///
    /// assert!(PubMedId::try_from_u32(0).is_err());
    /// ```
    pub fn try_from_u32(value: u32) -> Result<Self> {
        if value == 0 {
            return Err(PubMedError::InvalidPmid {
                pmid: value.to_string(),
            });
        }
        Ok(Self { value })
    }

    /// Get the PMID as a u32
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PubMedId;
    ///
    /// let pmid = PubMedId::parse("31978945").unwrap();
    /// assert_eq!(pmid.as_u32(), 31978945);
    /// ```
    pub fn as_u32(&self) -> u32 {
        self.value
    }

    /// Get the PMID as a string slice
    ///
    /// Note: This creates a temporary String and returns it.
    /// For owned String, use `to_string()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PubMedId;
    ///
    /// let pmid = PubMedId::from_u32(31978945);
    /// assert_eq!(pmid.as_str(), "31978945");
    /// ```
    pub fn as_str(&self) -> String {
        self.value.to_string()
    }
}

impl fmt::Display for PubMedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl FromStr for PubMedId {
    type Err = PubMedError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl From<u32> for PubMedId {
    fn from(value: u32) -> Self {
        Self::from_u32(value)
    }
}

impl From<PubMedId> for u32 {
    fn from(pmid: PubMedId) -> Self {
        pmid.value
    }
}

impl From<&PubMedId> for u32 {
    fn from(pmid: &PubMedId) -> Self {
        pmid.value
    }
}

/// A validated PubMed Central ID (PMC ID)
///
/// PMC IDs are identifiers for full-text articles in the PMC database.
/// They consist of the prefix "PMC" followed by numeric digits.
/// This type ensures that the ID is valid and provides methods for
/// parsing, cleaning, and normalizing the ID format.
///
/// # Examples
///
/// ```
/// use pubmed_client_rs::common::PmcId;
///
/// // Parse with PMC prefix
/// let pmcid = PmcId::parse("PMC7906746").unwrap();
/// assert_eq!(pmcid.as_str(), "PMC7906746");
/// assert_eq!(pmcid.numeric_part(), 7906746);
///
/// // Parse without PMC prefix (automatically added)
/// let pmcid = PmcId::parse("7906746").unwrap();
/// assert_eq!(pmcid.as_str(), "PMC7906746");
///
/// // With whitespace (automatically cleaned)
/// let pmcid = PmcId::parse("  PMC7906746  ").unwrap();
/// assert_eq!(pmcid.as_str(), "PMC7906746");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PmcId {
    value: u32,
}

impl PmcId {
    /// Parse a PMC ID from a string
    ///
    /// The input is automatically trimmed of whitespace and the "PMC" prefix
    /// is added if not present. Case-insensitive parsing is supported.
    ///
    /// # Errors
    ///
    /// Returns `PubMedError::InvalidPmcid` if:
    /// - The string is empty after trimming
    /// - The numeric part contains non-numeric characters
    /// - The numeric part is zero
    /// - The number is too large to fit in a u32
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PmcId;
    ///
    /// // With PMC prefix
    /// let pmcid = PmcId::parse("PMC7906746").unwrap();
    /// assert_eq!(pmcid.as_str(), "PMC7906746");
    ///
    /// // Without PMC prefix
    /// let pmcid = PmcId::parse("7906746").unwrap();
    /// assert_eq!(pmcid.as_str(), "PMC7906746");
    ///
    /// // Case insensitive
    /// let pmcid = PmcId::parse("pmc7906746").unwrap();
    /// assert_eq!(pmcid.as_str(), "PMC7906746");
    ///
    /// // With whitespace
    /// let pmcid = PmcId::parse("  PMC7906746  ").unwrap();
    /// assert_eq!(pmcid.as_str(), "PMC7906746");
    ///
    /// // Invalid cases
    /// assert!(PmcId::parse("").is_err());
    /// assert!(PmcId::parse("PMC").is_err());
    /// assert!(PmcId::parse("PMC0").is_err());
    /// assert!(PmcId::parse("PMCabc").is_err());
    /// ```
    pub fn parse(s: &str) -> Result<Self> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err(PubMedError::InvalidPmcid {
                pmcid: s.to_string(),
            });
        }

        // Remove PMC prefix if present (case-insensitive)
        let numeric_part = if trimmed.len() >= 3 && trimmed[0..3].eq_ignore_ascii_case("PMC") {
            &trimmed[3..]
        } else {
            trimmed
        };

        // Check if numeric part is empty
        if numeric_part.is_empty() {
            return Err(PubMedError::InvalidPmcid {
                pmcid: s.to_string(),
            });
        }

        // Parse numeric part as u32
        let value = numeric_part
            .parse::<u32>()
            .map_err(|_| PubMedError::InvalidPmcid {
                pmcid: s.to_string(),
            })?;

        // PMC IDs should be positive (non-zero)
        if value == 0 {
            return Err(PubMedError::InvalidPmcid {
                pmcid: s.to_string(),
            });
        }

        Ok(Self { value })
    }

    /// Create a PmcId from a u32 value
    ///
    /// # Panics
    ///
    /// Panics if the value is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PmcId;
    ///
    /// let pmcid = PmcId::from_u32(7906746);
    /// assert_eq!(pmcid.as_str(), "PMC7906746");
    /// assert_eq!(pmcid.numeric_part(), 7906746);
    /// ```
    pub fn from_u32(value: u32) -> Self {
        assert!(value > 0, "PMC ID numeric part must be greater than zero");
        Self { value }
    }

    /// Try to create a PmcId from a u32 value
    ///
    /// # Errors
    ///
    /// Returns `PubMedError::InvalidPmcid` if the value is zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PmcId;
    ///
    /// let pmcid = PmcId::try_from_u32(7906746).unwrap();
    /// assert_eq!(pmcid.numeric_part(), 7906746);
    ///
    /// assert!(PmcId::try_from_u32(0).is_err());
    /// ```
    pub fn try_from_u32(value: u32) -> Result<Self> {
        if value == 0 {
            return Err(PubMedError::InvalidPmcid {
                pmcid: value.to_string(),
            });
        }
        Ok(Self { value })
    }

    /// Get the full PMC ID as a string (with "PMC" prefix)
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PmcId;
    ///
    /// let pmcid = PmcId::from_u32(7906746);
    /// assert_eq!(pmcid.as_str(), "PMC7906746");
    /// ```
    pub fn as_str(&self) -> String {
        format!("PMC{}", self.value)
    }

    /// Get the numeric part of the PMC ID (without "PMC" prefix)
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PmcId;
    ///
    /// let pmcid = PmcId::parse("PMC7906746").unwrap();
    /// assert_eq!(pmcid.numeric_part(), 7906746);
    /// ```
    pub fn numeric_part(&self) -> u32 {
        self.value
    }

    /// Get the numeric part as a string (without "PMC" prefix)
    ///
    /// # Examples
    ///
    /// ```
    /// use pubmed_client_rs::common::PmcId;
    ///
    /// let pmcid = PmcId::parse("PMC7906746").unwrap();
    /// assert_eq!(pmcid.numeric_part_str(), "7906746");
    /// ```
    pub fn numeric_part_str(&self) -> String {
        self.value.to_string()
    }
}

impl fmt::Display for PmcId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PMC{}", self.value)
    }
}

impl FromStr for PmcId {
    type Err = PubMedError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl From<u32> for PmcId {
    fn from(value: u32) -> Self {
        Self::from_u32(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // PubMedId Tests

    #[test]
    fn test_pubmedid_parse_valid() {
        let pmid = PubMedId::parse("31978945").unwrap();
        assert_eq!(pmid.as_u32(), 31978945);
        assert_eq!(pmid.as_str(), "31978945");
    }

    #[test]
    fn test_pubmedid_parse_with_whitespace() {
        let pmid = PubMedId::parse("  31978945  ").unwrap();
        assert_eq!(pmid.as_u32(), 31978945);
    }

    #[test]
    fn test_pubmedid_parse_empty() {
        assert!(PubMedId::parse("").is_err());
        assert!(PubMedId::parse("   ").is_err());
    }

    #[test]
    fn test_pubmedid_parse_non_numeric() {
        assert!(PubMedId::parse("abc").is_err());
        assert!(PubMedId::parse("123abc").is_err());
        assert!(PubMedId::parse("12.34").is_err());
    }

    #[test]
    fn test_pubmedid_parse_zero() {
        assert!(PubMedId::parse("0").is_err());
    }

    #[test]
    fn test_pubmedid_parse_negative() {
        assert!(PubMedId::parse("-123").is_err());
    }

    #[test]
    fn test_pubmedid_from_u32() {
        let pmid = PubMedId::from_u32(31978945);
        assert_eq!(pmid.as_u32(), 31978945);
    }

    #[test]
    #[should_panic(expected = "PMID must be greater than zero")]
    fn test_pubmedid_from_u32_zero_panics() {
        PubMedId::from_u32(0);
    }

    #[test]
    fn test_pubmedid_try_from_u32() {
        let pmid = PubMedId::try_from_u32(31978945).unwrap();
        assert_eq!(pmid.as_u32(), 31978945);
        assert!(PubMedId::try_from_u32(0).is_err());
    }

    #[test]
    fn test_pubmedid_display() {
        let pmid = PubMedId::from_u32(31978945);
        assert_eq!(format!("{}", pmid), "31978945");
    }

    #[test]
    fn test_pubmedid_from_str_trait() {
        let pmid: PubMedId = "31978945".parse().unwrap();
        assert_eq!(pmid.as_u32(), 31978945);
    }

    #[test]
    fn test_pubmedid_conversions() {
        let pmid = PubMedId::from_u32(31978945);
        let value: u32 = pmid.clone().into();
        assert_eq!(value, 31978945);

        let value: u32 = (&pmid).into();
        assert_eq!(value, 31978945);
    }

    // PmcId Tests

    #[test]
    fn test_pmcid_parse_with_prefix() {
        let pmcid = PmcId::parse("PMC7906746").unwrap();
        assert_eq!(pmcid.as_str(), "PMC7906746");
        assert_eq!(pmcid.numeric_part(), 7906746);
    }

    #[test]
    fn test_pmcid_parse_without_prefix() {
        let pmcid = PmcId::parse("7906746").unwrap();
        assert_eq!(pmcid.as_str(), "PMC7906746");
        assert_eq!(pmcid.numeric_part(), 7906746);
    }

    #[test]
    fn test_pmcid_parse_case_insensitive() {
        let pmcid1 = PmcId::parse("pmc7906746").unwrap();
        let pmcid2 = PmcId::parse("Pmc7906746").unwrap();
        let pmcid3 = PmcId::parse("PMC7906746").unwrap();

        assert_eq!(pmcid1, pmcid2);
        assert_eq!(pmcid2, pmcid3);
        assert_eq!(pmcid1.as_str(), "PMC7906746");
    }

    #[test]
    fn test_pmcid_parse_with_whitespace() {
        let pmcid = PmcId::parse("  PMC7906746  ").unwrap();
        assert_eq!(pmcid.as_str(), "PMC7906746");

        let pmcid = PmcId::parse("  7906746  ").unwrap();
        assert_eq!(pmcid.as_str(), "PMC7906746");
    }

    #[test]
    fn test_pmcid_parse_empty() {
        assert!(PmcId::parse("").is_err());
        assert!(PmcId::parse("   ").is_err());
        assert!(PmcId::parse("PMC").is_err());
    }

    #[test]
    fn test_pmcid_parse_non_numeric() {
        assert!(PmcId::parse("PMCabc").is_err());
        assert!(PmcId::parse("PMC123abc").is_err());
        assert!(PmcId::parse("abc").is_err());
    }

    #[test]
    fn test_pmcid_parse_zero() {
        assert!(PmcId::parse("PMC0").is_err());
        assert!(PmcId::parse("0").is_err());
    }

    #[test]
    fn test_pmcid_from_u32() {
        let pmcid = PmcId::from_u32(7906746);
        assert_eq!(pmcid.as_str(), "PMC7906746");
        assert_eq!(pmcid.numeric_part(), 7906746);
    }

    #[test]
    #[should_panic(expected = "PMC ID numeric part must be greater than zero")]
    fn test_pmcid_from_u32_zero_panics() {
        PmcId::from_u32(0);
    }

    #[test]
    fn test_pmcid_try_from_u32() {
        let pmcid = PmcId::try_from_u32(7906746).unwrap();
        assert_eq!(pmcid.numeric_part(), 7906746);
        assert!(PmcId::try_from_u32(0).is_err());
    }

    #[test]
    fn test_pmcid_numeric_part_str() {
        let pmcid = PmcId::parse("PMC7906746").unwrap();
        assert_eq!(pmcid.numeric_part_str(), "7906746");
    }

    #[test]
    fn test_pmcid_display() {
        let pmcid = PmcId::from_u32(7906746);
        assert_eq!(format!("{}", pmcid), "PMC7906746");
    }

    #[test]
    fn test_pmcid_from_str_trait() {
        let pmcid: PmcId = "PMC7906746".parse().unwrap();
        assert_eq!(pmcid.numeric_part(), 7906746);

        let pmcid: PmcId = "7906746".parse().unwrap();
        assert_eq!(pmcid.as_str(), "PMC7906746");
    }

    #[test]
    fn test_pmcid_equality() {
        let pmcid1 = PmcId::parse("PMC7906746").unwrap();
        let pmcid2 = PmcId::parse("7906746").unwrap();
        let pmcid3 = PmcId::from_u32(7906746);

        assert_eq!(pmcid1, pmcid2);
        assert_eq!(pmcid2, pmcid3);
    }

    #[test]
    fn test_pmcid_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(PmcId::parse("PMC7906746").unwrap());
        set.insert(PmcId::parse("7906746").unwrap());

        // Should only contain one item since they're equal
        assert_eq!(set.len(), 1);
    }

    // Real-world examples from the codebase

    #[test]
    fn test_real_world_pmids() {
        let test_cases = vec![
            "31978945", // COVID-19 research
            "25760099", // CRISPR-Cas9
            "33515491", // Cancer treatment
            "12345678",
        ];

        for pmid_str in test_cases {
            let pmid = PubMedId::parse(pmid_str).unwrap();
            assert_eq!(pmid.as_str(), pmid_str);
        }
    }

    #[test]
    fn test_real_world_pmcids() {
        let test_cases = vec![
            ("PMC7906746", "PMC7906746"),
            ("PMC10618641", "PMC10618641"),
            ("PMC10000000", "PMC10000000"),
            ("7906746", "PMC7906746"),   // Without prefix
            ("10618641", "PMC10618641"), // Without prefix
        ];

        for (input, expected) in test_cases {
            let pmcid = PmcId::parse(input).unwrap();
            assert_eq!(pmcid.as_str(), expected);
        }
    }

    #[test]
    fn test_serialization() {
        let pmid = PubMedId::from_u32(31978945);
        let json = serde_json::to_string(&pmid).unwrap();
        let deserialized: PubMedId = serde_json::from_str(&json).unwrap();
        assert_eq!(pmid, deserialized);

        let pmcid = PmcId::from_u32(7906746);
        let json = serde_json::to_string(&pmcid).unwrap();
        let deserialized: PmcId = serde_json::from_str(&json).unwrap();
        assert_eq!(pmcid, deserialized);
    }
}
