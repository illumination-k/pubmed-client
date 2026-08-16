//! JATS `<fig>` parsing.

use crate::pmc::domain::Figure;
use crate::pmc::parser::reader_utils::{get_attr, read_text_content, skip_element};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use tracing::warn;

/// Attributes lifted off a `<fig>` start event before the reader moves on.
///
/// The reader borrows from its own buffer, so the attributes have to be owned
/// before the element body is consumed.
pub(super) struct FigAttrs {
    id: Option<String>,
    fig_type: Option<String>,
}

impl FigAttrs {
    /// Capture the attributes of a `<fig>` start event.
    pub(super) fn from_start(e: &BytesStart) -> Self {
        Self {
            id: get_attr(e, b"id"),
            fig_type: get_attr(e, b"fig-type"),
        }
    }
}

/// Extract all `<fig>` elements from content using Reader.
/// Scans the entire content string regardless of nesting depth.
pub(super) fn extract_figures_from_content(content: &str) -> Vec<Figure> {
    super::scan_elements(content, b"fig", FigAttrs::from_start, parse_figure_inner)
}

/// Parse figure content after `Event::Start` for `<fig>` has been consumed.
pub(super) fn parse_figure_inner(
    reader: &mut quick_xml::Reader<&[u8]>,
    attrs: FigAttrs,
) -> Option<Figure> {
    let mut label: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut alt_text: Option<String> = None;
    let mut file_name: Option<String> = None;

    loop {
        let action = match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"label" => FigAction::ReadLabel,
                b"caption" => FigAction::ReadCaption,
                b"alt-text" => FigAction::ReadAltText,
                b"graphic" => {
                    let href = get_attr(e, b"xlink:href").or_else(|| get_attr(e, b"href"));
                    FigAction::ReadGraphic(href)
                }
                other => FigAction::Skip(other.to_vec()),
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"fig" => FigAction::Done,
            Ok(Event::Eof) => FigAction::Done,
            Err(_) => FigAction::Done,
            _ => FigAction::Continue,
        };

        match action {
            FigAction::ReadLabel => {
                label = read_text_content(reader, b"label").ok();
            }
            FigAction::ReadCaption => {
                caption = match read_text_content(reader, b"caption") {
                    Ok(text) => Some(text),
                    Err(e) => {
                        warn!(
                            figure_id = ?attrs.id,
                            error = %e,
                            "failed to parse figure caption"
                        );
                        None
                    }
                };
            }
            FigAction::ReadAltText => {
                alt_text = read_text_content(reader, b"alt-text").ok();
            }
            FigAction::ReadGraphic(href) => {
                file_name = href;
                let _ = skip_element(reader, QName(b"graphic"));
            }
            FigAction::Skip(name) => {
                let _ = skip_element(reader, QName(&name));
            }
            FigAction::Done => break,
            FigAction::Continue => {}
        }
    }

    let id = match attrs.id {
        Some(id) => id,
        None => {
            warn!("figure element missing id attribute");
            format!("fig_unknown_{}", line!())
        }
    };
    Some(Figure {
        id,
        label,
        caption,
        alt_text,
        fig_type: attrs.fig_type,
        graphic_href: file_name,
    })
}

enum FigAction {
    Continue,
    Done,
    ReadLabel,
    ReadCaption,
    ReadAltText,
    ReadGraphic(Option<String>),
    Skip(Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_figures_from_section() {
        let content = r#"
        <fig id="fig1" fig-type="diagram">
            <label>Figure 1</label>
            <caption>This is a test figure.</caption>
            <alt-text>Alternative text</alt-text>
        </fig>
        "#;

        let figures = extract_figures_from_content(content);
        assert_eq!(figures.len(), 1);
        assert_eq!(figures[0].id, "fig1");
        assert_eq!(figures[0].label, Some("Figure 1".to_string()));
        assert_eq!(
            figures[0].caption.as_deref(),
            Some("This is a test figure.")
        );
        assert_eq!(figures[0].alt_text, Some("Alternative text".to_string()));
        assert_eq!(figures[0].fig_type, Some("diagram".to_string()));
    }
}
