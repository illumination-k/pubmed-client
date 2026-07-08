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
    //
    // NOTE: a hand-rolled scalar byte scanner was tried here and benchmarked
    // ~5-9% SLOWER on real PubMed XML — the `regex` crate uses a SIMD `memchr`
    // prefilter to find `<` candidates, which beats a per-byte scalar loop.
    // `replace_all` already returns a borrowed `Cow` (no allocation) when the
    // document contains no inline tags, so this is the fast path too.
    static INLINE_TAG_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    let re = INLINE_TAG_REGEX
        .get_or_init(|| Regex::new(r"</?(?:i|b|u|sup|sub|em|strong|italic|bold)>").ok());

    // If the (constant) pattern somehow failed to compile, leave the input untouched.
    let Some(re) = re else {
        return Cow::Borrowed(xml);
    };

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
///
/// Malformed or unrecognized sequences (missing `;`, unknown names, out-of-range
/// or non-parsable numeric values) are preserved verbatim rather than dropped, so
/// this never silently corrupts article text.
pub fn decode_xml_entities(content: &str) -> Cow<'_, str> {
    if !content.contains('&') {
        return Cow::Borrowed(content);
    }

    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars();

    while let Some(c) = chars.next() {
        if c != '&' {
            result.push(c);
            continue;
        }

        // Collect the entity body following '&', up to (and excluding) the ';'.
        let (entity, found_semicolon) = collect_entity_body(&mut chars);

        match decode_entity(&entity, found_semicolon) {
            Some(decoded) => result.push(decoded),
            // Unrecognized/malformed — reconstruct the original bytes verbatim.
            None => {
                result.push('&');
                result.push_str(&entity);
                if found_semicolon {
                    result.push(';');
                }
            }
        }
    }

    Cow::Owned(result)
}

/// Collect the characters of an entity body immediately following a `&`.
///
/// Reads until a terminating `;` is found (returning `true`) or the body grows
/// past the longest entity we recognize (returning `false`). The cap bounds work
/// on pathological input like `&aaaaaaaaaaaaaaa` that has no terminator nearby.
fn collect_entity_body(chars: &mut impl Iterator<Item = char>) -> (String, bool) {
    let mut entity = String::new();
    for ec in chars.by_ref() {
        if ec == ';' {
            return (entity, true);
        }
        entity.push(ec);
        if entity.len() > 10 {
            break;
        }
    }
    (entity, false)
}

/// Decode a single collected entity body into its character.
///
/// Returns `None` when the sequence is not a recognizable entity (no terminating
/// `;`, unknown name, or a numeric value that fails to parse / is out of range),
/// signalling the caller to preserve the original text.
fn decode_entity(entity: &str, found_semicolon: bool) -> Option<char> {
    if !found_semicolon {
        return None;
    }
    match entity.strip_prefix('#') {
        Some(numeric) => decode_numeric_entity(numeric),
        None => decode_named_entity(entity),
    }
}

/// Decode a predefined named entity body (the text between `&` and `;`).
fn decode_named_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    }
}

/// Decode a numeric entity body (the text after `&#`), dispatching on the
/// `x`/`X` hex marker.
fn decode_numeric_entity(body: &str) -> Option<char> {
    match body.strip_prefix(['x', 'X']) {
        Some(hex) => decode_numeric_hex(hex),
        None => decode_numeric_decimal(body),
    }
}

/// Decode a decimal numeric entity body (e.g. `169` from `&#169;`).
fn decode_numeric_decimal(digits: &str) -> Option<char> {
    digits.parse::<u32>().ok().and_then(char::from_u32)
}

