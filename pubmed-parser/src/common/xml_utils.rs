//! Common XML parsing utilities shared between PubMed and PMC parsers
//!
//! This module provides reusable XML parsing functions for both string-based
//! and serde-based XML parsing workflows.

use std::borrow::Cow;
use tracing::debug;

/// Strip inline HTML-like formatting tags from XML content
///
/// Handles tags like `<i>`, `<sup>`, `<sub>`, `<b>`, `<u>` that can appear in AbstractText and ArticleTitle.
/// These tags cause parsing issues with quick-xml's serde deserializer.
///
/// This function is used by both PubMed and PMC parsers to clean XML before parsing.
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
/// use pubmed_parser::common::xml_utils::strip_inline_html_tags;
///
/// let xml = "<AbstractText>CO<sup>2</sup> levels</AbstractText>";
/// let cleaned = strip_inline_html_tags(xml);
/// assert_eq!(cleaned, "<AbstractText>CO2 levels</AbstractText>");
/// ```
pub fn strip_inline_html_tags(xml: &str) -> Cow<'_, str> {
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
    if let Cow::Owned(ref _s) = cleaned {
        debug!(
            "Stripped inline HTML tags: original {} bytes -> cleaned {} bytes (removed {} bytes)",
            xml.len(),
            cleaned.len(),
            xml.len() - cleaned.len()
        );
    }

    cleaned
}

/// Strip XML tags from content
///
/// Removes all XML tags, leaving only text content.
///
/// # Arguments
///
/// * `content` - The XML content to strip
///
/// # Returns
///
/// A string with all XML tags removed
pub fn strip_xml_tags(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut in_tag = false;

    for &b in bytes {
        match b {
            b'<' => in_tag = true,
            b'>' => in_tag = false,
            _ if !in_tag => result.push(b),
            _ => {}
        }
    }

    // SAFETY: Input is valid UTF-8 and we only remove complete XML tags
    // (ASCII byte sequences between '<' and '>'). Since '<' and '>' are single-byte
    // ASCII and never appear as UTF-8 continuation bytes, this preserves valid UTF-8.
    let s = unsafe { String::from_utf8_unchecked(result) };

    // Trim in-place without re-allocating
    let trimmed = s.trim();
    if trimmed.len() == s.len() {
        s
    } else {
        trimmed.to_string()
    }
}

/// Decode XML character entities in a string
///
/// Decodes both named entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`)
/// and numeric entities (`&#169;`, `&#x00A9;`).
pub fn decode_xml_entities(content: &str) -> Cow<'_, str> {
    if !content.contains('&') {
        return Cow::Borrowed(content);
    }

    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            // Collect entity
            let mut entity = String::new();
            let mut found_semicolon = false;
            for ec in chars.by_ref() {
                if ec == ';' {
                    found_semicolon = true;
                    break;
                }
                entity.push(ec);
                if entity.len() > 10 {
                    break;
                }
            }

            if found_semicolon {
                match entity.as_str() {
                    "amp" => result.push('&'),
                    "lt" => result.push('<'),
                    "gt" => result.push('>'),
                    "quot" => result.push('"'),
                    "apos" => result.push('\''),
                    s if s.starts_with('#') => {
                        let code = if s.starts_with("#x") || s.starts_with("#X") {
                            u32::from_str_radix(&s[2..], 16).ok()
                        } else {
                            s[1..].parse::<u32>().ok()
                        };
                        if let Some(ch) = code.and_then(char::from_u32) {
                            result.push(ch);
                        } else {
                            // Unknown numeric entity - preserve as-is
                            result.push('&');
                            result.push_str(&entity);
                            result.push(';');
                        }
                    }
                    _ => {
                        // Unknown named entity - preserve as-is
                        result.push('&');
                        result.push_str(&entity);
                        result.push(';');
                    }
                }
            } else {
                // Malformed entity (no semicolon found) - preserve as-is
                result.push('&');
                result.push_str(&entity);
            }
        } else {
            result.push(c);
        }
    }

    Cow::Owned(result)
}

/// Check if a tag is self-closing
///
/// # Arguments
///
/// * `tag` - The XML tag to check
///
/// # Returns
///
/// true if the tag is self-closing (ends with "/>"), false otherwise
pub fn is_self_closing_tag(tag: &str) -> bool {
    tag.trim_end().ends_with("/>")
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

    #[test]
    fn test_strip_xml_tags() {
        let content = "<p>This is <b>bold</b> text</p>";
        let result = strip_xml_tags(content);
        assert_eq!(result, "This is bold text");
    }

    #[test]
    fn test_is_self_closing_tag() {
        assert!(is_self_closing_tag("<img src=\"test.jpg\"/>"));
        assert!(!is_self_closing_tag("<img src=\"test.jpg\">"));
    }

    #[test]
    fn test_decode_xml_entities() {
        // Named entities
        assert_eq!(decode_xml_entities("&amp;"), "&");
        assert_eq!(decode_xml_entities("&lt;"), "<");
        assert_eq!(decode_xml_entities("&gt;"), ">");
        assert_eq!(decode_xml_entities("&quot;"), "\"");
        assert_eq!(decode_xml_entities("&apos;"), "'");

        // Numeric entities (decimal)
        assert_eq!(decode_xml_entities("&#169;"), "©");
        assert_eq!(decode_xml_entities("&#231;"), "ç");
        assert_eq!(decode_xml_entities("&#193;"), "Á");

        // Numeric entities (hexadecimal)
        assert_eq!(decode_xml_entities("&#xA9;"), "©");
        assert_eq!(decode_xml_entities("&#x00A9;"), "©");

        // No entities — borrowed (no allocation)
        let result = decode_xml_entities("no entities here");
        assert!(matches!(result, Cow::Borrowed(_)));

        // Mixed content
        assert_eq!(
            decode_xml_entities("&#169; 2021 Fran&#231;ois &amp; Co"),
            "© 2021 François & Co"
        );
    }
}
