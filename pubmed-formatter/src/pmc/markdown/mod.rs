//! Markdown conversion functionality for PMC articles
//!
//! This module provides functionality to convert parsed PMC articles into
//! well-formatted Markdown documents with configurable styling options.

mod config;
mod entities;
mod frontmatter;
mod heading;
mod metadata;
mod references;
mod sections;
mod toc;

use std::collections::HashMap;

use pubmed_parser::pmc::PmcArticle;

pub use config::{FigureOptions, HeadingStyle, MarkdownConfig, MetadataOptions, ReferenceStyle};
use entities::clean_content;
use heading::format_heading;

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
        self.config.metadata.include_metadata = include;
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
        self.config.metadata.include_orcid_links = include;
        self
    }

    /// Set whether to include identifier links
    pub fn with_include_identifier_links(mut self, include: bool) -> Self {
        self.config.metadata.include_identifier_links = include;
        self
    }

    /// Set whether to include figure captions
    pub fn with_include_figure_captions(mut self, include: bool) -> Self {
        self.config.figures.include_figure_captions = include;
        self
    }

    /// Set whether to use YAML frontmatter for metadata
    pub fn with_yaml_frontmatter(mut self, use_yaml: bool) -> Self {
        self.config.metadata.use_yaml_frontmatter = use_yaml;
        self
    }

    /// Convert a PMC article to Markdown with optional figure paths
    pub fn convert_with_figures(
        &self,
        article: &PmcArticle,
        figure_paths: Option<&HashMap<String, String>>,
    ) -> String {
        let mut markdown = String::new();

        if self.config.metadata.include_metadata {
            markdown.push_str(&metadata::convert_metadata(&self.config, article));
            markdown.push_str("\n\n");
        } else {
            let title = article.title().unwrap_or("Untitled");
            markdown.push_str(&format_heading(&self.config, &clean_content(title), 1));
            markdown.push_str("\n\n");
        }

        if self.config.include_toc {
            markdown.push_str(&toc::convert_toc(&self.config, article));
            markdown.push_str("\n\n");
        }

        markdown.push_str(&sections::convert_sections(
            &self.config,
            article.sections(),
            1,
            figure_paths,
        ));

        if !article.references().is_empty() {
            markdown.push_str(&references::convert_references(
                &self.config,
                article.references(),
            ));
        }

        markdown.push_str(&sections::convert_additional_sections(
            &self.config,
            article,
        ));

        markdown.trim().to_string()
    }

    /// Convert a PMC article to Markdown
    pub fn convert(&self, article: &PmcArticle) -> String {
        self.convert_with_figures(article, None)
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
    use pubmed_parser::common::{Author, PmcId, PubMedId, PublicationDate};
    use pubmed_parser::pmc::{ArticleMeta, Front, JournalMeta, TitleGroup};

    fn test_article(title: &str, pmcid: &str) -> PmcArticle {
        PmcArticle {
            article_type: None,
            front: Front {
                journal_meta: JournalMeta {
                    title: Some("Test Journal".to_string()),
                    abbreviation: None,
                    issn_print: None,
                    issn_electronic: None,
                    publisher: None,
                },
                article_meta: ArticleMeta {
                    pmcid: PmcId::parse(pmcid).unwrap(),
                    pmid: None,
                    doi: None,
                    categories: vec![],
                    title_group: TitleGroup {
                        article_title: Some(title.to_string()),
                        subtitle: None,
                    },
                    authors: vec![],
                    pub_dates: vec![],
                    volume: None,
                    issue: None,
                    fpage: None,
                    lpage: None,
                    elocation_id: None,
                    history: vec![],
                    permissions: None,
                    abstracts: vec![],
                    keywords: vec![],
                    funding: vec![],
                },
            },
            body: None,
            back: None,
            supplementary_materials: vec![],
            data_availability: None,
        }
    }

    #[test]
    fn test_markdown_converter_creation() {
        let converter = PmcMarkdownConverter::new();
        assert!(converter.config.metadata.include_metadata);
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

        assert!(!converter.config.metadata.include_metadata);
        assert_eq!(converter.config.heading_style, HeadingStyle::Setext);
        assert_eq!(converter.config.reference_style, ReferenceStyle::AuthorYear);
        assert_eq!(converter.config.max_heading_level, 4);
    }

    #[test]
    fn test_heading_formatting() {
        let config = MarkdownConfig::default();

        assert_eq!(format_heading(&config, "Title", 1), "# Title");
        assert_eq!(format_heading(&config, "Subtitle", 2), "## Subtitle");

        let setext_config = MarkdownConfig {
            heading_style: HeadingStyle::Setext,
            ..Default::default()
        };
        assert_eq!(format_heading(&setext_config, "Title", 1), "Title\n=====");
        assert_eq!(
            format_heading(&setext_config, "Subtitle", 2),
            "Subtitle\n--------"
        );
        assert_eq!(format_heading(&setext_config, "Section", 3), "### Section");
    }

    #[test]
    fn test_clean_content() {
        let dirty = "<p>This is <em>emphasis</em> and &amp; entities</p>";
        let clean = clean_content(dirty);
        assert_eq!(clean, "This is emphasis and & entities");
    }

    #[test]
    fn test_anchor_creation() {
        use heading::heading_anchor;

        assert_eq!(heading_anchor("Introduction"), "introduction");
        assert_eq!(heading_anchor("Methods & Results"), "methods-results");
        assert_eq!(heading_anchor("Discussion (2023)"), "discussion-2023");
    }

    #[test]
    fn test_basic_conversion() {
        let converter = PmcMarkdownConverter::new();

        let mut article = test_article("Test Article", "PMC1234567");
        article.front.article_meta.pmid = Some(PubMedId::parse("12345").unwrap());
        article.front.article_meta.authors = vec![Author::from_full_name("John Doe".to_string())];
        article.front.article_meta.pub_dates = vec![PublicationDate {
            pub_type: None,
            year: Some(2023),
            month: None,
            day: None,
        }];
        article.front.article_meta.doi = Some("10.1000/test".to_string());
        article.article_type = Some("research-article".to_string());
        article.front.article_meta.keywords = vec!["test".to_string(), "example".to_string()];

        let markdown = converter.convert(&article);
        assert!(markdown.contains("# Test Article"));
        assert!(markdown.contains("**Authors:** John Doe"));
        assert!(markdown.contains("**Journal:** Test Journal"));
        assert!(markdown.contains("DOI: 10.1000/test"));
        assert!(markdown.contains("**Keywords:** test, example"));
    }

    #[test]
    fn test_yaml_frontmatter_basic() {
        let converter = PmcMarkdownConverter::new().with_yaml_frontmatter(true);

        let mut article = test_article("Test Article", "PMC1234567");
        article.front.article_meta.pmid = Some(PubMedId::parse("12345").unwrap());
        article.front.article_meta.authors = vec![
            Author::from_full_name("John Doe".to_string()),
            Author::from_full_name("Jane Smith".to_string()),
        ];
        article.front.article_meta.pub_dates = vec![PublicationDate {
            pub_type: None,
            year: Some(2023),
            month: Some(5),
            day: Some(15),
        }];
        article.front.article_meta.doi = Some("10.1000/test".to_string());
        article.article_type = Some("research-article".to_string());
        article.front.article_meta.keywords = vec!["test".to_string(), "example".to_string()];

        let markdown = converter.convert(&article);

        assert!(markdown.starts_with("---\n"));
        let delimiter_count = markdown.matches("---").count();
        assert_eq!(
            delimiter_count, 2,
            "Should have opening and closing YAML frontmatter delimiters"
        );

        assert!(markdown.contains("title: Test Article"));
        assert!(markdown.contains("authors:"));
        assert!(markdown.contains("- John Doe"));
        assert!(markdown.contains("- Jane Smith"));
        assert!(markdown.contains("journal: Test Journal"));
        assert!(
            markdown.contains("pub_date: '2023-05-15'")
                || markdown.contains("pub_date: 2023-05-15")
        );
        assert!(markdown.contains("pmcid: PMC1234567"));
        assert!(markdown.contains("pmid: '12345'"));
        assert!(markdown.contains("doi: 10.1000/test"));
        assert!(markdown.contains("article_type: research-article"));
        assert!(markdown.contains("keywords:"));
        assert!(markdown.contains("- test"));
        assert!(markdown.contains("- example"));
    }

    #[test]
    fn test_yaml_frontmatter_with_special_characters() {
        let converter = PmcMarkdownConverter::new().with_yaml_frontmatter(true);

        let mut article = test_article("COVID-19: A Comprehensive Study", "PMC7890123");
        article.front.journal_meta.title = Some("Nature: Medicine & Science".to_string());
        article.front.article_meta.authors =
            vec![Author::from_full_name("O'Brien, Michael".to_string())];
        article.front.article_meta.pub_dates = vec![PublicationDate {
            pub_type: None,
            year: Some(2023),
            month: None,
            day: None,
        }];
        article.front.article_meta.doi = Some("10.1038/s41591-023-01234-5".to_string());
        article.article_type = Some("research-article".to_string());
        article.front.article_meta.keywords = vec![
            "#COVID-19".to_string(),
            "SARS-CoV-2".to_string(),
            "vaccine".to_string(),
        ];

        let markdown = converter.convert(&article);

        assert!(
            markdown.contains("title: 'COVID-19: A Comprehensive Study'")
                || markdown.contains("title: \"COVID-19: A Comprehensive Study\"")
        );
        assert!(
            markdown.contains("journal: 'Nature: Medicine & Science'")
                || markdown.contains("journal: \"Nature: Medicine & Science\"")
        );
        assert!(markdown.contains("'#COVID-19'") || markdown.contains("\"#COVID-19\""));
        assert!(markdown.contains("SARS-CoV-2"));
    }

    #[test]
    fn test_yaml_frontmatter_backward_compatibility() {
        let converter = PmcMarkdownConverter::new();
        assert!(!converter.config.metadata.use_yaml_frontmatter);

        let article = test_article("Test Article", "PMC1234567");

        let markdown = converter.convert(&article);

        assert!(markdown.contains("# Test Article"));
        assert!(markdown.contains("**Journal:** Test Journal"));
        assert!(!markdown.starts_with("---\n"));
    }

    #[test]
    fn test_builder_pattern_with_yaml_frontmatter() {
        let converter = PmcMarkdownConverter::new()
            .with_yaml_frontmatter(true)
            .with_include_metadata(true)
            .with_heading_style(HeadingStyle::ATX);

        assert!(converter.config.metadata.use_yaml_frontmatter);
        assert!(converter.config.metadata.include_metadata);
        assert_eq!(converter.config.heading_style, HeadingStyle::ATX);
    }
}
