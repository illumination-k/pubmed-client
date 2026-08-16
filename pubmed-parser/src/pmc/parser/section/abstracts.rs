//! JATS `<abstract>` parsing.

use crate::pmc::domain::Section;
use crate::pmc::parser::reader_utils::{make_reader, read_text_content};
use quick_xml::events::Event;

use super::figure::extract_figures_from_content;
use super::table::extract_tables_from_content;

/// What to do with an element encountered while scanning an `<abstract>`.
///
/// The abstract has its own small vocabulary rather than reusing the `<sec>`
/// one: here a `<title>` is discarded (the section is always titled
/// "Abstract"), whereas inside a `<sec>` it becomes the section title.
enum AbstractAction {
    Continue,
    Break,
    Enter,
    ReadParagraph,
    SkipTitle,
}

/// Extract abstract section using Reader for text, Reader scan for figures/tables
pub(super) fn extract_abstract_section(content: &str) -> Option<Section> {
    let abstract_start = content.find("<abstract")?;
    let abstract_end_offset = content[abstract_start..].find("</abstract>")?;
    let abstract_xml =
        &content[abstract_start..abstract_start + abstract_end_offset + "</abstract>".len()];

    // Extract text content using Reader
    let mut reader = make_reader(abstract_xml);
    let mut text_parts = Vec::new();
    let mut in_abstract = false;

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"abstract" => AbstractAction::Enter,
                b"p" if in_abstract => AbstractAction::ReadParagraph,
                b"title" if in_abstract => AbstractAction::SkipTitle,
                _ => AbstractAction::Continue,
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"abstract" => AbstractAction::Break,
            Ok(Event::Eof) => AbstractAction::Break,
            Err(_) => AbstractAction::Break,
            _ => AbstractAction::Continue,
        };

        match action {
            AbstractAction::Enter => in_abstract = true,
            AbstractAction::ReadParagraph => {
                if let Ok(text) = read_text_content(&mut reader, b"p") {
                    // `read_text_content` already returns trimmed text.
                    if !text.is_empty() {
                        text_parts.push(text);
                    }
                }
            }
            AbstractAction::SkipTitle => {
                let _ = read_text_content(&mut reader, b"title");
            }
            AbstractAction::Break => break,
            AbstractAction::Continue => {}
        }
    }

    // Extract figures and tables from the raw abstract content (handles inline figs)
    let figures = extract_figures_from_content(abstract_xml);
    let tables = extract_tables_from_content(abstract_xml);

    let clean_content = text_parts.join("\n");
    if clean_content.is_empty() {
        return None;
    }

    Some(Section {
        id: None,
        section_type: Some("abstract".to_string()),
        label: None,
        title: Some("Abstract".to_string()),
        content: clean_content,
        subsections: Vec::new(),
        figures,
        tables,
        formulas: Vec::new(),
        // Abstract paragraph text is collected via `read_text_content`,
        // which does not track `<xref>` targets; abstracts rarely carry
        // bibliographic citations, so this is left empty by design.
        cited_reference_ids: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_abstract_section() {
        let content = r#"
        <abstract>
            <p>This is an abstract paragraph.</p>
        </abstract>
        "#;

        let section = extract_abstract_section(content);
        assert!(section.is_some());

        let section = section.unwrap();
        assert_eq!(section.section_type, Some("abstract".to_string()));
        assert_eq!(section.title, Some("Abstract".to_string()));
        assert!(section.content.contains("This is an abstract paragraph."));
    }
}
