use crate::error::Result;
use crate::pmc::models::{ArticleSection, PmcFullText, Reference};

/// XML parser for PMC articles
pub struct PmcXmlParser;

impl PmcXmlParser {
    /// Parse PMC XML content into structured data
    pub fn parse(xml_content: &str, pmcid: &str) -> Result<PmcFullText> {
        let parser = Self;

        // Extract title
        let title = parser
            .extract_text_between(xml_content, "<article-title>", "</article-title>")
            .unwrap_or_else(|| "Unknown Title".to_string());

        // Extract authors
        let authors = parser.extract_authors(xml_content);

        // Extract journal
        let journal = parser
            .extract_text_between(xml_content, "<journal-title>", "</journal-title>")
            .unwrap_or_else(|| "Unknown Journal".to_string());

        // Extract publication date
        let pub_date = parser.extract_pub_date(xml_content);

        // Extract DOI
        let doi = parser.extract_doi(xml_content);

        // Extract PMID
        let pmid = parser.extract_pmid(xml_content);

        // Extract sections
        let sections = parser.extract_sections(xml_content);

        // Extract references
        let references = parser.extract_references(xml_content);

        Ok(PmcFullText {
            pmcid: pmcid.to_string(),
            pmid,
            title,
            authors,
            journal,
            pub_date,
            doi,
            sections,
            references,
            article_type: None, // Article type extraction would require more complex XML parsing
        })
    }

    /// Extract text between two XML tags
    fn extract_text_between(&self, content: &str, start: &str, end: &str) -> Option<String> {
        let start_pos = content.find(start)? + start.len();
        let end_pos = content[start_pos..].find(end)? + start_pos;
        Some(content[start_pos..end_pos].trim().to_string())
    }

    /// Extract authors from contributor group
    fn extract_authors(&self, content: &str) -> Vec<String> {
        let mut authors = Vec::new();

        if let Some(contrib_start) = content.find("<contrib-group>") {
            if let Some(contrib_end) = content[contrib_start..].find("</contrib-group>") {
                let contrib_section = &content[contrib_start..contrib_start + contrib_end];

                let mut pos = 0;
                while let Some(surname_start) = contrib_section[pos..].find("<surname>") {
                    let surname_start = pos + surname_start + 9;
                    if let Some(surname_end) = contrib_section[surname_start..].find("</surname>") {
                        let surname_end = surname_start + surname_end;
                        let surname = &contrib_section[surname_start..surname_end];

                        if let Some(given_start) =
                            contrib_section[surname_end..].find("<given-names")
                        {
                            let given_start = surname_end + given_start;
                            if let Some(given_content_start) =
                                contrib_section[given_start..].find(">")
                            {
                                let given_content_start = given_start + given_content_start + 1;
                                if let Some(given_end) =
                                    contrib_section[given_content_start..].find("</given-names>")
                                {
                                    let given_end = given_content_start + given_end;
                                    let given_names =
                                        &contrib_section[given_content_start..given_end];
                                    authors.push(format!("{} {}", given_names, surname));
                                    pos = given_end;
                                    continue;
                                }
                            }
                        }

                        authors.push(surname.to_string());
                        pos = surname_end;
                    } else {
                        break;
                    }
                }
            }
        }

        authors
    }

    /// Extract publication date
    fn extract_pub_date(&self, content: &str) -> String {
        if let Some(year) = self.extract_text_between(content, "<year>", "</year>") {
            if let Some(month) = self.extract_text_between(content, "<month>", "</month>") {
                if let Some(day) = self.extract_text_between(content, "<day>", "</day>") {
                    return format!(
                        "{}-{:02}-{:02}",
                        year,
                        month.parse::<u32>().unwrap_or(1),
                        day.parse::<u32>().unwrap_or(1)
                    );
                }
                return format!("{}-{:02}", year, month.parse::<u32>().unwrap_or(1));
            }
            return year;
        }
        "Unknown Date".to_string()
    }

    /// Extract DOI from article metadata
    fn extract_doi(&self, content: &str) -> Option<String> {
        let mut pos = 0;
        while let Some(id_start) = content[pos..].find(r#"<article-id pub-id-type="doi""#) {
            let id_start = pos + id_start;
            if let Some(content_start) = content[id_start..].find(">") {
                let content_start = id_start + content_start + 1;
                if let Some(content_end) = content[content_start..].find("</article-id>") {
                    let content_end = content_start + content_end;
                    return Some(content[content_start..content_end].trim().to_string());
                }
            }
            pos = id_start + 1;
        }
        None
    }

    /// Extract PMID from article metadata
    fn extract_pmid(&self, content: &str) -> Option<String> {
        let mut pos = 0;
        while let Some(id_start) = content[pos..].find(r#"<article-id pub-id-type="pmid""#) {
            let id_start = pos + id_start;
            if let Some(content_start) = content[id_start..].find(">") {
                let content_start = id_start + content_start + 1;
                if let Some(content_end) = content[content_start..].find("</article-id>") {
                    let content_end = content_start + content_end;
                    return Some(content[content_start..content_end].trim().to_string());
                }
            }
            pos = id_start + 1;
        }
        None
    }

    /// Extract article sections from body
    fn extract_sections(&self, content: &str) -> Vec<ArticleSection> {
        let mut sections = Vec::new();

        // Extract abstract first
        if let Some(abstract_section) = self.extract_abstract_section(content) {
            sections.push(abstract_section);
        }

        // Extract body sections
        if let Some(body_start) = content.find("<body>") {
            if let Some(body_end) = content[body_start..].find("</body>") {
                let body_content = &content[body_start + 6..body_start + body_end];
                sections.extend(self.extract_body_sections(body_content));
            }
        }

        sections
    }

