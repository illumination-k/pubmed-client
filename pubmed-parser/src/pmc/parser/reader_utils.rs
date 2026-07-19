//! Reader-based XML parsing utilities for PMC parser
//!
//! Provides thin wrappers around `quick_xml::Reader` patterns used by
//! the PMC section and metadata parsers.

use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::name::QName;

use crate::error::{ParseError, Result};

/// Create a configured `Reader` from a string slice.
///
/// Configuration:
/// - `trim_text(false)`: preserves whitespace in mixed content (e.g., `<p>text <b>bold</b> more</p>`)
/// - `expand_empty_elements(true)`: turns `<tag/>` into `Start` + `End` events
///
/// Note: `trim_text` is intentionally false to preserve internal whitespace in mixed content.
/// Functions that need trimmed results should trim the final collected text.
pub fn make_reader(content: &str) -> Reader<&[u8]> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().expand_empty_elements = true;
    reader
}

/// Resolve an `Event::GeneralRef` (`&name;` or `&#…;`) to its text.
///
/// quick-xml no longer resolves references inside `Text` events; they arrive
/// as separate `GeneralRef` events. Character references and the five
/// predefined XML entities are resolved here; unknown (DTD-defined) entities
/// are kept verbatim as `&name;` so no text is silently lost.
pub fn resolve_general_ref(r: &BytesRef) -> Result<String> {
    if let Some(ch) = r
        .resolve_char_ref()
        .map_err(|e| ParseError::XmlError(e.to_string()))?
    {
        return Ok(ch.to_string());
    }
    let name = r
        .decode()
        .map_err(|e| ParseError::XmlError(e.to_string()))?;
    if let Some(text) = resolve_predefined_entity(&name) {
        return Ok(text.to_string());
    }
    Ok(format!("&{name};"))
}

/// Read all text content inside the current element, stripping child tags.
///
/// The reader must have just consumed `Event::Start` for `parent_tag`.
/// This function reads events until the matching `Event::End` for `parent_tag`,
/// collecting all `Text` events and ignoring child element tags.
///
/// Returns the concatenated text content (whitespace-trimmed by Reader config).
///
/// Uses the borrowing `read_event()` (zero-copy from the source slice) rather than
/// `read_event_into(&mut buf)`, which would copy every event's bytes into a scratch
/// `Vec`. On the hot PMC path this avoids a large amount of `memmove` traffic.
pub fn read_text_content(reader: &mut Reader<&[u8]>, parent_tag: &[u8]) -> Result<String> {
    let mut text = String::new();
    let mut depth: u32 = 1;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == parent_tag {
                    depth += 1;
                }
                // Skip child tags, keep reading for text
            }
            Ok(Event::Text(ref e)) => {
                // Mixed content (e.g. "<p>text <b>bold</b> more</p>") arrives as
                // multiple Text events; the source whitespace around inline tags
                // is preserved in the events, so we simply concatenate.
                let decoded = e
                    .decode()
                    .map_err(|err| ParseError::XmlError(err.to_string()))?;
                text.push_str(&decoded);
            }
            Ok(Event::GeneralRef(ref e)) => {
                text.push_str(&resolve_general_ref(e)?);
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == parent_tag {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::XmlError(e.to_string())),
            _ => {}
        }
    }

    // Reuse the existing buffer instead of allocating a second String via
    // `trim().to_string()`. This runs for every text-bearing element (titles,
    // paragraphs, captions, …), so avoiding a per-call allocation matters on
    // large documents.
    Ok(trim_in_place(text))
}

/// Trim leading/trailing whitespace from an owned `String` in place, reusing its
/// allocation rather than allocating a fresh `String` (as `trim().to_string()`
/// would). On the hot PMC parsing path this saves one allocation per text node.
pub(crate) fn trim_in_place(mut text: String) -> String {
    let trimmed_len = text.trim().len();
    if trimmed_len == text.len() {
        return text;
    }
    if trimmed_len == 0 {
        text.clear();
        return text;
    }
    // The trimmed content is a contiguous slice `[start, start + trimmed_len)`.
    let start = text.len() - text.trim_start().len();
    text.truncate(start + trimmed_len);
    text.drain(..start);
    text
}

