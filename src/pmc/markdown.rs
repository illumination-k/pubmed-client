//! Markdown conversion functionality for PMC articles
//!
//! This module provides functionality to convert parsed PMC articles into
//! well-formatted Markdown documents with configurable styling options.

use crate::pmc::models::{ArticleSection, Author, FundingInfo, PmcFullText, Reference};

/// Configuration options for Markdown conversion
#[derive(Debug, Clone)]
pub struct MarkdownConfig {
    /// Include metadata section at the top
    pub include_metadata: bool,
    /// Include table of contents
    pub include_toc: bool,
    /// Heading style preference
    pub heading_style: HeadingStyle,
    /// Reference formatting style
    pub reference_style: ReferenceStyle,
    /// Maximum heading level (1-6)
    pub max_heading_level: u8,
    /// Include author ORCID links
    pub include_orcid_links: bool,
    /// Include DOI and PMID links
    pub include_identifier_links: bool,
    /// Include figure and table captions
    pub include_figure_captions: bool,
}

/// Heading style options
#[derive(Debug, Clone, PartialEq)]
pub enum HeadingStyle {
    /// ATX style headers (# ## ###)
    ATX,
    /// Setext style headers (underlined)
    Setext,
}

/// Reference formatting style
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceStyle {
    /// Numbered references \[1\], \[2\], etc.
    Numbered,
    /// Author-year style (Smith, 2023)
    AuthorYear,
    /// Full citation format
    FullCitation,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            include_metadata: true,
            include_toc: false,
            heading_style: HeadingStyle::ATX,
            reference_style: ReferenceStyle::Numbered,
            max_heading_level: 6,
            include_orcid_links: true,
            include_identifier_links: true,
            include_figure_captions: true,
        }
    }
}

/// PMC to Markdown converter
pub struct PmcMarkdownConverter {
    config: MarkdownConfig,
}

impl PmcMarkdownConverter {
    /// Create a new converter with default configuration
    pub fn new() -> Self {
        Self {
            config: MarkdownConfig::default(),
        }
    }

    /// Create a converter with custom configuration
    pub fn with_config(config: MarkdownConfig) -> Self {
        Self { config }
    }

    /// Set whether to include metadata
    pub fn with_include_metadata(mut self, include: bool) -> Self {
        self.config.include_metadata = include;
        self
    }

    /// Set whether to include table of contents
    pub fn with_include_toc(mut self, include: bool) -> Self {
        self.config.include_toc = include;
        self
    }

    /// Set heading style
    pub fn with_heading_style(mut self, style: HeadingStyle) -> Self {
        self.config.heading_style = style;
        self
    }

    /// Set reference style
    pub fn with_reference_style(mut self, style: ReferenceStyle) -> Self {
        self.config.reference_style = style;
        self
    }

    /// Set maximum heading level
    pub fn with_max_heading_level(mut self, level: u8) -> Self {
        self.config.max_heading_level = level.clamp(1, 6);
        self
    }

    /// Set whether to include ORCID links
    pub fn with_include_orcid_links(mut self, include: bool) -> Self {
        self.config.include_orcid_links = include;
        self
    }

    /// Set whether to include identifier links
    pub fn with_include_identifier_links(mut self, include: bool) -> Self {
        self.config.include_identifier_links = include;
        self
    }

    /// Convert a PMC article to Markdown
    pub fn convert(&self, article: &PmcFullText) -> String {
        let mut markdown = String::new();

        // Add metadata section
        if self.config.include_metadata {
            markdown.push_str(&self.convert_metadata(article));
            markdown.push_str("\n\n");
        } else {
            // Always include at least the title even when metadata is disabled
            markdown.push_str(&self.format_heading(&self.clean_content(&article.title), 1));
            markdown.push_str("\n\n");
        }

        // Add table of contents if requested
        if self.config.include_toc {
            markdown.push_str(&self.convert_toc(article));
            markdown.push_str("\n\n");
        }

        // Add main content sections
        markdown.push_str(&self.convert_sections(&article.sections, 1));

        // Add references section
        if !article.references.is_empty() {
            markdown.push_str(&self.convert_references(&article.references));
        }

        // Add additional sections
        markdown.push_str(&self.convert_additional_sections(article));

        markdown.trim().to_string()
    }