/// Decode a hexadecimal numeric entity body (e.g. `A9` from `&#xA9;`).
fn decode_numeric_hex(digits: &str) -> Option<char> {
    u32::from_str_radix(digits, 16)
        .ok()
        .and_then(char::from_u32)
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
    fn test_strip_inline_html_tags_no_alloc_when_clean() {
        // When there are no strippable tags, the input is returned borrowed
        // (zero allocation) — this is the common hot path.
        let xml = r#"<Article><Title>A clean title</Title><b-cell>x</b-cell></Article>"#;
        let cleaned = strip_inline_html_tags(xml);
        assert!(matches!(cleaned, Cow::Borrowed(_)));
        assert_eq!(cleaned, xml);
    }

    #[test]
    fn test_strip_inline_html_tags_all_names() {
        // Every supported tag name, opening and closing.
        let xml = "<strong>a</strong><italic>b</italic><bold>c</bold><sup>d</sup>\
                   <sub>e</sub><em>f</em><i>g</i><b>h</b><u>i</u>";
        let cleaned = strip_inline_html_tags(xml);
        assert_eq!(cleaned, "abcdefghi");
    }

    #[test]
    fn test_strip_inline_html_tags_prefix_disambiguation() {
        // `<b>` is stripped but `<bold>`-prefixed names and longer element
        // names that merely start with a tag name must be preserved.
        let xml = "<b>x</b> <bdi>y</bdi> <subsection>z</subsection> <input>w</input>";
        let cleaned = strip_inline_html_tags(xml);
        assert_eq!(
            cleaned,
            "x <bdi>y</bdi> <subsection>z</subsection> <input>w</input>"
        );
    }

    #[test]
    fn test_strip_inline_html_tags_attributes_preserved() {
        // The original regex only matched bare tags (no attributes), so an
        // opening tag carrying attributes is NOT stripped, while the bare
        // closing tag still is. This reproduces that exact behavior.
        let xml = r#"<i class="x">kept</i>"#;
        let cleaned = strip_inline_html_tags(xml);
        assert_eq!(cleaned, r#"<i class="x">kept"#);
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

    #[test]
    fn test_decode_xml_entities_malformed_and_truncated() {
        // Truncated sequences with no terminating ';' are preserved verbatim.
        assert_eq!(decode_xml_entities("&"), "&");
        assert_eq!(decode_xml_entities("&amp"), "&amp");
        assert_eq!(decode_xml_entities("&#"), "&#");
        assert_eq!(decode_xml_entities("&#x"), "&#x");
        assert_eq!(decode_xml_entities("&#169"), "&#169");

        // Empty and malformed numeric bodies (terminated) are preserved as-is.
        assert_eq!(decode_xml_entities("&#;"), "&#;");
        assert_eq!(decode_xml_entities("&#x;"), "&#x;");
        assert_eq!(decode_xml_entities("&#xZZ;"), "&#xZZ;");
        assert_eq!(decode_xml_entities("&#12x4;"), "&#12x4;");

        // Unknown named entity is preserved with its delimiters intact.
        assert_eq!(decode_xml_entities("&unknown;"), "&unknown;");
        assert_eq!(decode_xml_entities("&nbsp;"), "&nbsp;");

        // Out-of-range code points (beyond U+10FFFF or surrogate range) fail to
        // convert and are preserved rather than dropped.
        assert_eq!(decode_xml_entities("&#xD800;"), "&#xD800;"); // surrogate
        assert_eq!(decode_xml_entities("&#1114112;"), "&#1114112;"); // > U+10FFFF

        // Overlong bodies (no ';' within the cap) are preserved and scanning
        // resumes cleanly afterwards.
        assert_eq!(
            decode_xml_entities("&abcdefghijklmnop;tail"),
            "&abcdefghijklmnop;tail"
        );

        // Bare '&' embedded in ordinary text, and entities at string boundaries.
        assert_eq!(decode_xml_entities("a & b"), "a & b");
        assert_eq!(decode_xml_entities("&amp;start"), "&start");
        assert_eq!(decode_xml_entities("end&amp;"), "end&");

        // The scanner reads greedily to the first ';', so an unterminated '&'
        // absorbs the following (otherwise valid) entity body into one unknown
        // run, which is then preserved verbatim.
        assert_eq!(decode_xml_entities("&foo &amp; bar"), "&foo &amp; bar");
        // A terminated malformed sequence followed by a valid one: the first is
        // preserved, the second still decodes.
        assert_eq!(decode_xml_entities("&#xZZ; &lt;"), "&#xZZ; <");
    }

    #[test]
    fn test_decode_xml_entities_hex_case_insensitive() {
        // Both the 'x' marker and the hex digits are case-insensitive.
        assert_eq!(decode_xml_entities("&#Xa9;"), "©");
        assert_eq!(decode_xml_entities("&#xa9;"), "©");
        assert_eq!(decode_xml_entities("&#xAF;"), "¯");
    }
}
