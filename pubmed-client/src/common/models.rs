//! Common data models shared across PubMed and PMC modules
//!
//! This module provides unified data structures for authors and affiliations
//! that are used consistently across both PubMed metadata and PMC full-text content.

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::Chars;

/// Represents an author's institutional affiliation
///
/// This structure is used across both PubMed and PMC to represent
/// institutional affiliations in a consistent way.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Affiliation {
    /// Affiliation ID (optional, commonly used in PMC XML)
    pub id: Option<String>,
    /// Institution name (e.g., "Harvard Medical School")
    pub institution: Option<String>,
    /// Department or division (e.g., "Department of Medicine")
    pub department: Option<String>,
    /// Full address including street, city, state/province
    pub address: Option<String>,
    /// Country
    pub country: Option<String>,
}

/// Represents a detailed author with enhanced metadata
///
/// This structure provides a unified representation of author information
/// across PubMed and PMC, consolidating various name formats and metadata.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Author {
    /// Author's surname (last name)
    pub surname: Option<String>,
    /// Author's given names (first name, middle names)
    pub given_names: Option<String>,
    /// Author's initials (useful when given_names not available)
    pub initials: Option<String>,
    /// Name suffix (e.g., "Jr", "Sr", "III")
    pub suffix: Option<String>,
    /// Full formatted name
    pub full_name: String,
    /// List of institutional affiliations
    pub affiliations: Vec<Affiliation>,
    /// ORCID identifier (e.g., "0000-0000-0000-0000")
    pub orcid: Option<String>,
    /// Author's email address
    pub email: Option<String>,
    /// Whether this author is a corresponding author
    pub is_corresponding: bool,
    /// Author's roles/contributions (e.g., ["Conceptualization", "Writing - original draft"])
    pub roles: Vec<String>,
}

impl Display for Author {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.full_name)
    }
}

impl PartialEq<str> for Author {
    fn eq(&self, other: &str) -> bool {
        self.full_name == other
    }
}

impl PartialEq<&str> for Author {
    fn eq(&self, other: &&str) -> bool {
        self.full_name == *other
    }
}

impl Author {
    /// Create a new Author with basic information
    pub fn new(surname: Option<String>, given_names: Option<String>) -> Self {
        let full_name = format_author_name(&surname, &given_names, &None);
        Author {
            surname,
            given_names,
            initials: None,
            suffix: None,
            full_name,
            affiliations: Vec::new(),
            orcid: None,
            email: None,
            is_corresponding: false,
            roles: Vec::new(),
        }
    }

    /// Create an author from separate name components
    pub fn with_names(surname: Option<String>, given_names: Option<String>) -> Self {
        Self::new(surname, given_names)
    }

    /// Create an author from a full name string
    ///
    /// This is a convenience method for when you have a complete name
    /// but don't need to separate it into surname and given names.
    pub fn from_full_name(full_name: String) -> Self {
        Author {
            surname: None,
            given_names: None,
            initials: None,
            suffix: None,
            full_name,
            affiliations: Vec::new(),
            orcid: None,
            email: None,
            is_corresponding: false,
            roles: Vec::new(),
        }
    }

    /// Check if the author is affiliated with a specific institution
    ///
    /// # Arguments
    ///
    /// * `institution` - Institution name to check (case-insensitive)
    ///
    /// # Returns
    ///
    /// `true` if the author has an affiliation matching the institution
    pub fn is_affiliated_with(&self, institution: &str) -> bool {
        let institution_lower = institution.to_lowercase();
        self.affiliations.iter().any(|affil| {
            affil
                .institution
                .as_ref()
                .is_some_and(|inst| inst.to_lowercase().contains(&institution_lower))
        })
    }

    /// Get the author's primary affiliation (first in the list)
    ///
    /// # Returns
    ///
    /// A reference to the primary affiliation, if any
    pub fn primary_affiliation(&self) -> Option<&Affiliation> {
        self.affiliations.first()
    }

    /// Check if the author has an ORCID identifier
    ///
    /// # Returns
    ///
    /// `true` if the author has an ORCID ID
    pub fn has_orcid(&self) -> bool {
        self.orcid.is_some()
    }

