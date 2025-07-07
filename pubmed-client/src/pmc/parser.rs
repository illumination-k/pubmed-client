use crate::error::Result;
use crate::pmc::models::{
    ArticleSection, Author, Figure, FundingInfo, JournalInfo, PmcFullText, Reference,
    SupplementaryMaterial, Table,
};

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

        // Extract authors with detailed information
        let authors = parser.extract_authors_detailed(xml_content);

        // Extract comprehensive journal information
        let journal = parser.extract_journal_info(xml_content);

        // Extract publication date
        let pub_date = parser.extract_pub_date(xml_content);

        // Extract DOI
        let doi = parser.extract_doi(xml_content);

        // Extract PMID
        let pmid = parser.extract_pmid(xml_content);

        // Extract article type
        let article_type = parser.extract_article_type(xml_content);

        // Extract keywords
        let keywords = parser.extract_keywords(xml_content);

        // Extract funding information
        let funding = parser.extract_funding(xml_content);

        // Extract conflict of interest
        let conflict_of_interest = parser.extract_conflict_of_interest(xml_content);

        // Extract acknowledgments
        let acknowledgments = parser.extract_acknowledgments(xml_content);

        // Extract data availability
        let data_availability = parser.extract_data_availability(xml_content);

        // Extract sections with enhanced parsing
        let sections = parser.extract_sections_enhanced(xml_content);

        // Extract references with detailed information
        let references = parser.extract_references_detailed(xml_content);

        // Extract supplementary materials
        let supplementary_materials = parser.extract_supplementary_materials(xml_content);

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
            article_type,
            keywords,
            funding,
            conflict_of_interest,
            acknowledgments,
            data_availability,
            supplementary_materials,
        })
    }

    /// Extract text between two XML tags
    fn extract_text_between(&self, content: &str, start: &str, end: &str) -> Option<String> {
        let start_pos = content.find(start)? + start.len();
        let end_pos = content[start_pos..].find(end)? + start_pos;
        Some(content[start_pos..end_pos].trim().to_string())
    }

    /// Extract authors from contributor group (legacy method for compatibility)
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
                                    authors.push(format!("{given_names} {surname}"));
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

    /// Extract detailed author information with affiliations and ORCID
    fn extract_authors_detailed(&self, content: &str) -> Vec<Author> {
        let mut authors = Vec::new();

        if let Some(contrib_start) = content.find("<contrib-group>") {
            if let Some(contrib_end) = content[contrib_start..].find("</contrib-group>") {
                let contrib_section = &content[contrib_start..contrib_start + contrib_end];

                let mut pos = 0;
                while let Some(contrib_start) = contrib_section[pos..].find("<contrib") {
                    let contrib_start = pos + contrib_start;
                    if let Some(contrib_end) = contrib_section[contrib_start..].find("</contrib>") {
                        let contrib_end = contrib_start + contrib_end;
                        let contrib_content = &contrib_section[contrib_start..contrib_end];

                        if let Some(author) = self.parse_single_author(contrib_content) {
                            authors.push(author);
                        }

                        pos = contrib_end;
                    } else {
                        break;
                    }
                }
            }
        }

        // Fallback to simple author extraction if detailed extraction fails
        if authors.is_empty() {
            let simple_authors = self.extract_authors(content);
            authors = simple_authors.into_iter().map(Author::new).collect();
        }

        authors
    }

    /// Parse a single author from contrib XML
    fn parse_single_author(&self, contrib_content: &str) -> Option<Author> {
        let surname = self.extract_text_between(contrib_content, "<surname>", "</surname>");
        let given_names = self
            .extract_text_between(contrib_content, "<given-names>", "</given-names>")
            .or_else(|| {
                // Handle self-closing given-names tag
                if let Some(start) = contrib_content.find("<given-names") {
                    if let Some(end) = contrib_content[start..].find(">") {
                        let tag_content = &contrib_content[start..start + end + 1];
                        if tag_content.contains("/>") {
                            return None; // Self-closing tag with no content
                        }
                    }
                }
                None
            });

        let mut author = Author::with_names(given_names, surname);

        // Extract ORCID
        if let Some(orcid_start) = contrib_content.find("https://orcid.org/") {
            if let Some(orcid_end) = contrib_content[orcid_start..].find('"') {
                let orcid = contrib_content[orcid_start..orcid_start + orcid_end].to_string();
                author.orcid = Some(orcid);
            }
        }

        // Extract email
        author.email = self
            .extract_text_between(contrib_content, "<email", "</email>")
            .and_then(|email_content| {
                // Extract actual email from the tag content
                email_content
                    .find(">")
                    .map(|start| email_content[start + 1..].to_string())
            });

        // Check if corresponding author
        author.is_corresponding = contrib_content.contains("corresp=\"yes\"");

        // Extract roles
        let mut roles = Vec::new();
        let mut pos = 0;
        while let Some(role_start) = contrib_content[pos..].find("<role") {
            let role_start = pos + role_start;
            if let Some(role_end) = contrib_content[role_start..].find("</role>") {
                let role_end = role_start + role_end;
                let role_section = &contrib_content[role_start..role_end];

                if let Some(content_start) = role_section.find(">") {
                    let role_content = &role_section[content_start + 1..];
                    if !role_content.trim().is_empty() {
                        roles.push(role_content.trim().to_string());
                    }
                }
                pos = role_end;
            } else {
                break;
            }
        }
        author.roles = roles;

        Some(author)
    }

    /// Extract comprehensive journal information
    fn extract_journal_info(&self, content: &str) -> JournalInfo {
        let mut journal = JournalInfo::new(
            self.extract_text_between(content, "<journal-title>", "</journal-title>")
                .unwrap_or_else(|| "Unknown Journal".to_string()),
        );

        // Extract journal abbreviation
        journal.abbreviation = self.extract_text_between(
            content,
            "<journal-id journal-id-type=\"iso-abbrev\">",
            "</journal-id>",
        );

        // Extract ISSNs
        let mut pos = 0;
        while let Some(issn_start) = content[pos..].find("<issn") {
            let issn_start = pos + issn_start;
            if let Some(issn_end) = content[issn_start..].find("</issn>") {
                let issn_end = issn_start + issn_end;
                let issn_section = &content[issn_start..issn_end];

                if let Some(content_start) = issn_section.find(">") {
                    let issn_value = &issn_section[content_start + 1..];

                    if issn_section.contains("pub-type=\"epub\"") {
                        journal.issn_electronic = Some(issn_value.to_string());
                    } else if issn_section.contains("pub-type=\"ppub\"") {
                        journal.issn_print = Some(issn_value.to_string());
                    }
                }
                pos = issn_end;
            } else {
                break;
            }
        }

        // Extract publisher
        journal.publisher =
            self.extract_text_between(content, "<publisher-name>", "</publisher-name>");

        // Extract volume and issue
        journal.volume = self.extract_text_between(content, "<volume>", "</volume>");
        journal.issue = self.extract_text_between(content, "<issue>", "</issue>");

        journal
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

    /// Extract article type
    fn extract_article_type(&self, content: &str) -> Option<String> {
        // Look for article-type attribute in article tag
        if let Some(article_start) = content.find("<article") {
            if let Some(article_end) = content[article_start..].find(">") {
                let article_tag = &content[article_start..article_start + article_end];
                if let Some(type_start) = article_tag.find("article-type=\"") {
                    let type_start = type_start + 14; // Length of "article-type=\""
                    if let Some(type_end) = article_tag[type_start..].find('"') {
                        return Some(article_tag[type_start..type_start + type_end].to_string());
                    }
                }
            }
        }

        // Fallback: look in article-categories
        self.extract_text_between(content, "<subject>", "</subject>")
    }

    /// Extract keywords
    fn extract_keywords(&self, content: &str) -> Vec<String> {
        let mut keywords = Vec::new();

        if let Some(kwd_start) = content.find("<kwd-group") {
            if let Some(kwd_end) = content[kwd_start..].find("</kwd-group>") {
                let kwd_section = &content[kwd_start..kwd_start + kwd_end];

                let mut pos = 0;
                while let Some(kwd_start) = kwd_section[pos..].find("<kwd>") {
                    let kwd_start = pos + kwd_start + 5; // Length of "<kwd>"
                    if let Some(kwd_end) = kwd_section[kwd_start..].find("</kwd>") {
                        let keyword = kwd_section[kwd_start..kwd_start + kwd_end]
                            .trim()
                            .to_string();
                        if !keyword.is_empty() {
                            keywords.push(keyword);
                        }
                        pos = kwd_start + kwd_end;
                    } else {
                        break;
                    }
                }
            }
        }

        keywords
    }

    /// Extract funding information
    fn extract_funding(&self, content: &str) -> Vec<FundingInfo> {
        let mut funding = Vec::new();

        if let Some(funding_start) = content.find("<funding-group>") {
            if let Some(funding_end) = content[funding_start..].find("</funding-group>") {
                let funding_section = &content[funding_start..funding_start + funding_end];

                let mut pos = 0;
                while let Some(award_start) = funding_section[pos..].find("<award-group") {
                    let award_start = pos + award_start;
                    if let Some(award_end) = funding_section[award_start..].find("</award-group>") {
                        let award_end = award_start + award_end;
                        let award_section = &funding_section[award_start..award_end];

                        let source = self
                            .extract_text_between(
                                award_section,
                                "<funding-source>",
                                "</funding-source>",
                            )
                            .unwrap_or_else(|| "Unknown Source".to_string());

                        let mut funding_info = FundingInfo::new(source);
                        funding_info.award_id =
                            self.extract_text_between(award_section, "<award-id>", "</award-id>");

                        funding.push(funding_info);
                        pos = award_end;
                    } else {
                        break;
                    }
                }

                // Extract funding statements
                if let Some(statement) = self.extract_text_between(
                    funding_section,
                    "<funding-statement>",
                    "</funding-statement>",
                ) {
                    if funding.is_empty() {
                        let mut funding_info = FundingInfo::new("General Funding".to_string());
                        funding_info.statement = Some(statement);
                        funding.push(funding_info);
                    } else {
                        funding[0].statement = Some(statement);
                    }
                }
            }
        }

        funding
    }

    /// Extract conflict of interest statement
    fn extract_conflict_of_interest(&self, content: &str) -> Option<String> {
        // Look for COI statement in various locations
        if let Some(coi) =
            self.extract_text_between(content, "<fn fn-type=\"COI-statement\">", "</fn>")
        {
            return Some(self.strip_xml_tags(&coi));
        }

        if let Some(coi) = self.extract_text_between(content, "<fn fn-type=\"conflict\">", "</fn>")
        {
            return Some(self.strip_xml_tags(&coi));
        }

        // Look in notes section
        if let Some(notes_start) = content.find("<author-notes>") {
            if let Some(notes_end) = content[notes_start..].find("</author-notes>") {
                let notes_section = &content[notes_start..notes_start + notes_end];
                if let Some(coi) = self.extract_text_between(notes_section, "<fn", "</fn>") {
                    if coi.to_lowercase().contains("conflict")
                        || coi.to_lowercase().contains("competing")
                    {
                        return Some(self.strip_xml_tags(&coi));
                    }
                }
            }
        }

        None
    }

    /// Extract acknowledgments
    fn extract_acknowledgments(&self, content: &str) -> Option<String> {
        self.extract_text_between(content, "<ack>", "</ack>")
            .map(|ack| self.strip_xml_tags(&ack))
    }

    /// Extract data availability statement
    fn extract_data_availability(&self, content: &str) -> Option<String> {
        // Look for data availability in various sections
        if let Some(data_avail) =
            self.extract_text_between(content, "<sec sec-type=\"data-availability\">", "</sec>")
        {
            return Some(self.strip_xml_tags(&data_avail));
        }

        // Look in supplementary material
        if let Some(supp_start) = content.find("<supplementary-material") {
            if let Some(supp_end) = content[supp_start..].find("</supplementary-material>") {
                let supp_section = &content[supp_start..supp_start + supp_end];
                if supp_section.to_lowercase().contains("data") {
                    return Some(self.strip_xml_tags(supp_section));
                }
            }
        }

        None
    }

    /// Enhanced section extraction with nested sections and rich content
    fn extract_sections_enhanced(&self, content: &str) -> Vec<ArticleSection> {
        let mut sections = Vec::new();

        // Extract abstract first
        if let Some(abstract_section) = self.extract_abstract_section_enhanced(content) {
            sections.push(abstract_section);
        }

        // Extract body sections with enhanced parsing
        if let Some(body_start) = content.find("<body>") {
            if let Some(body_end) = content[body_start..].find("</body>") {
                let body_content = &content[body_start + 6..body_start + body_end];
                sections.extend(self.extract_body_sections_enhanced(body_content));
            }
        }

        sections
    }

    /// Enhanced abstract section extraction
    fn extract_abstract_section_enhanced(&self, content: &str) -> Option<ArticleSection> {
        if let Some(abstract_start) = content.find("<abstract") {
            if let Some(abstract_end) = content[abstract_start..].find("</abstract>") {
                let abstract_content = &content[abstract_start..abstract_start + abstract_end];

                // Find the actual content start (after the opening tag)
                if let Some(content_start) = abstract_content.find(">") {
                    let content_part = &abstract_content[content_start + 1..];

                    // Extract figures and tables from abstract
                    let figures = self.extract_figures_from_section(content_part);
                    let tables = self.extract_tables_from_section(content_part);

                    let clean_content = self.strip_xml_tags(content_part);

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

    /// Enhanced body sections extraction with nested sections
    fn extract_body_sections_enhanced(&self, content: &str) -> Vec<ArticleSection> {
        let mut sections = Vec::new();

        // Extract sections marked with <sec> tags
        let mut pos = 0;
        while let Some(sec_start) = content[pos..].find("<sec") {
            let sec_start = pos + sec_start;
            if let Some(sec_end) = content[sec_start..].find("</sec>") {
                let sec_end = sec_start + sec_end;
                let section_content = &content[sec_start..sec_end];

                if let Some(section) = self.parse_section_enhanced(section_content) {
                    sections.push(section);
                }

                pos = sec_end;
            } else {
                break;
            }
        }

        // If no sections found, extract paragraphs as a single section
        if sections.is_empty() {
            if let Some(body_section) = self.extract_paragraphs_as_section_enhanced(content) {
                sections.push(body_section);
            }
        }

        sections
    }

    /// Enhanced section parsing with figures, tables, and nested sections
    fn parse_section_enhanced(&self, content: &str) -> Option<ArticleSection> {
        // Extract section ID
        let id = self.extract_attribute_value(content, "id");

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

        // Extract figures and tables
        let figures = self.extract_figures_from_section(content);
        let tables = self.extract_tables_from_section(content);

        // Extract nested subsections
        let subsections = self.extract_nested_sections(content);

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
                None => {
                    ArticleSection::new("section".to_string(), section_content.trim().to_string())
                }
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
    fn extract_nested_sections(&self, content: &str) -> Vec<ArticleSection> {
        let mut nested_sections = Vec::new();

        // Look for nested <sec> tags
        let mut depth = 0;
        let mut start_pos = None;

        for (i, _) in content.char_indices() {
            if content[i..].starts_with("<sec") {
                if depth == 0 {
                    start_pos = Some(i);
                }
                depth += 1;
            } else if content[i..].starts_with("</sec>") {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start_pos {
                        let nested_content = &content[start..i + 6]; // +6 for "</sec>"
                        if let Some(nested_section) = self.parse_section_enhanced(nested_content) {
                            nested_sections.push(nested_section);
                        }
                    }
                }
            }
        }

        nested_sections
    }

    /// Extract paragraphs as enhanced section
    fn extract_paragraphs_as_section_enhanced(&self, content: &str) -> Option<ArticleSection> {
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
            let mut section = ArticleSection::with_title(
                "body".to_string(),
                "Main Content".to_string(),
                para_content.trim().to_string(),
            );

            // Extract figures and tables from the entire content
            section.figures = self.extract_figures_from_section(content);
            section.tables = self.extract_tables_from_section(content);

            Some(section)
        } else {
            None
        }
    }

    /// Extract figures from a section
    fn extract_figures_from_section(&self, content: &str) -> Vec<Figure> {
        let mut figures = Vec::new();

        let mut pos = 0;
        while let Some(fig_start) = content[pos..].find("<fig") {
            let fig_start = pos + fig_start;
            if let Some(fig_end) = content[fig_start..].find("</fig>") {
                let fig_end = fig_start + fig_end;
                let fig_content = &content[fig_start..fig_end];

                let id = self
                    .extract_attribute_value(fig_content, "id")
                    .unwrap_or_else(|| {
                        let fig_num = figures.len() + 1;
                        format!("fig_{fig_num}")
                    });

                let label = self.extract_text_between(fig_content, "<label>", "</label>");
                let caption = self
                    .extract_text_between(fig_content, "<caption>", "</caption>")
                    .map(|c| self.strip_xml_tags(&c))
                    .unwrap_or_else(|| "No caption available".to_string());

                let alt_text = self.extract_text_between(fig_content, "<alt-text>", "</alt-text>");
                let fig_type = self.extract_attribute_value(fig_content, "fig-type");

                let mut figure = Figure::new(id, caption);
                figure.label = label;
                figure.alt_text = alt_text;
                figure.fig_type = fig_type;

                figures.push(figure);
                pos = fig_end;
            } else {
                break;
            }
        }

        figures
    }

    /// Extract tables from a section
    fn extract_tables_from_section(&self, content: &str) -> Vec<Table> {
        let mut tables = Vec::new();

        let mut pos = 0;
        while let Some(table_start) = content[pos..].find("<table-wrap") {
            let table_start = pos + table_start;
            if let Some(table_end) = content[table_start..].find("</table-wrap>") {
                let table_end = table_start + table_end;
                let table_content = &content[table_start..table_end];

                let id = self
                    .extract_attribute_value(table_content, "id")
                    .unwrap_or_else(|| {
                        let table_num = tables.len() + 1;
                        format!("table_{table_num}")
                    });

                let label = self.extract_text_between(table_content, "<label>", "</label>");
                let caption = self
                    .extract_text_between(table_content, "<caption>", "</caption>")
                    .map(|c| self.strip_xml_tags(&c))
                    .unwrap_or_else(|| "No caption available".to_string());

                // Extract table footnotes
                let mut footnotes = Vec::new();
                let mut fn_pos = 0;
                while let Some(fn_start) = table_content[fn_pos..].find("<table-wrap-foot") {
                    let fn_start = fn_pos + fn_start;
                    if let Some(fn_end) = table_content[fn_start..].find("</table-wrap-foot>") {
                        let fn_end = fn_start + fn_end;
                        let fn_content = &table_content[fn_start..fn_end];
                        let footnote = self.strip_xml_tags(fn_content);
                        if !footnote.trim().is_empty() {
                            footnotes.push(footnote);
                        }
                        fn_pos = fn_end;
                    } else {
                        break;
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

    /// Extract enhanced reference information
    fn extract_references_detailed(&self, content: &str) -> Vec<Reference> {
        let mut references = Vec::new();

        if let Some(ref_start) = content.find("<ref-list") {
            if let Some(ref_end) = content[ref_start..].find("</ref-list>") {
                let ref_content = &content[ref_start..ref_start + ref_end];

                let mut pos = 0;
                while let Some(ref_start) = ref_content[pos..].find("<ref id=\"") {
                    let ref_start = pos + ref_start;
                    if let Some(id_end) = ref_content[ref_start + 9..].find('"') {
                        let id = ref_content[ref_start + 9..ref_start + 9 + id_end].to_string();

                        if let Some(ref_end) = ref_content[ref_start..].find("</ref>") {
                            let ref_section = &ref_content[ref_start..ref_start + ref_end];

                            let reference = self.parse_detailed_reference(ref_section, id);
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

    /// Parse detailed reference information
    fn parse_detailed_reference(&self, ref_content: &str, id: String) -> Reference {
        let mut reference = Reference::new(id);

        // Extract title
        reference.title =
            self.extract_text_between(ref_content, "<article-title>", "</article-title>");

        // Extract journal
        reference.journal = self.extract_text_between(ref_content, "<source>", "</source>");

        // Extract year
        reference.year = self.extract_text_between(ref_content, "<year>", "</year>");

        // Extract volume
        reference.volume = self.extract_text_between(ref_content, "<volume>", "</volume>");

        // Extract issue
        reference.issue = self.extract_text_between(ref_content, "<issue>", "</issue>");

        // Extract pages
        if let Some(fpage) = self.extract_text_between(ref_content, "<fpage>", "</fpage>") {
            if let Some(lpage) = self.extract_text_between(ref_content, "<lpage>", "</lpage>") {
                reference.pages = Some(format!("{fpage}-{lpage}"));
            } else {
                reference.pages = Some(fpage);
            }
        }

        // Extract DOI
        reference.doi =
            self.extract_text_between(ref_content, "<pub-id pub-id-type=\"doi\">", "</pub-id>");

        // Extract PMID
        reference.pmid =
            self.extract_text_between(ref_content, "<pub-id pub-id-type=\"pmid\">", "</pub-id>");

        // Extract authors
        reference.authors = self.extract_reference_authors(ref_content);

        // Determine reference type
        if ref_content.contains("<element-citation publication-type") {
            reference.ref_type = self.extract_attribute_value(ref_content, "publication-type");
        }

        reference
    }

    /// Extract authors from reference
    fn extract_reference_authors(&self, ref_content: &str) -> Vec<Author> {
        let mut authors = Vec::new();

        let mut pos = 0;
        while let Some(name_start) = ref_content[pos..].find("<name>") {
            let name_start = pos + name_start;
            if let Some(name_end) = ref_content[name_start..].find("</name>") {
                let name_end = name_start + name_end;
                let name_content = &ref_content[name_start..name_end];

                let surname = self.extract_text_between(name_content, "<surname>", "</surname>");
                let given_names =
                    self.extract_text_between(name_content, "<given-names>", "</given-names>");

                let author = Author::with_names(given_names, surname);
                authors.push(author);

                pos = name_end;
            } else {
                break;
            }
        }

        authors
    }

    /// Extract attribute value from XML tag
    fn extract_attribute_value(&self, content: &str, attribute: &str) -> Option<String> {
        let pattern = format!("{attribute}=\"");
        if let Some(attr_start) = content.find(&pattern) {
            let value_start = attr_start + pattern.len();
            if let Some(value_end) = content[value_start..].find('"') {
                return Some(content[value_start..value_start + value_end].to_string());
            }
        }
        None
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

    /// Extract supplementary materials from XML content
    fn extract_supplementary_materials(&self, content: &str) -> Vec<SupplementaryMaterial> {
        let mut materials = Vec::new();

        let mut pos = 0;
        while let Some(supp_start) = content[pos..].find("<supplementary-material") {
            let supp_start = pos + supp_start;
            if let Some(supp_end) = content[supp_start..].find("</supplementary-material>") {
                let supp_end = supp_start + supp_end;
                let supp_content = &content[supp_start..supp_end];

                if let Some(material) = self.parse_supplementary_material(supp_content) {
                    materials.push(material);
                }

                pos = supp_end;
            } else {
                break;
            }
        }

        materials
    }

    /// Parse a single supplementary material element
    fn parse_supplementary_material(&self, supp_content: &str) -> Option<SupplementaryMaterial> {
        // Extract ID from the opening tag
        let id = self
            .extract_attribute_value(supp_content, "id")
            .unwrap_or_else(|| {
                // Generate a simple ID based on content hash
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                supp_content.hash(&mut hasher);
                format!("supp_{}", hasher.finish())
            });

        let mut material = SupplementaryMaterial::new(id);

        // Extract attributes from the opening tag
        material.content_type = self.extract_attribute_value(supp_content, "content-type");
        material.position = self.extract_attribute_value(supp_content, "position");

        // Extract title from caption
        material.title = self.extract_text_between(supp_content, "<title>", "</title>");

        // Extract description from caption paragraph
        material.description = self
            .extract_text_between(supp_content, "<caption>", "</caption>")
            .map(|caption| self.strip_xml_tags(&caption));

        // Extract file URL from media element
        if let Some(media_start) = supp_content.find("<media") {
            if let Some(media_end) = supp_content[media_start..].find(">") {
                let media_tag = &supp_content[media_start..media_start + media_end + 1];
                material.file_url = self.extract_attribute_value(media_tag, "xlink:href");

                // Infer file type from URL
                if material.file_url.is_some() {
                    material.file_type = material.get_file_extension();
                }
            }
        }

        // Only return materials with valid URLs
        if material.file_url.is_some() {
            Some(material)
        } else {
            None
        }
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

    #[test]
    fn test_extract_supplementary_materials() {
        let parser = PmcXmlParser;
        let content = r#"
            <supplementary-material id="supp1" content-type="local-data" position="float">
                <caption>
                    <title>Supplementary Data</title>
                    <p>Click here for additional data file.</p>
                </caption>
                <media xlink:href="dataset.tar.gz"/>
            </supplementary-material>
            <supplementary-material id="supp2" content-type="local-data">
                <caption>
                    <title>Figures</title>
                </caption>
                <media xlink:href="figures.zip"/>
            </supplementary-material>
        "#;

        let materials = parser.extract_supplementary_materials(content);
        assert_eq!(materials.len(), 2);

        let first_material = &materials[0];
        assert_eq!(first_material.id, "supp1");
        assert_eq!(first_material.content_type, Some("local-data".to_string()));
        assert_eq!(first_material.position, Some("float".to_string()));
        assert_eq!(first_material.title, Some("Supplementary Data".to_string()));
        assert_eq!(first_material.file_url, Some("dataset.tar.gz".to_string()));
        assert!(first_material.is_tar_file());

        let second_material = &materials[1];
        assert_eq!(second_material.id, "supp2");
        assert_eq!(second_material.file_url, Some("figures.zip".to_string()));
        assert!(!second_material.is_tar_file());
        assert!(second_material.is_archive());
    }

    #[test]
    fn test_parse_supplementary_material_single() {
        let parser = PmcXmlParser;
        let content = r#"<supplementary-material id="test-supp" content-type="local-data">
            <caption>
                <title>Test Data</title>
                <p>This is a test archive file.</p>
            </caption>
            <media xlink:href="test-data.tar.gz"/>
        </supplementary-material>"#;

        let material = parser.parse_supplementary_material(content);
        assert!(material.is_some());

        let material = material.unwrap();
        assert_eq!(material.id, "test-supp");
        assert_eq!(material.content_type, Some("local-data".to_string()));
        assert_eq!(material.title, Some("Test Data".to_string()));
        assert_eq!(material.file_url, Some("test-data.tar.gz".to_string()));
        assert!(material.is_tar_file());
        assert_eq!(material.get_file_extension(), Some("gz".to_string()));
    }

    #[test]
    fn test_parse_supplementary_material_no_url() {
        let parser = PmcXmlParser;
        let content = r#"<supplementary-material id="test-supp" content-type="local-data">
            <caption>
                <title>Test Data</title>
            </caption>
        </supplementary-material>"#;

        let material = parser.parse_supplementary_material(content);
        assert!(material.is_none()); // Should return None if no URL
    }
}