/// Extract an attribute value from a `BytesStart` event.
///
/// Returns `Some(String)` if the attribute exists, `None` otherwise.
pub fn get_attr(e: &BytesStart, name: &[u8]) -> Option<String> {
    e.try_get_attribute(name)
        .ok()?
        .map(|a| String::from_utf8_lossy(&a.value).into_owned())
}

/// Skip an entire element. The reader must have just consumed `Event::Start` for the tag.
///
/// Uses the borrowing `read_to_end` (zero-copy) to efficiently skip all child content.
pub fn skip_element(reader: &mut Reader<&[u8]>, tag: QName) -> Result<()> {
    reader
        .read_to_end(tag)
        .map_err(|e| ParseError::XmlError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    /// Advance the reader until a `Start` event for `tag` is consumed.
    fn advance_to(reader: &mut Reader<&[u8]>, tag: &[u8]) {
        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if e.name().as_ref() == tag => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
        }
    }

    #[test]
    fn test_read_text_content_simple() {
        let xml = "<root><title>Hello World</title></root>";
        let mut reader = make_reader(xml);
        advance_to(&mut reader, b"title");

        let text = read_text_content(&mut reader, b"title").unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_read_text_content_mixed() {
        let xml = "<p>Normal <b>bold</b> text</p>";
        let mut reader = make_reader(xml);
        advance_to(&mut reader, b"p");

        let text = read_text_content(&mut reader, b"p").unwrap();
        assert_eq!(text, "Normal bold text");
    }

    #[test]
    fn test_read_text_content_nested_same_tag() {
        let xml = "<sec><sec><title>Inner</title></sec></sec>";
        let mut reader = make_reader(xml);
        advance_to(&mut reader, b"sec");

        let text = read_text_content(&mut reader, b"sec").unwrap();
        assert_eq!(text, "Inner");
    }

    #[test]
    fn test_read_text_content_entities() {
        let xml = "<p>CO&amp;2 &lt;levels&gt;</p>";
        let mut reader = make_reader(xml);
        advance_to(&mut reader, b"p");

        let text = read_text_content(&mut reader, b"p").unwrap();
        assert_eq!(text, "CO&2 <levels>");
    }

    #[test]
    fn test_get_attr() {
        let xml = r#"<article id="test-id" article-type="research-article">"#;
        let mut reader = make_reader(xml);

        if let Ok(Event::Start(ref e)) = reader.read_event() {
            assert_eq!(get_attr(e, b"id"), Some("test-id".to_string()));
            assert_eq!(
                get_attr(e, b"article-type"),
                Some("research-article".to_string())
            );
            assert_eq!(get_attr(e, b"nonexistent"), None);
        } else {
            panic!("expected Start event");
        }
    }

    #[test]
    fn test_get_attr_namespaced() {
        let xml = r#"<graphic xlink:href="fig1.jpg"/>"#;
        let mut reader = make_reader(xml);

        // expand_empty_elements turns this into Start + End
        if let Ok(Event::Start(ref e)) = reader.read_event() {
            assert_eq!(get_attr(e, b"xlink:href"), Some("fig1.jpg".to_string()));
        } else {
            panic!("expected Start event");
        }
    }

    #[test]
    fn test_skip_element() {
        let xml = "<root><skip><nested>deep</nested></skip><target>found</target></root>";
        let mut reader = make_reader(xml);

        // Advance to <root>
        reader.read_event().unwrap();

        // Read <skip> — clone name to avoid borrow conflict
        let skip_name = if let Ok(Event::Start(ref e)) = reader.read_event() {
            assert_eq!(e.name().as_ref(), b"skip");
            e.name().0.to_vec()
        } else {
            panic!("expected <skip>");
        };
        skip_element(&mut reader, QName(&skip_name)).unwrap();

        // Next should be <target>
        if let Ok(Event::Start(ref e)) = reader.read_event() {
            assert_eq!(e.name().as_ref(), b"target");
            let text = read_text_content(&mut reader, b"target").unwrap();
            assert_eq!(text, "found");
        } else {
            panic!("expected <target>");
        }
    }

    #[test]
    fn test_read_text_content_empty_element() {
        let xml = "<root><title></title></root>";
        let mut reader = make_reader(xml);
        advance_to(&mut reader, b"title");

        let text = read_text_content(&mut reader, b"title").unwrap();
        assert_eq!(text, "");
    }
}
