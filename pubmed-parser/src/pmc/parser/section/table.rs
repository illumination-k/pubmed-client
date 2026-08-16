//! JATS `<table-wrap>` parsing.

use crate::pmc::domain::Table;
use crate::pmc::parser::reader_utils::{get_attr, read_text_content, skip_element};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use tracing::warn;

/// Attributes lifted off a `<table-wrap>` start event before the reader moves on.
pub(super) struct TableAttrs {
    id: Option<String>,
}

impl TableAttrs {
    /// Capture the attributes of a `<table-wrap>` start event.
    pub(super) fn from_start(e: &BytesStart) -> Self {
        Self {
            id: get_attr(e, b"id"),
        }
    }
}

/// Extract all `<table-wrap>` elements from content using Reader.
pub(super) fn extract_tables_from_content(content: &str) -> Vec<Table> {
    super::scan_elements(
        content,
        b"table-wrap",
        TableAttrs::from_start,
        parse_table_inner,
    )
}

/// Parse table-wrap content after `Event::Start` for `<table-wrap>` has been consumed.
pub(super) fn parse_table_inner(
    reader: &mut quick_xml::Reader<&[u8]>,
    attrs: TableAttrs,
) -> Option<Table> {
    let mut label: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut footnotes = Vec::new();

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"label" => TableAction::ReadLabel,
                b"caption" => TableAction::ReadCaption,
                b"table-wrap-foot" => TableAction::ReadFootnote,
                other => TableAction::Skip(other.to_vec()),
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"table-wrap" => TableAction::Done,
            Ok(Event::Eof) => TableAction::Done,
            Err(_) => TableAction::Done,
            _ => TableAction::Continue,
        };

        match action {
            TableAction::ReadLabel => {
                label = read_text_content(reader, b"label").ok();
            }
            TableAction::ReadCaption => {
                caption = match read_text_content(reader, b"caption") {
                    Ok(text) => Some(text),
                    Err(e) => {
                        warn!(
                            table_id = ?attrs.id,
                            error = %e,
                            "failed to parse table caption"
                        );
                        None
                    }
                };
            }
            TableAction::ReadFootnote => {
                if let Ok(text) = read_text_content(reader, b"table-wrap-foot") {
                    // Already trimmed by `read_text_content`.
                    if !text.is_empty() {
                        footnotes.push(text);
                    }
                }
            }
            TableAction::Skip(name) => {
                let _ = skip_element(reader, QName(&name));
            }
            TableAction::Done => break,
            TableAction::Continue => {}
        }
    }

    let id = match attrs.id {
        Some(id) => id,
        None => {
            warn!("table-wrap element missing id attribute");
            format!("table_unknown_{}", line!())
        }
    };
    Some(Table {
        id,
        label,
        caption,
        head: Vec::new(),
        body: Vec::new(),
        footnotes,
    })
}

enum TableAction {
    Continue,
    Done,
    ReadLabel,
    ReadCaption,
    ReadFootnote,
    Skip(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tables_from_section() {
        let content = r#"
        <root>
        <table-wrap id="table1">
            <label>Table 1</label>
            <caption>This is a test table.</caption>
            <table>
                <tr><th>Header</th></tr>
                <tr><td>Data</td></tr>
            </table>
        </table-wrap>
        </root>
        "#;

        let tables = extract_tables_from_content(content);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].id, "table1");
        assert_eq!(tables[0].label, Some("Table 1".to_string()));
        assert_eq!(tables[0].caption.as_deref(), Some("This is a test table."));
    }
}
