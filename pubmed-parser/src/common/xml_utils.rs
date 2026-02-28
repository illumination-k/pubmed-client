//! Common XML parsing utilities shared between PubMed and PMC parsers
//!
//! This module provides reusable XML parsing functions for both string-based
//! and serde-based XML parsing workflows.

use std::borrow::Cow;
use std::collections::HashMap;
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

/// Extract text between two XML tags
///
/// Finds the first occurrence of text between start and end tags.
///
/// # Arguments
///
/// * `content` - The XML content to search
/// * `start` - The opening tag (e.g., "<title>")
/// * `end` - The closing tag (e.g., "</title>")
///
/// # Returns
///
/// Some(String) with the text between tags, or None if not found
pub fn extract_text_between(content: &str, start: &str, end: &str) -> Option<String> {
    extract_text_between_ref(content, start, end).map(|s| s.to_string())
}

/// Extract text between two XML tags as a borrowed string slice
///
/// Same as [`extract_text_between`] but returns a `&str` slice to avoid allocation.
pub fn extract_text_between_ref<'a>(content: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start_pos = content.find(start)? + start.len();
    let end_pos = content[start_pos..].find(end)? + start_pos;
    Some(content[start_pos..end_pos].trim())
}

/// Extract attribute value from XML tag
///
/// # Arguments
///
/// * `content` - The XML tag content
/// * `attribute` - The attribute name to extract
///
/// # Returns
///
/// Some(String) with the attribute value, or None if not found
pub fn extract_attribute_value(content: &str, attribute: &str) -> Option<String> {
    let pattern = format!("{attribute}=\"");
    if let Some(attr_start) = content.find(&pattern) {
        let value_start = attr_start + pattern.len();
        if let Some(value_end) = content[value_start..].find('"') {
            return Some(content[value_start..value_start + value_end].to_string());
        }
    }
    None
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

/// Find all occurrences of a tag in content
///
/// Returns a vector of strings, each containing a complete tag with its content.
///
/// # Arguments
///
/// * `content` - The XML content to search
/// * `tag` - The tag name to find (e.g., "p" for <p>...</p>)
///
/// # Returns
///
/// A vector of strings containing all found tags
pub fn find_all_tags(content: &str, tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let start_tag = format!("<{}", tag);
    let end_tag = format!("</{}>", tag);

    let mut pos = 0;
    while let Some(start_pos) = content[pos..].find(&start_tag) {
        let start_pos = pos + start_pos;

        // Find the end of the opening tag
        if let Some(tag_end) = content[start_pos..].find(">") {
            let tag_end = start_pos + tag_end + 1;

            // Find the closing tag
            if let Some(end_pos) = content[tag_end..].find(&end_tag) {
                let end_pos = tag_end + end_pos;
                let tag_content = content[start_pos..end_pos + end_tag.len()].to_string();
                results.push(tag_content);
                pos = end_pos;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    results
}

/// Extract content between tags for all occurrences
///
/// Returns a vector of strings containing the text between each occurrence of the tags.
///
/// # Arguments
///
/// * `content` - The XML content to search
/// * `start` - The opening tag (e.g., "<p>")
/// * `end` - The closing tag (e.g., "</p>")
///
/// # Returns
///
/// A vector of strings containing all found text between tags
pub fn extract_all_text_between(content: &str, start: &str, end: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut pos = 0;

    while let Some(start_pos) = content[pos..].find(start) {
        let start_pos = pos + start_pos + start.len();
        if let Some(end_pos) = content[start_pos..].find(end) {
            let end_pos = start_pos + end_pos;
            let text = content[start_pos..end_pos].trim().to_string();
            if !text.is_empty() {
                results.push(text);
            }
            pos = end_pos;
        } else {
            break;
        }
    }

    results
}

/// Extract element content with its tag name
///
/// Finds the first occurrence of an XML element and returns its inner content.
///
/// # Arguments
///
/// * `content` - The XML content to search
/// * `tag` - The tag name (e.g., "section")
///
/// # Returns
///
/// Some(String) with the element's inner content, or None if not found
pub fn extract_element_content(content: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}", tag);
    let end_tag = format!("</{}>", tag);

    if let Some(start_pos) = content.find(&start_tag)
        && let Some(tag_end) = content[start_pos..].find(">")
    {
        let content_start = start_pos + tag_end + 1;
        if let Some(end_pos) = content[content_start..].find(&end_tag) {
            let content_end = content_start + end_pos;
            return Some(content[content_start..content_end].to_string());
        }
    }

    None
}

/// Extract all attributes from an XML tag
///
/// Parses an XML tag and returns a HashMap of attribute names to values.
///
/// # Arguments
///
/// * `tag` - The XML tag to parse (e.g., "<element id=\"test\" class=\"foo\">")
///
/// # Returns
///
/// A HashMap containing all attribute name-value pairs
pub fn extract_all_attributes(tag: &str) -> HashMap<String, String> {
    let mut attributes = HashMap::new();

    // Find the opening tag
    if let Some(start) = tag.find('<')
        && let Some(end) = tag[start..].find('>')
    {
        let tag_content = &tag[start + 1..start + end];

        // Skip the tag name
        if let Some(space_pos) = tag_content.find(' ') {
            let attrs_part = &tag_content[space_pos + 1..];
            let bytes = attrs_part.as_bytes();
            let len = bytes.len();

            // Parse attributes using byte-level operations (O(n) instead of O(n²))
            let mut pos = 0;
            while pos < len {
                // Skip whitespace
                while pos < len && bytes[pos].is_ascii_whitespace() {
                    pos += 1;
                }

                if pos >= len {
                    break;
                }

                // Find attribute name
                let name_start = pos;
                while pos < len && bytes[pos] != b'=' && !bytes[pos].is_ascii_whitespace() {
                    pos += 1;
                }

                if pos >= len {
                    break;
                }

                let attr_name = &attrs_part[name_start..pos];

                // Skip whitespace and '='
                while pos < len {
                    if bytes[pos] == b'=' {
                        pos += 1;
                        break;
                    } else if bytes[pos].is_ascii_whitespace() {
                        pos += 1;
                    } else {
                        break;
                    }
                }

                // Skip whitespace after '='
                while pos < len && bytes[pos].is_ascii_whitespace() {
                    pos += 1;
                }

                if pos >= len {
                    break;
                }

                // Extract quoted value
                let quote_byte = bytes[pos];
                if quote_byte == b'"' || quote_byte == b'\'' {
                    pos += 1; // Skip opening quote
                    let value_start = pos;
                    while pos < len && bytes[pos] != quote_byte {
                        pos += 1;
                    }
                    if pos < len {
                        attributes.insert(
                            attr_name.to_string(),
                            attrs_part[value_start..pos].to_string(),
                        );
                        pos += 1; // Skip closing quote
                    }
                }
            }
        }
    }

    attributes
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

/// Extract text content from a section, handling nested tags
///
/// Combines element extraction and tag stripping for convenience.
///
/// # Arguments
///
/// * `content` - The XML content to search
/// * `section_tag` - The section tag name
///
/// # Returns
///
/// Some(String) with the section text (tags removed), or None if not found
pub fn extract_section_text(content: &str, section_tag: &str) -> Option<String> {
    extract_element_content(content, section_tag)
        .map(|section_content| strip_xml_tags(&section_content))
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
    fn test_extract_text_between() {
        let content = "<title>Test Title</title>";
        let result = extract_text_between(content, "<title>", "</title>");
        assert_eq!(result, Some("Test Title".to_string()));
    }

    #[test]
    fn test_extract_attribute_value() {
        let content = r#"<element id="test-id" class="test-class">"#;
        let result = extract_attribute_value(content, "id");
        assert_eq!(result, Some("test-id".to_string()));
    }

    #[test]
    fn test_strip_xml_tags() {
        let content = "<p>This is <b>bold</b> text</p>";
        let result = strip_xml_tags(content);
        assert_eq!(result, "This is bold text");
    }

    #[test]
    fn test_find_all_tags() {
        let content = "<p>First paragraph</p><p>Second paragraph</p>";
        let results = find_all_tags(content, "p");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], "<p>First paragraph</p>");
        assert_eq!(results[1], "<p>Second paragraph</p>");
    }

    #[test]
    fn test_extract_all_text_between() {
        let content = "<p>First</p><p>Second</p><p>Third</p>";
        let results = extract_all_text_between(content, "<p>", "</p>");
        assert_eq!(results, vec!["First", "Second", "Third"]);
    }

    #[test]
    fn test_extract_element_content() {
        let content = "<section><title>Test</title><p>Content</p></section>";
        let result = extract_element_content(content, "section");
        assert_eq!(
            result,
            Some("<title>Test</title><p>Content</p>".to_string())
        );
    }

    #[test]
    fn test_is_self_closing_tag() {
        assert!(is_self_closing_tag("<img src=\"test.jpg\"/>"));
        assert!(!is_self_closing_tag("<img src=\"test.jpg\">"));
    }

    #[test]
    fn test_extract_all_attributes() {
        let tag = r#"<element id="test-id" class="test-class" data-value="123">"#;
        let attributes = extract_all_attributes(tag);

        assert_eq!(attributes.get("id"), Some(&"test-id".to_string()));
        assert_eq!(attributes.get("class"), Some(&"test-class".to_string()));
        assert_eq!(attributes.get("data-value"), Some(&"123".to_string()));
    }

    #[test]
    fn test_extract_section_text() {
        let content = "<section><title>Test</title><p>Content</p></section>";
        let result = extract_section_text(content, "section");
        // Note: strip_xml_tags removes tags but doesn't add spaces between elements
        assert_eq!(result, Some("TestContent".to_string()));
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
