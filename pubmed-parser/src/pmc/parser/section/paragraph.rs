//! JATS `<p>` parsing, including the inline elements a paragraph can carry.

use crate::pmc::domain::{Figure, Table};
use crate::pmc::parser::reader_utils::{get_attr, resolve_general_ref, trim_in_place};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::mem::take;

use super::figure::{FigAttrs, parse_figure_inner};
use super::table::{TableAttrs, parse_table_inner};

/// Everything a single `<p>` contributes to its enclosing section.
pub(super) struct Paragraph {
    /// Concatenated, trimmed text of the paragraph, citation markers included.
    pub(super) text: String,
    /// `<fig>` elements found inline in the paragraph.
    pub(super) figures: Vec<Figure>,
    /// `<table-wrap>` elements found inline in the paragraph.
    pub(super) tables: Vec<Table>,
    /// `rid` targets of the paragraph's `<xref ref-type="bibr">` citations.
    pub(super) cited_reference_ids: Vec<String>,
}

/// Whether `tag` is a JATS block-level (`%para-level;`) element whose text we
/// extract inline rather than skipping. Shared by the body and `<sec>` parsers.
pub(super) fn is_block_level(tag: &[u8]) -> bool {
    matches!(
        tag,
        b"list"
            | b"def-list"
            | b"disp-formula"
            | b"disp-formula-group"
            | b"disp-quote"
            | b"boxed-text"
            | b"code"
            | b"preformat"
            | b"media"
            | b"supplementary-material"
            | b"speech"
            | b"statement"
            | b"verse-group"
            | b"array"
            | b"graphic"
            | b"fn-group"
    )
}

/// Read a `<p>` element, collecting text while extracting inline figures and tables.
///
/// The reader must have just consumed `Event::Start` for the `<p>`. The loop
/// here only accumulates text and tracks nesting depth; every non-text child is
/// dispatched to [`InlineContent`], which owns the `<fig>`/`<table-wrap>`/`<xref>`
/// handling.
///
/// Text is pushed from the `Cow<str>` returned by `decode()`, which borrows for
/// plain UTF-8 input and only allocates when a decode is actually needed.
pub(super) fn read_paragraph_with_inline(reader: &mut Reader<&[u8]>) -> Paragraph {
    let mut text = String::new();
    let mut inline = InlineContent::default();
    let mut depth: u32 = 1; // We're inside <p>

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                if e.name().as_ref() == b"p" {
                    depth += 1;
                } else {
                    inline.note_start(e);
                }
            }
            Ok(Event::Text(ref e)) => {
                if let Ok(decoded) = e.decode() {
                    text.push_str(&decoded);
                }
            }
            Ok(Event::GeneralRef(ref e)) => {
                if let Ok(resolved) = resolve_general_ref(e) {
                    text.push_str(&resolved);
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"p" => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }

        // Only reachable once the borrow taken by `read_event()` above has
        // ended, which is exactly why the parsing is deferred.
        inline.parse_pending(reader);
    }

    inline.into_paragraph(trim_in_place(text))
}

/// The non-text children collected while scanning a `<p>`.
///
/// `<fig>` and `<table-wrap>` bodies cannot be consumed while their start event
/// is still borrowing the reader, so the attributes are queued in `pending_*`
/// and the bodies parsed by [`InlineContent::parse_pending`] once the borrow ends.
#[derive(Default)]
struct InlineContent {
    figures: Vec<Figure>,
    tables: Vec<Table>,
    cited_reference_ids: Vec<String>,
    pending_figs: Vec<FigAttrs>,
    pending_tables: Vec<TableAttrs>,
}

impl InlineContent {
    /// Dispatch one non-`<p>` start event to the handler for its element.
    ///
    /// Unhandled inline markup (`<italic>`, `<sup>`, …) is deliberately ignored:
    /// its text still arrives as `Event::Text` and lands in the paragraph.
    fn note_start(&mut self, e: &BytesStart) {
        match e.name().as_ref() {
            b"xref" => self.collect_bibr_rids(e),
            b"fig" => self.pending_figs.push(FigAttrs::from_start(e)),
            b"table-wrap" => self.pending_tables.push(TableAttrs::from_start(e)),
            _ => {}
        }
    }

    /// Append the bibliographic reference targets of an `<xref>` start event, in
    /// document order.
    ///
    /// Only `<xref ref-type="bibr">` contributes — other cross-reference kinds
    /// (`fig`, `table`, `disp-formula`, …) are ignored here. The `rid` attribute
    /// is JATS `IDREFS`, so a grouped citation such as `rid="B1 B2 B3"` yields
    /// each id separately. The `<xref>`'s visible text (the citation marker) is
    /// left in the surrounding paragraph content untouched.
    fn collect_bibr_rids(&mut self, e: &BytesStart) {
        if get_attr(e, b"ref-type").as_deref() != Some("bibr") {
            return;
        }
        if let Some(rid) = get_attr(e, b"rid") {
            self.cited_reference_ids
                .extend(rid.split_whitespace().map(str::to_string));
        }
    }