    /// Convert metadata section
    fn convert_metadata(&self, article: &PmcFullText) -> String {
        let mut metadata = String::new();

        // Title
        metadata.push_str(&self.format_heading(&self.clean_content(&article.title), 1));
        metadata.push('\n');

        // Authors
        if !article.authors.is_empty() {
            metadata.push_str("\n**Authors:** ");
            metadata.push_str(&self.format_authors(&article.authors));
            metadata.push('\n');
        }

        // Journal information
        let journal_title = &article.journal.title;
        metadata.push_str(&format!("\n**Journal:** {journal_title}"));
        if let Some(abbrev) = &article.journal.abbreviation {
            metadata.push_str(&format!(" ({abbrev})"));
        }
        metadata.push('\n');

        // Publication date
        if !article.pub_date.is_empty() && article.pub_date != "Unknown Date" {
            let pub_date = &article.pub_date;
            metadata.push_str(&format!("**Published:** {pub_date}\n"));
        }

        // Identifiers
        let mut identifiers = Vec::new();
        if let Some(doi) = &article.doi {
            if self.config.include_identifier_links {
                identifiers.push(format!("[DOI: {doi}](https://doi.org/{doi})"));
            } else {
                identifiers.push(format!("DOI: {doi}"));
            }
        }
        if let Some(pmid) = &article.pmid {
            if self.config.include_identifier_links {
                identifiers.push(format!(
                    "[PMID: {pmid}](https://pubmed.ncbi.nlm.nih.gov/{pmid})"
                ));
            } else {
                identifiers.push(format!("PMID: {pmid}"));
            }
        }
        let pmcid = &article.pmcid;
        identifiers.push(format!("PMC: {pmcid}"));

        if !identifiers.is_empty() {
            let identifiers_str = identifiers.join(" | ");
            metadata.push_str(&format!("**Identifiers:** {identifiers_str}\n"));
        }

        // Article type
        if let Some(article_type) = &article.article_type {
            metadata.push_str(&format!("**Article Type:** {article_type}\n"));
        }

        // Keywords
        if !article.keywords.is_empty() {
            let clean_keywords: Vec<String> = article
                .keywords
                .iter()
                .map(|k| self.clean_content(k))
                .collect();
            let keywords_str = clean_keywords.join(", ");
            metadata.push_str(&format!("**Keywords:** {keywords_str}\n"));
        }

        // Journal details
        let mut journal_details = Vec::new();
        if let Some(volume) = &article.journal.volume {
            journal_details.push(format!("Volume {volume}"));
        }
        if let Some(issue) = &article.journal.issue {
            journal_details.push(format!("Issue {issue}"));
        }
        if let Some(publisher) = &article.journal.publisher {
            journal_details.push(format!("Publisher: {publisher}"));
        }
        if !journal_details.is_empty() {
            metadata.push_str(&format!(
                "**Journal Details:** {}\n",
                journal_details.join(" | ")
            ));
        }

        metadata
    }

    /// Convert table of contents
    fn convert_toc(&self, article: &PmcFullText) -> String {
        let mut toc = String::new();
        toc.push_str(&self.format_heading("Table of Contents", 2));
        toc.push('\n');

        for (i, section) in article.sections.iter().enumerate() {
            let default_title = "Untitled".to_string();
            let title = section.title.as_ref().unwrap_or(&default_title);
            let anchor = self.create_anchor(title);
            let index = i + 1;
            toc.push_str(&format!("{index}. [{title}](#{anchor})\n"));

            // Add subsections
            for (j, subsection) in section.subsections.iter().enumerate() {
                let default_sub_title = "Untitled".to_string();
                let sub_title = subsection.title.as_ref().unwrap_or(&default_sub_title);
                let sub_anchor = self.create_anchor(sub_title);
                let main_index = i + 1;
                let sub_index = j + 1;
                toc.push_str(&format!(
                    "   {main_index}.{sub_index}. [{sub_title}](#{sub_anchor})\n"
                ));
            }
        }

        toc
    }

