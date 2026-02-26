//! Data extraction utilities for PubMed article metadata
//!
//! This module provides helper functions for extracting structured information
//! from unstructured text fields in PubMed XML.

/// Extract email address from affiliation text
///
/// Searches for email patterns (containing '@' and '.') in the text.
///
/// # Arguments
///
/// * `text` - The affiliation text to search
///
/// # Returns
///
/// The first email address found, or `None` if no valid email is detected
///
/// # Example
///
/// ```ignore
/// let text = "Harvard Medical School, Boston, MA, USA. john.doe@hms.harvard.edu";
/// let email = extract_email_from_text(text);
/// assert_eq!(email, Some("john.doe@hms.harvard.edu".to_string()));
/// ```
pub(super) fn extract_email_from_text(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|part| part.contains('@') && part.contains('.'))
        .map(|part| part.trim_end_matches(&['.', ',', ';', ')'][..]).to_string())
        .filter(|email| email.len() > 5)
}

/// Extract country from affiliation text
///
/// Searches for common country names at the end of affiliation strings.
///
/// # Arguments
///
/// * `text` - The affiliation text to search
///
/// # Returns
///
/// The country name if found, or `None` if no known country is detected
///
/// # Implementation Notes
///
/// This uses a predefined list of common countries. It matches countries that:
/// - Appear at the end of the text
/// - Are preceded by a comma and space
///
/// # Example
///
/// ```ignore
/// let text = "Harvard Medical School, Boston, MA, USA";
/// let country = extract_country_from_text(text);
/// assert_eq!(country, Some("USA".to_string()));
/// ```
pub(super) fn extract_country_from_text(text: &str) -> Option<String> {
    const COUNTRIES: &[&str] = &[
        "USA",
        "United States",
        "US",
        "UK",
        "United Kingdom",
        "England",
        "Scotland",
        "Wales",
        "Canada",
        "Australia",
        "Germany",
        "France",
        "Italy",
        "Spain",
        "Japan",
        "China",
        "India",
        "Brazil",
        "Netherlands",
        "Sweden",
        "Switzerland",
        "Denmark",
        "Norway",
        "Finland",
        "Belgium",
        "Austria",
        "Portugal",
        "Ireland",
        "Israel",
        "South Korea",
        "Singapore",
        "Hong Kong",
        "Taiwan",
        "New Zealand",
        "Mexico",
    ];

    let text_lower = text.to_lowercase();
    COUNTRIES.iter().find_map(|&country| {
        let country_lower = country.to_lowercase();
        if text_lower.ends_with(&country_lower)
            || text_lower.contains(&format!(", {}", country_lower))
        {
            Some(country.to_string())
        } else {
            None
        }
    })
}

/// Format an author name from components
///
/// Constructs a full author name from available components, handling missing fields gracefully.
///
/// # Arguments
///
/// * `last_name` - Author's last name/surname
/// * `fore_name` - Author's fore name (given name)
/// * `initials` - Author's initials (used if fore_name is missing)
///
/// # Returns
///
/// A formatted full name string
///
/// # Formatting Rules
///
/// 1. If both fore_name and last_name exist: "ForeNamLast
/// 2. If only last_name exists: "Initials LastName" (if initials available) or "LastName"
/// 3. If only fore_name exists: "ForeName"
/// 4. If neither exists: "Unknown Author"
///
/// # Example
///
/// ```ignore
/// let name = format_author_name(
///     &Some("Smith".to_string()),
///     &Some("John".to_string()),
///     &None
/// );
/// assert_eq!(name, "John Smith");
/// ```
pub(super) fn format_author_name(
    last_name: &Option<String>,
    fore_name: &Option<String>,
    initials: &Option<String>,
) -> String {
    match (fore_name, last_name) {
        (Some(fore), Some(last)) => format!("{fore} {last}"),
        (None, Some(last)) => {
            if let Some(init) = initials {
                format!("{init} {last}")
            } else {
                last.clone()
            }
        }
        (Some(fore), None) => fore.clone(),
        (None, None) => "Unknown Author".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_email_from_text() {
        assert_eq!(
            extract_email_from_text("Contact john.doe@example.com for details"),
            Some("john.doe@example.com".to_string())
        );

        assert_eq!(
            extract_email_from_text("Email: jane.smith@university.edu."),
            Some("jane.smith@university.edu".to_string())
        );

        assert_eq!(extract_email_from_text("No email here"), None);
    }

    #[test]
    fn test_extract_country_from_text() {
        assert_eq!(
            extract_country_from_text("Harvard Medical School, Boston, MA, USA"),
            Some("USA".to_string())
        );

        assert_eq!(
            extract_country_from_text("University of Oxford, Oxford, UK"),
            Some("UK".to_string())
        );

        assert_eq!(extract_country_from_text("Local Institution"), None);
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
    }
}
