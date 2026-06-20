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

    COUNTRIES.iter().find_map(|&country| {
        let clen = country.len();
        // Check if text ends with country (case-insensitive)
        if text.len() >= clen
            && text.is_char_boundary(text.len() - clen)
            && text[text.len() - clen..].eq_ignore_ascii_case(country)
        {
            return Some(country.to_string());
        }
        // Check if ", country" appears anywhere (case-insensitive)
        let mut search_pos = 0;
        while let Some(comma_pos) = text[search_pos..].find(", ") {
            let candidate_start = search_pos + comma_pos + 2;
            let candidate_end = candidate_start + clen;
            if candidate_end <= text.len()
                && text.is_char_boundary(candidate_end)
                && text[candidate_start..candidate_end].eq_ignore_ascii_case(country)
            {
                return Some(country.to_string());
            }
            search_pos = candidate_start;
        }
        None
    })
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
}
