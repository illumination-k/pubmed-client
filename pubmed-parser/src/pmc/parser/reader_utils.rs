//! Reader-based XML parsing utilities for PMC parser
//!
//! Provides thin wrappers around `quick_xml::Reader` patterns used by
//! the PMC section and metadata parsers.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
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

/// Read all text content inside the current element, stripping child tags.
///
/// The reader must have just consumed `Event::Start` for `parent_tag`.
/// This function reads events until the matching `Event::End` for `parent_tag`,
/// collecting all `Text` events and ignoring child element tags.
///
/// Returns the concatenated text content (whitespace-trimmed by Reader config).
pub fn read_text_content(
    reader: &mut Reader<&[u8]>,
    parent_tag: &[u8],
    buf: &mut Vec<u8>,
) -> Result<String> {
    let mut text = String::new();
    let mut depth: u32 = 1;

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == parent_tag {
                    depth += 1;
                }
                // Skip child tags, keep reading for text
            }
            Ok(Event::Text(ref e)) => {
                let unescaped = e
                    .unescape()
                    .map_err(|err| ParseError::XmlError(err.to_string()))?;
                if !text.is_empty() && !unescaped.is_empty() {
                    // Add space between adjacent text fragments from mixed content
                    // e.g., "<p>text <b>bold</b> more</p>" → "text bold more"
                    // Only if the previous text doesn't end with space and current doesn't start with space
                    let needs_space = !text.ends_with(' ')
                        && !text.ends_with('\n')
                        && !unescaped.starts_with(' ')
                        && !unescaped.starts_with('\n');
                    if needs_space {
                        // Don't add space — the XML already had spacing around inline tags
                        // text events already include surrounding whitespace from the source
                    }
                }
                text.push_str(&unescaped);
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
        buf.clear();
    }

    Ok(text.trim().to_string())
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
/// Uses `read_to_end_into` to efficiently skip all child content.
pub fn skip_element(reader: &mut Reader<&[u8]>, tag: QName, buf: &mut Vec<u8>) -> Result<()> {
    reader
        .read_to_end_into(tag, buf)
        .map_err(|e| ParseError::XmlError(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_read_text_content_simple() {
        let xml = "<root><title>Hello World</title></root>";
        let mut reader = make_reader(xml);
        let mut buf = Vec::new();

        // Advance to <root>
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"title" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }

        let text = read_text_content(&mut reader, b"title", &mut buf).unwrap();
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_read_text_content_mixed() {
        let xml = "<p>Normal <b>bold</b> text</p>";
        let mut reader = make_reader(xml);
        let mut buf = Vec::new();

        // Advance past <p>
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"p" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }

        let text = read_text_content(&mut reader, b"p", &mut buf).unwrap();
        assert_eq!(text, "Normal bold text");
    }

    #[test]
    fn test_read_text_content_nested_same_tag() {
        let xml = "<sec><sec><title>Inner</title></sec></sec>";
        let mut reader = make_reader(xml);
        let mut buf = Vec::new();

        // Advance past outer <sec>
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"sec" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }

        let text = read_text_content(&mut reader, b"sec", &mut buf).unwrap();
        assert_eq!(text, "Inner");
    }

    #[test]
    fn test_read_text_content_entities() {
        let xml = "<p>CO&amp;2 &lt;levels&gt;</p>";
        let mut reader = make_reader(xml);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"p" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }

        let text = read_text_content(&mut reader, b"p", &mut buf).unwrap();
        assert_eq!(text, "CO&2 <levels>");
    }

    #[test]
    fn test_get_attr() {
        let xml = r#"<article id="test-id" article-type="research-article">"#;
        let mut reader = make_reader(xml);
        let mut buf = Vec::new();

        if let Ok(Event::Start(ref e)) = reader.read_event_into(&mut buf) {
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
        let mut buf = Vec::new();

        // expand_empty_elements turns this into Start + End
        if let Ok(Event::Start(ref e)) = reader.read_event_into(&mut buf) {
            assert_eq!(get_attr(e, b"xlink:href"), Some("fig1.jpg".to_string()));
        } else {
            panic!("expected Start event");
        }
    }

    #[test]
    fn test_skip_element() {
        let xml = "<root><skip><nested>deep</nested></skip><target>found</target></root>";
        let mut reader = make_reader(xml);
        let mut buf = Vec::new();

        // Advance to <root>
        reader.read_event_into(&mut buf).unwrap();
        buf.clear();

        // Read <skip> — clone name to avoid borrow conflict
        let skip_name = if let Ok(Event::Start(ref e)) = reader.read_event_into(&mut buf) {
            assert_eq!(e.name().as_ref(), b"skip");
            e.name().0.to_vec()
        } else {
            panic!("expected <skip>");
        };
        skip_element(&mut reader, QName(&skip_name), &mut buf).unwrap();
        buf.clear();

        // Next should be <target>
        if let Ok(Event::Start(ref e)) = reader.read_event_into(&mut buf) {
            assert_eq!(e.name().as_ref(), b"target");
            let text = read_text_content(&mut reader, b"target", &mut buf).unwrap();
            assert_eq!(text, "found");
        } else {
            panic!("expected <target>");
        }
    }

    #[test]
    fn test_read_text_content_empty_element() {
        let xml = "<root><title></title></root>";
        let mut reader = make_reader(xml);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"title" => break,
                Ok(Event::Eof) => panic!("unexpected EOF"),
                _ => {}
            }
            buf.clear();
        }

        let text = read_text_content(&mut reader, b"title", &mut buf).unwrap();
        assert_eq!(text, "");
    }
}