    /// Parse the bodies of the elements queued by [`InlineContent::note_start`].
    fn parse_pending(&mut self, reader: &mut Reader<&[u8]>) {
        // Taken out and put back so the queues keep their capacity across
        // iterations while `self.figures`/`self.tables` stay mutably borrowable.
        let mut figs = take(&mut self.pending_figs);
        for attrs in figs.drain(..) {
            if let Some(fig) = parse_figure_inner(reader, attrs) {
                self.figures.push(fig);
            }
        }
        self.pending_figs = figs;

        let mut tables = take(&mut self.pending_tables);
        for attrs in tables.drain(..) {
            if let Some(table) = parse_table_inner(reader, attrs) {
                self.tables.push(table);
            }
        }
        self.pending_tables = tables;
    }

    fn into_paragraph(self, text: String) -> Paragraph {
        Paragraph {
            text,
            figures: self.figures,
            tables: self.tables,
            cited_reference_ids: self.cited_reference_ids,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::extract_sections_enhanced;

    #[test]
    fn test_inline_figure_in_paragraph() {
        let content = r#"
        <body>
            <p>Some text <fig id="fig1"><label>Figure 1</label><caption>Test caption</caption><graphic xlink:href="fig1.jpg"/></fig> more text.</p>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        // Figures should be found even when inline in <p>
        assert!(
            !sections[0].figures.is_empty(),
            "Expected figures to be extracted from inline position"
        );
        assert_eq!(sections[0].figures[0].id, "fig1");
    }

    // --- Tests for in-text citation linkage (<xref ref-type="bibr">) ---

    #[test]
    fn test_cited_reference_ids_captured_per_section() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Introduction</title>
            <p>Prior work established this <xref ref-type="bibr" rid="B1">1</xref>,
               and later studies confirmed it <xref ref-type="bibr" rid="B2">2</xref>.</p>
            <p>A follow-up <xref ref-type="bibr" rid="B3">3</xref> extended the results.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let section = &sections[0];
        assert_eq!(
            section.cited_reference_ids,
            vec!["B1".to_string(), "B2".to_string(), "B3".to_string()]
        );
        // The visible citation markers stay in the content unchanged.
        assert!(section.content.contains('1'));
        assert!(section.content.contains("Prior work established this"));
    }

    #[test]
    fn test_grouped_citation_rids_split() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Methods</title>
            <p>Several groups reported this <xref ref-type="bibr" rid="B1 B2 B3">1-3</xref>.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        assert_eq!(
            sections[0].cited_reference_ids,
            vec!["B1".to_string(), "B2".to_string(), "B3".to_string()]
        );
    }

    #[test]
    fn test_non_bibr_xref_not_captured_as_citation() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Results</title>
            <p>As shown in <xref ref-type="fig" rid="fig1">Figure 1</xref>, the effect
               is significant <xref ref-type="bibr" rid="B5">5</xref>.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        // Only the bibr xref contributes; the figure xref is ignored.
        assert_eq!(sections[0].cited_reference_ids, vec!["B5".to_string()]);
    }

    #[test]
    fn test_cited_reference_ids_empty_without_citations() {
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Discussion</title>
            <p>No citations in this paragraph.</p>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        assert!(sections[0].cited_reference_ids.is_empty());
    }

    #[test]
    fn test_cited_reference_ids_in_body_without_sections() {
        let content = r#"
        <body>
            <p>Early results <xref ref-type="bibr" rid="B1">1</xref> were promising.</p>
            <p>Later work <xref ref-type="bibr" rid="B2">2</xref> disagreed.</p>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].section_type, Some("body".to_string()));
        assert_eq!(
            sections[0].cited_reference_ids,
            vec!["B1".to_string(), "B2".to_string()]
        );
    }

    #[test]
    fn test_cited_reference_ids_scoped_to_own_section() {
        // Each <sec> keeps only the citations from its own paragraphs; the
        // recursive accessor on the domain model aggregates subsections.
        let content = r#"
        <body>
        <sec id="sec1">
            <title>Outer</title>
            <p>Outer cite <xref ref-type="bibr" rid="B1">1</xref>.</p>
            <sec id="sec1.1">
                <title>Inner</title>
                <p>Inner cite <xref ref-type="bibr" rid="B2">2</xref>.</p>
            </sec>
        </sec>
        </body>
        "#;

        let sections = extract_sections_enhanced(content);
        assert_eq!(sections.len(), 1);
        let outer = &sections[0];
        assert_eq!(outer.cited_reference_ids, vec!["B1".to_string()]);
        assert_eq!(outer.subsections.len(), 1);
        assert_eq!(
            outer.subsections[0].cited_reference_ids,
            vec!["B2".to_string()]
        );
        // Recursive accessor collects both, in document order.
        let all: Vec<&String> = outer.all_cited_reference_ids();
        assert_eq!(all, vec![&"B1".to_string(), &"B2".to_string()]);
    }
}