    /// Convert article sections
    fn convert_sections(&self, sections: &[ArticleSection], level: u8) -> String {
        let mut content = String::new();

        for section in sections {
            // Section heading
            if let Some(title) = &section.title {
                content.push_str(&self.format_heading(title, level));
                content.push_str("\n\n");
            }

            // Section content
            if !section.content.is_empty() {
                content.push_str(&self.clean_content(&section.content));
                content.push_str("\n\n");
            }

            // Figures
            if self.config.include_figure_captions {
                for figure in &section.figures {
                    content.push_str(&self.convert_figure(figure));
                    content.push_str("\n\n");
                }
            }

            // Tables
            if self.config.include_figure_captions {
                for table in &section.tables {
                    content.push_str(&self.convert_table(table));
                    content.push_str("\n\n");
                }
            }

            // Subsections
            if !section.subsections.is_empty() {
                let next_level = (level + 1).min(self.config.max_heading_level);
                content.push_str(&self.convert_sections(&section.subsections, next_level));
            }
        }

        content
    }

    /// Convert references section
    fn convert_references(&self, references: &[Reference]) -> String {
        let mut content = String::new();
        content.push_str(&self.format_heading("References", 2));
        content.push_str("\n\n");

        match self.config.reference_style {
            ReferenceStyle::Numbered => {
                for (i, reference) in references.iter().enumerate() {
                    content.push_str(&format!(
                        "{}. {}\n",
                        i + 1,
                        self.format_reference(reference)
                    ));
                }
            }
            ReferenceStyle::AuthorYear | ReferenceStyle::FullCitation => {
                for reference in references {
                    let formatted_ref = self.format_reference(reference);
                    content.push_str(&format!("- {formatted_ref}\n"));
                }
            }
        }

        content.push('\n');
        content
    }

    /// Convert additional sections (funding, conflicts, acknowledgments)
    fn convert_additional_sections(&self, article: &PmcFullText) -> String {
        let mut content = String::new();

        // Funding
        if !article.funding.is_empty() {
            content.push_str(&self.format_heading("Funding", 2));
            content.push_str("\n\n");
            for funding in &article.funding {
                content.push_str(&self.format_funding(funding));
                content.push('\n');
            }
            content.push('\n');
        }

        // Conflict of interest
        if let Some(coi) = &article.conflict_of_interest {
            content.push_str(&self.format_heading("Conflict of Interest", 2));
            content.push_str("\n\n");
            content.push_str(&self.clean_content(coi));
            content.push_str("\n\n");
        }

        // Acknowledgments
        if let Some(ack) = &article.acknowledgments {
            content.push_str(&self.format_heading("Acknowledgments", 2));
            content.push_str("\n\n");
            content.push_str(&self.clean_content(ack));
            content.push_str("\n\n");
        }

        // Data availability
        if let Some(data_avail) = &article.data_availability {
            content.push_str(&self.format_heading("Data Availability", 2));
            content.push_str("\n\n");
            content.push_str(&self.clean_content(data_avail));
            content.push_str("\n\n");
        }

        content
    }

    /// Format a heading based on the configured style
    fn format_heading(&self, text: &str, level: u8) -> String {
        let level = level.min(self.config.max_heading_level);

        match self.config.heading_style {
            HeadingStyle::ATX => {
                let hashes = "#".repeat(level as usize);
                format!("{hashes} {text}")
            }
            HeadingStyle::Setext => {
                if level == 1 {
                    let underline = "=".repeat(text.len());
                    format!("{text}\n{underline}")
                } else if level == 2 {
                    let underline = "-".repeat(text.len());
                    format!("{text}\n{underline}")
                } else {
                    // Fall back to ATX for levels 3+
                    let hashes = "#".repeat(level as usize);
                    format!("{hashes} {text}")
                }
            }
        }
    }

