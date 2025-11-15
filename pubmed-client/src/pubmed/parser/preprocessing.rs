//! XML preprocessing utilities for PubMed parser
//!
//! This module handles cleaning and preprocessing of XML content before parsing.

use tracing::debug;

/// Strip inline HTML-like formatting tags from XML content
///
/// Handles tags like `<i>`, `<sup>`, `<sub>`, `<b>`, `<u>` that can appear in AbstractText and ArticleTitle.
/// These tags cause parsing issues with quick-xml's serde deserializer.
///
/// This function is public within the crate so it can be reused by PMC parser as well.
///
/// # Arguments
///
/// * `xml` - The XML string to clean
///
/// # Returns
///
/// A cleaned XML string with inline HTML tags removed
///
/// # Example
///
/// ```ignore
/// let xml = "<AbstractText>CO<sup>2</sup> levels</AbstractText>";
/// let cleaned = strip_inline_html_tags(xml);
/// assert_eq!(cleaned, "<AbstractText>CO2 levels</AbstractText>");
/// ```
pub(crate) fn strip_inline_html_tags(xml: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;

    // Regex pattern to match inline HTML tags (both opening and closing)
    // Matches: <i>, </i>, <b>, </b>, <sup>, </sup>, <sub>, </sub>, <u>, </u>, <em>, </em>, <strong>, </strong>
    static INLINE_TAG_REGEX: OnceLock<Regex> = OnceLock::new();
    let re = INLINE_TAG_REGEX.get_or_init(|| {
        Regex::new(r"</?(?:i|b|u|sup|sub|em|strong|italic|bold)>")
            .expect("Failed to compile inline tag regex")
    });

    let cleaned = re.replace_all(xml, "");

    // Log if any tags were stripped
    if cleaned.len() != xml.len() {
        debug!(
            "Stripped inline HTML tags: original {} bytes -> cleaned {} bytes (removed {} bytes)",
            xml.len(),
            cleaned.len(),
            xml.len() - cleaned.len()
        );
    }

    cleaned.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_inline_html_tags() {
        // Test stripping <sup> tags
        let xml_with_sup = r#"<AbstractText>CO<sup>2</sup> levels</AbstractText>"#;
        let cleaned = strip_inline_html_tags(xml_with_sup);
        assert!(
            !cleaned.contains("<sup>"),
            "Cleaned XML still contains <sup>: {}",
            cleaned
        );
        assert!(
            !cleaned.contains("</sup>"),
            "Cleaned XML still contains </sup>: {}",
            cleaned
        );
        assert!(cleaned.contains("CO2 levels"));

        // Test stripping <i> tags
        let xml_with_i = r#"<AbstractText>The <i>e.g.</i> example</AbstractText>"#;
        let cleaned = strip_inline_html_tags(xml_with_i);
        assert!(!cleaned.contains("<i>"));
        assert!(!cleaned.contains("</i>"));
        assert!(cleaned.contains("e.g."));

        // Test stripping <sub> tags
        let xml_with_sub = r#"<AbstractText>H<sub>2</sub>O</AbstractText>"#;
        let cleaned = strip_inline_html_tags(xml_with_sub);
        assert!(!cleaned.contains("<sub>"));
        assert!(!cleaned.contains("</sub>"));
        assert!(cleaned.contains("H2O"));

        // Test preserving other tags
        let xml_with_mixed = r#"<Article><Title>CO<sup>2</sup> Study</Title></Article>"#;
        let cleaned = strip_inline_html_tags(xml_with_mixed);
        assert!(cleaned.contains("<Article>"));
        assert!(cleaned.contains("</Article>"));
        assert!(cleaned.contains("<Title>"));
        assert!(!cleaned.contains("<sup>"));
    }
}