    /// Check if the author name is empty
    ///
    /// # Returns
    ///
    /// `true` if the full name is empty or just whitespace
    pub fn is_empty(&self) -> bool {
        self.full_name.trim().is_empty()
    }

    /// Get the length of the author's full name
    ///
    /// # Returns
    ///
    /// Length of the full name string
    pub fn len(&self) -> usize {
        self.full_name.len()
    }

    /// Get an iterator over the characters in the author's full name
    ///
    /// # Returns
    ///
    /// Iterator over characters
    pub fn chars(&self) -> Chars<'_> {
        self.full_name.chars()
    }
}

impl Affiliation {
    /// Create a new Affiliation instance
    pub fn new(institution: Option<String>) -> Self {
        Self {
            id: None,
            institution,
            department: None,
            address: None,
            country: None,
        }
    }
}

/// Format an author name from components
///
/// # Arguments
///
/// * `surname` - Author's surname (last name)
/// * `given_names` - Author's given names (first, middle)
/// * `initials` - Author's initials (used if given_names is missing)
///
/// # Returns
///
/// Formatted full name following these rules:
/// 1. If both given_names and surname exist: "GivenNames Surname"
/// 2. If only surname exists: "Initials Surname" (if initials available) or "Surname"
/// 3. If only given_names exists: "GivenNames"
/// 4. If neither exists: "Unknown Author"
pub fn format_author_name(
    surname: &Option<String>,
    given_names: &Option<String>,
    initials: &Option<String>,
) -> String {
    match (given_names, surname) {
        (Some(given), Some(sur)) => format!("{given} {sur}"),
        (None, Some(sur)) => {
            if let Some(init) = initials {
                format!("{init} {sur}")
            } else {
                sur.clone()
            }
        }
        (Some(given), None) => given.clone(),
        (None, None) => "Unknown Author".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_author_creation() {
        let author = Author::new(Some("Smith".to_string()), Some("Jane".to_string()));
        assert_eq!(author.surname, Some("Smith".to_string()));
        assert_eq!(author.given_names, Some("Jane".to_string()));
        assert_eq!(author.full_name, "Jane Smith");
        assert!(!author.has_orcid());
        assert!(!author.is_corresponding);
    }

    #[test]
    fn test_author_with_names() {
        let author = Author::with_names(Some("Doe".to_string()), Some("John".to_string()));
        assert_eq!(author.full_name, "John Doe");
    }

    #[test]
    fn test_author_affiliations() {
        let mut author = Author::new(Some("Doe".to_string()), Some("John".to_string()));
        author.affiliations.push(Affiliation {
            id: None,
            institution: Some("Harvard Medical School".to_string()),
            department: Some("Department of Medicine".to_string()),
            address: Some("Boston, MA".to_string()),
            country: Some("USA".to_string()),
        });

        assert!(author.is_affiliated_with("Harvard"));
        assert!(!author.is_affiliated_with("Stanford"));

        let primary = author.primary_affiliation().unwrap();
        assert_eq!(
            primary.institution,
            Some("Harvard Medical School".to_string())
        );
    }

    #[test]
    fn test_format_author_name() {
        assert_eq!(
            format_author_name(&Some("Smith".to_string()), &Some("John".to_string()), &None),
            "John Smith"
        );

        assert_eq!(
            format_author_name(&Some("Doe".to_string()), &None, &Some("J".to_string())),
            "J Doe"
        );

        assert_eq!(
            format_author_name(&Some("Johnson".to_string()), &None, &None),
            "Johnson"
        );

        assert_eq!(
            format_author_name(&None, &Some("Jane".to_string()), &None),
            "Jane"
        );

        assert_eq!(format_author_name(&None, &None, &None), "Unknown Author");
    }

    #[test]
    fn test_affiliation_creation() {
        let affil = Affiliation::new(Some("MIT".to_string()));
        assert_eq!(affil.institution, Some("MIT".to_string()));
        assert_eq!(affil.id, None);
        assert_eq!(affil.department, None);
    }
}