    /// Format authors list
    fn format_authors(&self, authors: &[Author]) -> String {
        let formatted_authors: Vec<String> = authors
            .iter()
            .map(|author| {
                let mut name = self.clean_content(&author.full_name);

                // Add ORCID link if available and enabled
                if self.config.include_orcid_links {
                    if let Some(orcid) = &author.orcid {
                        let clean_orcid = orcid.trim_start_matches("https://orcid.org/");
                        if clean_orcid.len() > 10 {
                            // Basic ORCID format check
                            name.push_str(&format!(" ([ORCID](https://orcid.org/{clean_orcid}))"));
                        }
                    }
                }

                // Add corresponding author indicator
                if author.is_corresponding {
                    name.push_str(" **(corresponding)**");
                }

                name
            })
            .collect();

        formatted_authors.join(", ")
    }

    /// Format a single reference
    fn format_reference(&self, reference: &Reference) -> String {
        match self.config.reference_style {
            ReferenceStyle::Numbered | ReferenceStyle::FullCitation => {
                let citation = reference.format_citation();

                if self.config.include_identifier_links {
                    let mut formatted = citation;

                    // Add DOI link
                    if let Some(doi) = &reference.doi {
                        formatted.push_str(&format!(" [DOI](https://doi.org/{doi})"));
                    }

                    // Add PMID link
                    if let Some(pmid) = &reference.pmid {
                        formatted
                            .push_str(&format!(" [PMID](https://pubmed.ncbi.nlm.nih.gov/{pmid})"));
                    }

                    formatted
                } else {
                    citation
                }
            }
            ReferenceStyle::AuthorYear => {
                if !reference.authors.is_empty() && reference.year.is_some() {
                    format!(
                        "{} ({})",
                        reference.authors.first().unwrap().full_name,
                        reference.year.as_ref().unwrap()
                    )
                } else {
                    reference.format_citation()
                }
            }
        }
    }

    /// Format funding information
    fn format_funding(&self, funding: &FundingInfo) -> String {
        let source = &funding.source;
        let mut text = format!("- **{source}**");

        if let Some(award_id) = &funding.award_id {
            text.push_str(&format!(" (Award ID: {award_id})"));
        }

        if let Some(statement) = &funding.statement {
            let content = self.clean_content(statement);
            text.push_str(&format!(": {content}"));
        }

        text
    }

    /// Convert figure to markdown
    fn convert_figure(&self, figure: &crate::pmc::models::Figure) -> String {
        let mut content = String::new();

        if let Some(label) = &figure.label {
            content.push_str(&format!("**{label}**"));
        } else {
            let figure_id = &figure.id;
            content.push_str(&format!("**Figure {figure_id}**"));
        }

        let caption = self.clean_content(&figure.caption);
        content.push_str(&format!(": {caption}"));

        if let Some(alt_text) = &figure.alt_text {
            let alt_content = self.clean_content(alt_text);
            content.push_str(&format!("\n\n*Alt text: {alt_content}*"));
        }

        content
    }

    /// Convert table to markdown
    fn convert_table(&self, table: &crate::pmc::models::Table) -> String {
        let mut content = String::new();

        if let Some(label) = &table.label {
            content.push_str(&format!("**{label}**"));
        } else {
            let table_id = &table.id;
            content.push_str(&format!("**Table {table_id}**"));
        }

        let caption = self.clean_content(&table.caption);
        content.push_str(&format!(": {caption}"));

        if !table.footnotes.is_empty() {
            content.push_str("\n\n*Footnotes:*\n");
            for (i, footnote) in table.footnotes.iter().enumerate() {
                let index = i + 1;
                let footnote_content = self.clean_content(footnote);
                content.push_str(&format!("{index}. {footnote_content}\n"));
            }
        }

        content
    }

