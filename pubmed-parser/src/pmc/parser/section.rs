use super::xml_utils;
use crate::pmc::models::{ArticleSection, Figure, Table};

/// Extract all sections from PMC XML content
pub fn extract_sections_enhanced(content: &str) -> Vec<ArticleSection> {
    let mut sections = Vec::new();

    // First extract all figures from the entire document (including floats, back section, etc.)
    let global_figures = extract_all_figures_from_document(content);

    // Extract abstract first
    if let Some(abstract_section) = extract_abstract_section_enhanced(content) {
        sections.push(abstract_section);
    }

    // Extract body sections with enhanced parsing
    if let Some(body_start) = content.find("<body>") {
        if let Some(body_end) = content[body_start..].find("</body>") {
            let body_content = &content[body_start + 6..body_start + body_end];
            sections.extend(extract_body_sections_enhanced(body_content));
        }
    }

    // Distribute global figures to sections based on references
    distribute_figures_to_sections(&mut sections, &global_figures, content);

    sections
}

/// Enhanced abstract section extraction
fn extract_abstract_section_enhanced(content: &str) -> Option<ArticleSection> {
    if let Some(abstract_start) = content.find("<abstract") {
        if let Some(abstract_end) = content[abstract_start..].find("</abstract>") {
            let abstract_content = &content[abstract_start..abstract_start + abstract_end];

            // Find the actual content start (after the opening tag)
            if let Some(content_start) = abstract_content.find(">") {
                let content_part = &abstract_content[content_start + 1..];

                // Extract figures and tables from abstract
                let figures = extract_figures_from_section(content_part);
                let tables = extract_tables_from_section(content_part);

                let clean_content = xml_utils::strip_xml_tags(content_part);

                if !clean_content.trim().is_empty() {
                    let mut section = ArticleSection::with_title(
                        "abstract".to_string(),
                        "Abstract".to_string(),
                        clean_content,
                    );
                    section.figures = figures;
                    section.tables = tables;
                    return Some(section);
                }
            }
        }
    }
    None
}

/// Extract body sections with enhanced parsing
fn extract_body_sections_enhanced(content: &str) -> Vec<ArticleSection> {
    let mut sections = Vec::new();

    // Extract sections marked with <sec> tags
    let mut pos = 0;
    while let Some(sec_start) = content[pos..].find("<sec") {
        let sec_start = pos + sec_start;
        if let Some(sec_end) = content[sec_start..].find("</sec>") {
            let sec_end = sec_start + sec_end;
            let section_content = &content[sec_start..sec_end];

            if let Some(section) = parse_section_enhanced(section_content) {
                sections.push(section);
            }

            pos = sec_end;
        } else {
            break;
        }
    }

    // If no sections found, extract paragraphs as a single section
    if sections.is_empty() {
        if let Some(body_section) = extract_paragraphs_as_section_enhanced(content) {
            sections.push(body_section);
        }
    }

    sections
}