    /// Extract abstract section
    fn extract_abstract_section(&self, content: &str) -> Option<ArticleSection> {
        if let Some(abstract_start) = content.find("<abstract") {
            if let Some(abstract_end) = content[abstract_start..].find("</abstract>") {
                let abstract_content = &content[abstract_start..abstract_start + abstract_end];

                // Find the actual content start (after the opening tag)
                if let Some(content_start) = abstract_content.find(">") {
                    let content_part = &abstract_content[content_start + 1..];
                    let clean_content = self.strip_xml_tags(content_part);

                    if !clean_content.trim().is_empty() {
                        return Some(ArticleSection::with_title(
                            "abstract".to_string(),
                            "Abstract".to_string(),
                            clean_content,
                        ));
                    }
                }
            }
        }
        None
    }

    /// Extract sections from body content
    fn extract_body_sections(&self, content: &str) -> Vec<ArticleSection> {
        let mut sections = Vec::new();

        // Extract sections marked with <sec> tags
        let mut pos = 0;
        while let Some(sec_start) = content[pos..].find("<sec") {
            let sec_start = pos + sec_start;
            if let Some(sec_end) = content[sec_start..].find("</sec>") {
                let sec_end = sec_start + sec_end;
                let section_content = &content[sec_start..sec_end];

                if let Some(section) = self.parse_section(section_content) {
                    sections.push(section);
                }

                pos = sec_end;
            } else {
                break;
            }
        }

        // If no sections found, extract paragraphs as a single section
        if sections.is_empty() {
            if let Some(body_section) = self.extract_paragraphs_as_section(content) {
                sections.push(body_section);
            }
        }

        sections
    }

    /// Parse a single section
    fn parse_section(&self, content: &str) -> Option<ArticleSection> {
        let title = self.extract_text_between(content, "<title>", "</title>");

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
                    let clean_text = self.strip_xml_tags(paragraph);
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
            Some(match title {
                Some(t) => ArticleSection::with_title(
                    "section".to_string(),
                    t,
                    section_content.trim().to_string(),
                ),
                None => {
                    ArticleSection::new("section".to_string(), section_content.trim().to_string())
                }
            })
        } else {
            None
        }
    }

    /// Extract paragraphs as a single section
    fn extract_paragraphs_as_section(&self, content: &str) -> Option<ArticleSection> {
        let mut para_content = String::new();
        let mut pos = 0;

        while let Some(p_start) = content[pos..].find("<p") {
            let p_start = pos + p_start;
            if let Some(content_start) = content[p_start..].find(">") {
                let content_start = p_start + content_start + 1;
                if let Some(p_end) = content[content_start..].find("</p>") {
                    let p_end = content_start + p_end;
                    let paragraph = &content[content_start..p_end];
                    let clean_text = self.strip_xml_tags(paragraph);
                    if !clean_text.trim().is_empty() {
                        para_content.push_str(&clean_text);
                        para_content.push('\n');
                    }
                    pos = p_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if !para_content.trim().is_empty() {
            Some(ArticleSection::with_title(
                "body".to_string(),
                "Main Content".to_string(),
                para_content.trim().to_string(),
            ))
        } else {
            None
        }
    }

    /// Extract references from reference list
    fn extract_references(&self, content: &str) -> Vec<Reference> {
        let mut references = Vec::new();

        if let Some(ref_start) = content.find("<ref-list") {
            if let Some(ref_end) = content[ref_start..].find("</ref-list>") {
                let ref_content = &content[ref_start..ref_start + ref_end];

                let mut pos = 0;
                while let Some(ref_start) = ref_content[pos..].find("<ref id=\"") {
                    let ref_start = pos + ref_start;
                    if let Some(id_end) = ref_content[ref_start + 9..].find("\"") {
                        let id = ref_content[ref_start + 9..ref_start + 9 + id_end].to_string();

                        if let Some(ref_end) = ref_content[ref_start..].find("</ref>") {
                            let ref_section = &ref_content[ref_start..ref_start + ref_end];

                            let title = self.extract_text_between(
                                ref_section,
                                "<article-title>",
                                "</article-title>",
                            );
                            let journal =
                                self.extract_text_between(ref_section, "<source>", "</source>");
                            let year = self.extract_text_between(ref_section, "<year>", "</year>");

                            let mut reference = Reference::new(id);
                            reference.title = title;
                            reference.journal = journal;
                            reference.year = year;

                            references.push(reference);
                            pos = ref_start + ref_end;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        references
    }

    /// Strip XML tags from content
    fn strip_xml_tags(&self, content: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;

        for ch in content.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ if !in_tag => result.push(ch),
                _ => {}
            }
        }

        result.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_between() {
        let parser = PmcXmlParser;
        let content = "<title>Test Title</title>";
        let result = parser.extract_text_between(content, "<title>", "</title>");
        assert_eq!(result, Some("Test Title".to_string()));
    }

    #[test]
    fn test_strip_xml_tags() {
        let parser = PmcXmlParser;
        let content = "This is <bold>bold</bold> text with <italic>italic</italic>.";
        let result = parser.strip_xml_tags(content);
        assert_eq!(result, "This is bold text with italic.");
    }

    #[test]
    fn test_extract_pub_date() {
        let parser = PmcXmlParser;
        let content = "<year>2023</year><month>12</month><day>25</day>";
        let result = parser.extract_pub_date(content);
        assert_eq!(result, "2023-12-25");
    }
}