    /// Clean content by removing XML tags and fixing formatting
    fn clean_content(&self, content: &str) -> String {
        // Remove XML tags but preserve content
        let mut cleaned = content.to_string();

        // Remove common XML tags while preserving content
        cleaned = regex::Regex::new(r"<[^>]*>")
            .unwrap()
            .replace_all(&cleaned, "")
            .to_string();

        // Fix common HTML entities
        cleaned = cleaned
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#x27;", "'");

        // Normalize whitespace
        cleaned = regex::Regex::new(r"\s+")
            .unwrap()
            .replace_all(&cleaned, " ")
            .trim()
            .to_string();

        cleaned
    }

    /// Create URL-safe anchor from title
    fn create_anchor(&self, title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

impl Default for PmcMarkdownConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pmc::models::{Author, JournalInfo, PmcFullText};

    #[test]
    fn test_markdown_converter_creation() {
        let converter = PmcMarkdownConverter::new();
        assert!(converter.config.include_metadata);
        assert_eq!(converter.config.heading_style, HeadingStyle::ATX);
        assert_eq!(converter.config.reference_style, ReferenceStyle::Numbered);
    }

    #[test]
    fn test_configuration_builder() {
        let converter = PmcMarkdownConverter::new()
            .with_include_metadata(false)
            .with_heading_style(HeadingStyle::Setext)
            .with_reference_style(ReferenceStyle::AuthorYear)
            .with_max_heading_level(4);

        assert!(!converter.config.include_metadata);
        assert_eq!(converter.config.heading_style, HeadingStyle::Setext);
        assert_eq!(converter.config.reference_style, ReferenceStyle::AuthorYear);
        assert_eq!(converter.config.max_heading_level, 4);
    }

    #[test]
    fn test_heading_formatting() {
        let converter = PmcMarkdownConverter::new();

        // ATX style
        assert_eq!(converter.format_heading("Title", 1), "# Title");
        assert_eq!(converter.format_heading("Subtitle", 2), "## Subtitle");

        // Setext style
        let converter = converter.with_heading_style(HeadingStyle::Setext);
        assert_eq!(converter.format_heading("Title", 1), "Title\n=====");
        assert_eq!(
            converter.format_heading("Subtitle", 2),
            "Subtitle\n--------"
        );
        assert_eq!(converter.format_heading("Section", 3), "### Section");
    }

    #[test]
    fn test_clean_content() {
        let converter = PmcMarkdownConverter::new();

        let dirty = "<p>This is <em>emphasis</em> and &amp; entities</p>";
        let clean = converter.clean_content(dirty);
        assert_eq!(clean, "This is emphasis and & entities");
    }

    #[test]
    fn test_anchor_creation() {
        let converter = PmcMarkdownConverter::new();

        assert_eq!(converter.create_anchor("Introduction"), "introduction");
        assert_eq!(
            converter.create_anchor("Methods & Results"),
            "methods-results"
        );
        assert_eq!(
            converter.create_anchor("Discussion (2023)"),
            "discussion-2023"
        );
    }

    #[test]
    fn test_basic_conversion() {
        let converter = PmcMarkdownConverter::new();

        let article = PmcFullText {
            pmcid: "PMC1234567".to_string(),
            pmid: Some("12345".to_string()),
            title: "Test Article".to_string(),
            authors: vec![Author::new("John Doe".to_string())],
            journal: JournalInfo::new("Test Journal".to_string()),
            pub_date: "2023".to_string(),
            doi: Some("10.1000/test".to_string()),
            sections: vec![],
            references: vec![],
            article_type: Some("research-article".to_string()),
            keywords: vec!["test".to_string(), "example".to_string()],
            funding: vec![],
            conflict_of_interest: None,
            acknowledgments: None,
            data_availability: None,
        };

        let markdown = converter.convert(&article);
        assert!(markdown.contains("# Test Article"));
        assert!(markdown.contains("**Authors:** John Doe"));
        assert!(markdown.contains("**Journal:** Test Journal"));
        assert!(markdown.contains("DOI: 10.1000/test"));
        assert!(markdown.contains("**Keywords:** test, example"));
    }
}
