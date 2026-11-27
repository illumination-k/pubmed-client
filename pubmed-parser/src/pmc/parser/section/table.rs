//! JATS `<table-wrap>` parsing.

use crate::pmc::domain::{Table, TableCell, TableRow};
use crate::pmc::parser::reader_utils::{
    get_attr, read_block_text, read_text_content, skip_element,
};
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
///
/// Reads the label, caption, the XHTML `<table>` rows (also when wrapped in
/// `<alternatives>` next to a `<graphic>`), and one footnote per `<fn>` / `<p>`
/// of `<table-wrap-foot>`.
pub(super) fn parse_table_inner(
    reader: &mut quick_xml::Reader<&[u8]>,
    attrs: TableAttrs,
) -> Option<Table> {
    let mut label: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut head = Vec::new();
    let mut body = Vec::new();
    let mut footnotes = Vec::new();

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"label" => TableAction::ReadLabel,
                b"caption" => TableAction::ReadCaption,
                b"table" => TableAction::ReadTable,
                b"table-wrap-foot" => TableAction::ReadFootnote,
                // Wrappers around the table itself: descend.
                b"alternatives" | b"oasis:table" => TableAction::Continue,
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
                caption = match read_block_text(reader, b"caption") {
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
            TableAction::ReadTable => read_table_rows(reader, &mut head, &mut body),
            TableAction::ReadFootnote => footnotes.extend(read_table_footnotes(reader)),
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
        head,
        body,
        footnotes,
    })
}

/// Read the rows of an XHTML `<table>`; the reader has just consumed its start tag.
///
/// Rows inside `<thead>` go to `head`; rows in `<tbody>`, `<tfoot>` or directly
/// under `<table>` go to `body`.
fn read_table_rows(
    reader: &mut quick_xml::Reader<&[u8]>,
    head: &mut Vec<TableRow>,
    body: &mut Vec<TableRow>,
) {
    let mut in_head = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"thead" => in_head = true,
                b"tr" => {
                    let row = read_table_row(reader);
                    if !row.cells.is_empty() {
                        if in_head {
                            head.push(row);
                        } else {
                            body.push(row);
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"thead" => in_head = false,
                b"table" => break,
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
}

/// Read the cells of a `<tr>`; the reader has just consumed its start tag.
fn read_table_row(reader: &mut quick_xml::Reader<&[u8]>) -> TableRow {
    let mut cells = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if matches!(e.name().as_ref(), b"td" | b"th") => {
                let is_header = e.name().as_ref() == b"th";
                let colspan = get_attr(e, b"colspan").and_then(|v| v.trim().parse().ok());
                let rowspan = get_attr(e, b"rowspan").and_then(|v| v.trim().parse().ok());
                let tag: &[u8] = if is_header { b"th" } else { b"td" };
                let content = read_block_text(reader, tag).unwrap_or_default();
                cells.push(TableCell {
                    content,
                    is_header,
                    colspan,
                    rowspan,
                });
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"tr" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    TableRow { cells }
}

/// Read `<table-wrap-foot>` as one entry per `<fn>` or `<p>`; the reader has
/// just consumed its start tag. Text outside any such block becomes its own entry.
fn read_table_footnotes(reader: &mut quick_xml::Reader<&[u8]>) -> Vec<String> {
    let mut footnotes = Vec::new();
    let mut loose = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if matches!(e.name().as_ref(), b"fn" | b"p") => {
                let tag = e.name().as_ref().to_vec();
                if let Ok(text) = read_block_text(reader, &tag)
                    && !text.is_empty()
                {
                    footnotes.push(text);
                }
            }
            Ok(Event::Text(ref t)) => {
                if let Ok(text) = t.decode() {
                    loose.push_str(&text);
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"table-wrap-foot" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    let loose = loose.split_whitespace().collect::<Vec<_>>().join(" ");
    if !loose.is_empty() {
        footnotes.push(loose);
    }
    footnotes
}

enum TableAction {
    Continue,
    Done,
    ReadLabel,
    ReadCaption,
    ReadTable,
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

    #[test]
    fn test_table_rows_caption_and_footnotes() {
        let content = r#"
        <table-wrap id="t1">
            <label>Table 1</label>
            <caption><title>Baseline characteristics.</title><p>Mean (SD).</p></caption>
            <alternatives>
                <graphic xlink:href="t1.jpg"/>
                <table frame="hsides">
                    <thead><tr><th rowspan="2">Variable</th><th colspan="2">Group <italic>n</italic></th></tr></thead>
                    <tbody>
                        <tr><td>Age</td><td>41.2</td><td><p>39.8</p></td></tr>
                    </tbody>
                </table>
            </alternatives>
            <table-wrap-foot><fn id="fn1"><p>First note.</p></fn><fn id="fn2"><p>Second note.</p></fn></table-wrap-foot>
        </table-wrap>
        "#;

        let tables = extract_tables_from_content(content);
        assert_eq!(tables.len(), 1);
        let table = &tables[0];
        assert_eq!(
            table.caption.as_deref(),
            Some("Baseline characteristics. Mean (SD).")
        );

        assert_eq!(table.head.len(), 1);
        let header = &table.head[0].cells;
        assert_eq!(header.len(), 2);
        assert!(header[0].is_header);
        assert_eq!(header[0].rowspan, Some(2));
        assert_eq!(header[1].content, "Group n");
        assert_eq!(header[1].colspan, Some(2));

        assert_eq!(table.body.len(), 1);
        let cells: Vec<&str> = table.body[0]
            .cells
            .iter()
            .map(|c| c.content.as_str())
            .collect();
        assert_eq!(cells, ["Age", "41.2", "39.8"]);
        assert!(!table.body[0].cells[0].is_header);

        assert_eq!(table.footnotes, ["First note.", "Second note."]);
    }
}