/// Enhanced section parsing with figures, tables, and nested sections
fn parse_section_enhanced(content: &str) -> Option<ArticleSection> {
    // Extract section ID
    let id = xml_utils::extract_attribute_value(content, "id");

    let title = xml_utils::extract_text_between(content, "<title>", "</title>");

    // Extract content from paragraphs
    let mut section_content = String::new();
    let mut pos = 0;

    while let Some(p_start) = content[pos..].find("<p") {
        let p_start = pos + p_start;
        if let Some(content_start) = content[p_start..].find(">") {
            let content_start = p_start + content_start + 1;
            if let Some(p_end) = content[content_start..].find("</p>") {
                let p_end = content_start + p_end;
                let paragraph = &content[content_start..p_end];
                let clean_text = xml_utils::strip_xml_tags(paragraph);
                if !clean_text.trim().is_empty() {
                    section_content.push_str(&clean_text);
                    section_content.push('\n');
                }
                pos = p_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Extract figures and tables
    let figures = extract_figures_from_section(content);
    let tables = extract_tables_from_section(content);

    // Extract nested subsections
    let subsections = extract_nested_sections(content);

    if !section_content.trim().is_empty()
        || !subsections.is_empty()
        || !figures.is_empty()
        || !tables.is_empty()
    {
        let mut section = match title {
            Some(t) => ArticleSection::with_title(
                "section".to_string(),
                t,
                section_content.trim().to_string(),
            ),
            None => ArticleSection::new("section".to_string(), section_content.trim().to_string()),
        };

        section.id = id;
        section.figures = figures;
        section.tables = tables;
        section.subsections = subsections;

        Some(section)
    } else {
        None
    }
}

/// Extract nested sections within a section
fn extract_nested_sections(content: &str) -> Vec<ArticleSection> {
    let mut sections = Vec::new();

    let mut pos = 0;
    while let Some(sec_start) = content[pos..].find("<sec") {
        let sec_start = pos + sec_start;
        if let Some(sec_end) = content[sec_start..].find("</sec>") {
            let sec_end = sec_start + sec_end;
            let section_content = &content[sec_start..sec_end];

            if let Some(section) = parse_section_enhanced(section_content) {
                sections.push(section);
            }

            pos = sec_end;
        } else {
            break;
        }
    }

    sections
}

/// Extract paragraphs as a single section when no explicit sections are found
fn extract_paragraphs_as_section_enhanced(content: &str) -> Option<ArticleSection> {
    let mut section_content = String::new();
    let mut pos = 0;

    while let Some(p_start) = content[pos..].find("<p") {
        let p_start = pos + p_start;
        if let Some(content_start) = content[p_start..].find(">") {
            let content_start = p_start + content_start + 1;
            if let Some(p_end) = content[content_start..].find("</p>") {
                let p_end = content_start + p_end;
                let paragraph = &content[content_start..p_end];
                let clean_text = xml_utils::strip_xml_tags(paragraph);
                if !clean_text.trim().is_empty() {
                    section_content.push_str(&clean_text);
                    section_content.push('\n');
                }
                pos = p_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if !section_content.trim().is_empty() {
        let mut section =
            ArticleSection::new("body".to_string(), section_content.trim().to_string());

        // Extract figures and tables from the entire content
        section.figures = extract_figures_from_section(content);
        section.tables = extract_tables_from_section(content);

        Some(section)
    } else {
        None
    }
}

/// Extract figures from a section
fn extract_figures_from_section(content: &str) -> Vec<Figure> {
    let mut figures = Vec::new();

    let mut pos = 0;
    while let Some(fig_start) = content[pos..].find("<fig") {
        let fig_start = pos + fig_start;
        if let Some(fig_end) = content[fig_start..].find("</fig>") {
            let fig_end = fig_start + fig_end;
            let fig_content = &content[fig_start..fig_end];

            let id = xml_utils::extract_attribute_value(fig_content, "id").unwrap_or_else(|| {
                let fig_num = figures.len() + 1;
                format!("fig_{fig_num}")
            });

            let label = xml_utils::extract_text_between(fig_content, "<label>", "</label>");
            let caption = xml_utils::extract_text_between(fig_content, "<caption>", "</caption>")
                .map(|c| xml_utils::strip_xml_tags(&c))
                .unwrap_or_else(|| "No caption available".to_string());

            let alt_text =
                xml_utils::extract_text_between(fig_content, "<alt-text>", "</alt-text>");
            let fig_type = xml_utils::extract_attribute_value(fig_content, "fig-type");

            // Extract file name from graphic element
            let file_name = xml_utils::extract_attribute_value(fig_content, "xlink:href")
                .or_else(|| xml_utils::extract_attribute_value(fig_content, "href"));

            let mut figure = Figure::new(id, caption);
            figure.label = label;
            figure.alt_text = alt_text;
            figure.fig_type = fig_type;
            figure.file_name = file_name;

            figures.push(figure);
            pos = fig_end;
        } else {
            break;
        }
    }

    figures
}

/// Extract tables from a section
fn extract_tables_from_section(content: &str) -> Vec<Table> {
    let mut tables = Vec::new();

    let mut pos = 0;
    while let Some(table_start) = content[pos..].find("<table-wrap") {
        let table_start = pos + table_start;
        if let Some(table_end) = content[table_start..].find("</table-wrap>") {
            let table_end = table_start + table_end;
            let table_content = &content[table_start..table_end];

            let id = xml_utils::extract_attribute_value(table_content, "id").unwrap_or_else(|| {
                let table_num = tables.len() + 1;
                format!("table_{table_num}")
            });

            let label = xml_utils::extract_text_between(table_content, "<label>", "</label>");
            let caption = xml_utils::extract_text_between(table_content, "<caption>", "</caption>")
                .map(|c| xml_utils::strip_xml_tags(&c))
                .unwrap_or_else(|| "No caption available".to_string());

            // Note: Table content is not stored in the Table struct

            // Extract table footnotes
            let mut footnotes = Vec::new();
            let table_footnotes = xml_utils::find_all_tags(table_content, "table-wrap-foot");
            for footnote_tag in table_footnotes {
                if let Some(footnote_content) =
                    xml_utils::extract_element_content(&footnote_tag, "table-wrap-foot")
                {
                    let footnote = xml_utils::strip_xml_tags(&footnote_content);
                    if !footnote.trim().is_empty() {
                        footnotes.push(footnote);
                    }
                }
            }

            let mut table = Table::new(id, caption);
            table.label = label;
            table.footnotes = footnotes;

            tables.push(table);
            pos = table_end;
        } else {
            break;
        }
    }

    tables
}

/// Extract section title from section content
pub fn extract_section_title(content: &str) -> Option<String> {
    xml_utils::extract_text_between(content, "<title>", "</title>")
}

/// Extract section ID from section content
pub fn extract_section_id(content: &str) -> Option<String> {
    xml_utils::extract_attribute_value(content, "id")
}

/// Extract all paragraph content from a section
pub fn extract_paragraph_content(content: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut pos = 0;

    while let Some(p_start) = content[pos..].find("<p") {
        let p_start = pos + p_start;
        if let Some(content_start) = content[p_start..].find(">") {
            let content_start = p_start + content_start + 1;
            if let Some(p_end) = content[content_start..].find("</p>") {
                let p_end = content_start + p_end;
                let paragraph = &content[content_start..p_end];
                let clean_text = xml_utils::strip_xml_tags(paragraph);
                if !clean_text.trim().is_empty() {
                    paragraphs.push(clean_text);
                }
                pos = p_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    paragraphs
}

/// Extract all figures from the entire XML document, including floats, back sections, etc.
fn extract_all_figures_from_document(content: &str) -> Vec<Figure> {
    let mut figures = Vec::new();

    let mut pos = 0;
    while let Some(fig_start) = content[pos..].find("<fig") {
        let fig_start = pos + fig_start;
        if let Some(fig_end) = content[fig_start..].find("</fig>") {
            let fig_end = fig_start + fig_end;
            let fig_content = &content[fig_start..fig_end];

            let id = xml_utils::extract_attribute_value(fig_content, "id").unwrap_or_else(|| {
                let fig_num = figures.len() + 1;
                format!("fig_{fig_num}")
            });

            let label = xml_utils::extract_text_between(fig_content, "<label>", "</label>");
            let caption = xml_utils::extract_text_between(fig_content, "<caption>", "</caption>")
                .map(|c| xml_utils::strip_xml_tags(&c))
                .unwrap_or_else(|| "No caption available".to_string());

            let alt_text =
                xml_utils::extract_text_between(fig_content, "<alt-text>", "</alt-text>");
            let fig_type = xml_utils::extract_attribute_value(fig_content, "fig-type");

            // Extract file name from graphic element
            let file_name = xml_utils::extract_attribute_value(fig_content, "xlink:href")
                .or_else(|| xml_utils::extract_attribute_value(fig_content, "href"));

            let mut figure = Figure::new(id, caption);
            figure.label = label;
            figure.alt_text = alt_text;
            figure.fig_type = fig_type;
            figure.file_name = file_name;

            figures.push(figure);
            pos = fig_end;
        } else {
            break;
        }
    }

    figures
}

/// Distribute global figures to sections based on figure references in the text
fn distribute_figures_to_sections(
    sections: &mut Vec<ArticleSection>,
    global_figures: &[Figure],
    _content: &str,
) {
    if global_figures.is_empty() {
        return;
    }

    // For now, add all global figures to the first section if no specific mapping can be determined
    // This could be enhanced in the future to do more sophisticated matching based on xref patterns
    if let Some(first_section) = sections.first_mut() {
        first_section.figures.extend(global_figures.iter().cloned());
    } else if !global_figures.is_empty() {
        // If no sections exist but we have figures, create a special figures section
        let mut figures_section =
            ArticleSection::new("figures".to_string(), "Figures section".to_string());
        figures_section
            .figures
            .extend(global_figures.iter().cloned());
        sections.push(figures_section);
    }
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

        let section = extract_abstract_section_enhanced(content);
        assert!(section.is_some());

        let section = section.unwrap();
        assert_eq!(section.section_type, "abstract");
        assert_eq!(section.title, Some("Abstract".to_string()));
        assert!(section.content.contains("This is an abstract paragraph."));
    }

    #[test]
    fn test_extract_section_title() {
        let content = r#"<sec id="sec1"><title>Introduction</title><p>Content</p></sec>"#;
        let title = extract_section_title(content);
        assert_eq!(title, Some("Introduction".to_string()));
    }

    #[test]
    fn test_extract_section_id() {
        let content = r#"<sec id="sec1"><title>Introduction</title><p>Content</p></sec>"#;
        let id = extract_section_id(content);
        assert_eq!(id, Some("sec1".to_string()));
    }

    #[test]
    fn test_extract_paragraph_content() {
        let content = r#"
        <p>First paragraph.</p>
        <p>Second paragraph with <em>emphasis</em>.</p>
        "#;

        let paragraphs = extract_paragraph_content(content);
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0], "First paragraph.");
        assert_eq!(paragraphs[1], "Second paragraph with emphasis.");
    }

    #[test]
    fn test_extract_figures_from_section() {
        let content = r#"
        <fig id="fig1" fig-type="diagram">
            <label>Figure 1</label>
            <caption>This is a test figure.</caption>
            <alt-text>Alternative text</alt-text>
        </fig>
        "#;

        let figures = extract_figures_from_section(content);
        assert_eq!(figures.len(), 1);
        assert_eq!(figures[0].id, "fig1");
        assert_eq!(figures[0].label, Some("Figure 1".to_string()));
        assert_eq!(figures[0].caption, "This is a test figure.");
        assert_eq!(figures[0].alt_text, Some("Alternative text".to_string()));
        assert_eq!(figures[0].fig_type, Some("diagram".to_string()));
    }

    #[test]
    fn test_extract_tables_from_section() {
        let content = r#"
        <table-wrap id="table1">
            <label>Table 1</label>
            <caption>This is a test table.</caption>
            <table>
                <tr><th>Header</th></tr>
                <tr><td>Data</td></tr>
            </table>
        </table-wrap>
        "#;

        let tables = extract_tables_from_section(content);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].id, "table1");
        assert_eq!(tables[0].label, Some("Table 1".to_string()));
        assert_eq!(tables[0].caption, "This is a test table.");
    }
}
