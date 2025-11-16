use rstest::*;
use tracing::{debug, info, warn};

use pubmed_client::pmc::{
    parse_pmc_xml, HeadingStyle, MarkdownConfig, PmcMarkdownConverter, ReferenceStyle,
};

mod common;
use common::{pmc_xml_test_cases, PmcXmlTestCase};

/// 全XMLファイルを返すフィクスチャ
#[fixture]
fn markdown_test_cases() -> Vec<PmcXmlTestCase> {
    pmc_xml_test_cases()
}

#[rstest]
fn test_markdown_conversion_basic(#[from(markdown_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    let converter = PmcMarkdownConverter::new();
    // Create regex outside loop for efficiency
    let xml_tag_regex = regex::Regex::new(r"<[^>]*>").unwrap();

    for test_case in &test_cases {
        info!(
            filename = test_case.filename(),
            "Testing markdown conversion"
        );

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_pmc_xml(&xml_content, &test_case.pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );

        let article = result.unwrap();
        let markdown = converter.convert(&article);

        // Basic validation
        assert!(
            !markdown.is_empty(),
            "Markdown should not be empty for {}",
            test_case.filename()
        );

        // Check that title appears in markdown (use cleaned version for comparison)
        // Clean the title using regex to remove XML tags
        let clean_title = xml_tag_regex.replace_all(&article.title, "").to_string();
        let title_words: Vec<&str> = clean_title.split_whitespace().take(4).collect();
        let title_portion = title_words.join(" ");
        assert!(
            markdown
                .to_lowercase()
                .contains(&title_portion.to_lowercase()),
            "Markdown should contain title portion '{}' for {}",
            title_portion,
            test_case.filename()
        );

        // Check that it contains properly formatted markdown
        assert!(
            markdown.contains("# "),
            "Should contain ATX headers for {}",
            test_case.filename()
        );

        // If article has authors, they should be in the markdown
        if !article.authors.is_empty() {
            assert!(
                markdown.contains("**Authors:**"),
                "Should contain authors section for {}",
                test_case.filename()
            );
        }

        // Check journal information
        assert!(
            markdown.contains("**Journal:**"),
            "Should contain journal information for {}",
            test_case.filename()
        );
        assert!(
            markdown.contains(&article.journal.title),
            "Should contain journal title for {}",
            test_case.filename()
        );

        info!(
            filename = test_case.filename(),
            "Basic markdown conversion passed"
        );
    }
}

#[rstest]
fn test_markdown_conversion_with_different_configs(
    #[from(markdown_test_cases)] test_cases: Vec<PmcXmlTestCase>,
) {
    // Test with different configurations
    let configs = vec![
        ("default", PmcMarkdownConverter::new()),
        (
            "no_metadata",
            PmcMarkdownConverter::new().with_include_metadata(false),
        ),
        (
            "setext_headers",
            PmcMarkdownConverter::new().with_heading_style(HeadingStyle::Setext),
        ),
        (
            "author_year_refs",
            PmcMarkdownConverter::new().with_reference_style(ReferenceStyle::AuthorYear),
        ),
        (
            "no_links",
            PmcMarkdownConverter::new()
                .with_include_orcid_links(false)
                .with_include_identifier_links(false),
        ),
        (
            "with_toc",
            PmcMarkdownConverter::new().with_include_toc(true),
        ),
    ];

    // Test with the first few files to avoid excessive output
    for test_case in test_cases.iter().take(3) {
        info!(
            filename = test_case.filename(),
            "Testing different configurations"
        );

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_pmc_xml(&xml_content, &test_case.pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );
        let article = result.unwrap();

        for (config_name, converter) in &configs {
            let markdown = converter.convert(&article);
            if markdown.is_empty() {
                warn!(filename = test_case.filename(), config_name = config_name,
                      section_count = article.sections.len(), title = %article.title,
                      "Empty markdown generated");
            }
            assert!(
                !markdown.is_empty(),
                "Markdown should not be empty for {} with config {}",
                test_case.filename(),
                config_name
            );

            // Test specific config behaviors
            match *config_name {
                "no_metadata" => {
                    assert!(
                        !markdown.contains("**Authors:**"),
                        "Should not contain authors metadata for {}",
                        config_name
                    );
                    assert!(
                        !markdown.contains("**Journal:**") || article.sections.is_empty(),
                        "Should not contain journal metadata or have minimal content for {}",
                        config_name
                    );
                }
                "setext_headers" => {
                    // For level 1 headers, should contain underlines
                    if markdown.contains(&article.title) {
                        let lines: Vec<&str> = markdown.lines().collect();
                        let title_line_idx =
                            lines.iter().position(|&line| line.contains(&article.title));
                        if let Some(idx) = title_line_idx {
                            if idx + 1 < lines.len() {
                                let next_line = lines[idx + 1];
                                if !next_line.is_empty() {
                                    assert!(
                                        next_line.chars().all(|c| c == '=' || c == '-'),
                                        "Setext headers should be underlined for {}",
                                        config_name
                                    );
                                }
                            }
                        }
                    }
                }
                "no_links" => {
                    assert!(
                        !markdown.contains("](http"),
                        "Should not contain links for {}",
                        config_name
                    );
                }
                "with_toc" => {
                    assert!(
                        markdown.contains("Table of Contents"),
                        "Should contain TOC for {}",
                        config_name
                    );
                }
                _ => {}
            }

            debug!(config_name = config_name, "Config test passed");
        }

        info!(filename = test_case.filename(), "All configurations tested");
    }
}

#[rstest]
fn test_markdown_metadata_extraction(#[from(markdown_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    let converter = PmcMarkdownConverter::new();

    let mut total_tested = 0;
    let mut articles_with_doi_links = 0;
    let mut articles_with_pmid_links = 0;
    let mut articles_with_keywords = 0;
    let mut articles_with_funding = 0;

    for test_case in test_cases.iter().take(5) {
        info!(
            filename = test_case.filename(),
            "Testing metadata extraction"
        );

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_pmc_xml(&xml_content, &test_case.pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );

        let article = result.unwrap();
        let markdown = converter.convert(&article);
        total_tested += 1;

        // Check DOI links
        if article.doi.is_some() && markdown.contains("](https://doi.org/") {
            articles_with_doi_links += 1;
            debug!("DOI link found");
        }

        // Check PMID links
        if article.pmid.is_some() && markdown.contains("](https://pubmed.ncbi.nlm.nih.gov/") {
            articles_with_pmid_links += 1;
            debug!("PMID link found");
        }

        // Check keywords
        if !article.keywords.is_empty() {
            articles_with_keywords += 1;
            assert!(
                markdown.contains("**Keywords:**"),
                "Should contain keywords section"
            );
            for keyword in &article.keywords {
                assert!(
                    markdown.contains(keyword),
                    "Should contain keyword: {}",
                    keyword
                );
            }
            debug!(keyword_count = article.keywords.len(), "Keywords found");
        }

        // Check funding
        if !article.funding.is_empty() {
            articles_with_funding += 1;
            assert!(
                markdown.contains("# Funding") || markdown.contains("## Funding"),
                "Should contain funding section"
            );
            debug!(
                funding_count = article.funding.len(),
                "Funding sources found"
            );
        }

        info!(
            filename = test_case.filename(),
            "Metadata extraction tested"
        );
    }

    // 統計サマリー
    info!("Markdown Metadata Extraction Summary");
    info!(total_tested = total_tested, "Files tested");
    if total_tested > 0 {
        info!(
            doi_links = articles_with_doi_links,
            doi_percentage = (articles_with_doi_links as f64 / total_tested as f64) * 100.0,
            "Articles with DOI links"
        );
        info!(
            pmid_links = articles_with_pmid_links,
            pmid_percentage = (articles_with_pmid_links as f64 / total_tested as f64) * 100.0,
            "Articles with PMID links"
        );
        info!(
            keywords = articles_with_keywords,
            keywords_percentage = (articles_with_keywords as f64 / total_tested as f64) * 100.0,
            "Articles with keywords"
        );
        info!(
            funding = articles_with_funding,
            funding_percentage = (articles_with_funding as f64 / total_tested as f64) * 100.0,
            "Articles with funding"
        );
    }
}

#[rstest]
fn test_markdown_content_structure(#[from(markdown_test_cases)] test_cases: Vec<PmcXmlTestCase>) {
    let converter = PmcMarkdownConverter::new();

    for test_case in test_cases.iter().take(3) {
        info!(filename = test_case.filename(), "Testing content structure");

        let xml_content = test_case.read_xml_content_or_panic();

        let result = parse_pmc_xml(&xml_content, &test_case.pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );

        let article = result.unwrap();
        let markdown = converter.convert(&article);

        // Check section structure
        let lines: Vec<&str> = markdown.lines().collect();
        let mut heading_count = 0;
        let mut content_lines = 0;

        for line in &lines {
            if line.starts_with('#') {
                heading_count += 1;
                debug!(heading = %line, "Found heading");
            } else if !line.trim().is_empty() && !line.starts_with("**") {
                content_lines += 1;
            }
        }

        assert!(
            heading_count > 0,
            "Should have at least one heading for {}",
            test_case.filename()
        );
        debug!(
            heading_count = heading_count,
            content_lines = content_lines,
            "Content statistics"
        );

        // Check for figures and tables in markdown if they exist in the original
        let has_figures = article.sections.iter().any(|s| !s.figures.is_empty());
        let has_tables = article.sections.iter().any(|s| !s.tables.is_empty());

        if has_figures {
            if markdown.contains("**Figure") {
                debug!("Contains figure references");
            } else {
                debug!("Article has figures but they may not be properly formatted in markdown");
            }
        }

        if has_tables {
            if markdown.contains("**Table") {
                debug!("Contains table references");
            } else {
                debug!("Article has tables but they may not be properly formatted in markdown");
            }
        }

        // Check references section
        if !article.references.is_empty() {
            assert!(
                markdown.contains("# References") || markdown.contains("## References"),
                "Should contain references section for {}",
                test_case.filename()
            );
            debug!("References section found");
        }

        info!(
            filename = test_case.filename(),
            "Content structure validated"
        );
    }
}

#[rstest]
#[case("PMC10618641.xml")]
#[case("PMC10653940.xml")]
fn test_specific_markdown_conversion(#[case] filename: &str) {
    let test_case = match common::get_pmc_xml_test_case(filename) {
        Some(case) => case,
        None => {
            warn!(filename = filename, "Skipping test: file not found");
            return;
        }
    };

    let xml_content = test_case.read_xml_content_or_panic();
    let result = parse_pmc_xml(&xml_content, &test_case.pmcid);

    assert!(
        result.is_ok(),
        "Failed to parse specific XML file: {}",
        filename
    );

    let article = result.unwrap();

    // Test with different converters
    let converters = vec![
        ("standard", PmcMarkdownConverter::new()),
        (
            "minimal",
            PmcMarkdownConverter::new()
                .with_include_metadata(false)
                .with_include_orcid_links(false)
                .with_include_identifier_links(false),
        ),
        (
            "academic",
            PmcMarkdownConverter::new()
                .with_include_toc(true)
                .with_reference_style(ReferenceStyle::FullCitation)
                .with_heading_style(HeadingStyle::ATX),
        ),
    ];

    for (style_name, converter) in converters {
        let markdown = converter.convert(&article);

        info!(
            style = style_name,
            filename = filename,
            length = markdown.len(),
            line_count = markdown.lines().count(),
            "Generated markdown"
        );

        // Show first few lines as example
        for (i, line) in markdown.lines().take(10).enumerate() {
            debug!(line_number = i + 1, content = %line, "Markdown line");
        }
        debug!("...");

        // Validation
        assert!(!markdown.is_empty(), "Markdown should not be empty");
        assert!(
            markdown.len() > 100,
            "Markdown should have substantial content"
        );

        // Style-specific checks
        match style_name {
            "minimal" => {
                assert!(
                    !markdown.contains("**Authors:**"),
                    "Minimal style should not include metadata"
                );
            }
            "academic" => {
                assert!(
                    markdown.contains("Table of Contents"),
                    "Academic style should include TOC"
                );
            }
            _ => {}
        }

        info!(style = style_name, "Style conversion completed");
    }
}

#[test]
fn test_markdown_config_builder() {
    let config = MarkdownConfig {
        include_metadata: false,
        include_toc: true,
        heading_style: HeadingStyle::Setext,
        reference_style: ReferenceStyle::AuthorYear,
        max_heading_level: 4,
        include_orcid_links: false,
        include_identifier_links: false,
        include_figure_captions: true,
        include_local_figures: false,
    };

    let converter = PmcMarkdownConverter::with_config(config.clone());

    // Test the converter functionality instead of accessing private fields
    // We'll create a simple test article to verify the configuration is working
    use pubmed_client::pmc::models::{JournalInfo, PmcFullText};

    let test_article = PmcFullText {
        pmcid: "PMC123456".to_string(),
        pmid: None,
        title: "Test Article".to_string(),
        authors: vec![],
        journal: JournalInfo::new("Test Journal".to_string()),
        pub_date: "2023".to_string(),
        doi: None,
        sections: vec![],
        references: vec![],
        article_type: None,
        keywords: vec![],
        funding: vec![],
        conflict_of_interest: None,
        acknowledgments: None,
        data_availability: None,
        supplementary_materials: vec![],
    };

    let markdown = converter.convert(&test_article);

    // Test that metadata is not included (since include_metadata is false)
    assert!(
        !markdown.contains("**Authors:**"),
        "Should not include metadata"
    );
    assert!(markdown.contains("Table of Contents"), "Should include TOC");

    // Test Setext headers - title should be underlined
    let lines: Vec<&str> = markdown.lines().collect();
    let title_line_idx = lines.iter().position(|&line| line.contains("Test Article"));
    if let Some(idx) = title_line_idx {
        if idx + 1 < lines.len() {
            let next_line = lines[idx + 1];
            if !next_line.is_empty() {
                assert!(
                    next_line.chars().all(|c| c == '=' || c == '-'),
                    "Setext headers should be underlined"
                );
            }
        }
    }
}

#[test]
fn test_markdown_edge_cases() {
    use pubmed_client::pmc::models::{Author, JournalInfo, PmcFullText};

    // Test with minimal article data
    let minimal_article = PmcFullText {
        pmcid: "PMC000000".to_string(),
        pmid: None,
        title: "Minimal Test Article".to_string(),
        authors: vec![],
        journal: JournalInfo::new("Test Journal".to_string()),
        pub_date: "Unknown Date".to_string(),
        doi: None,
        sections: vec![],
        references: vec![],
        article_type: None,
        keywords: vec![],
        funding: vec![],
        conflict_of_interest: None,
        acknowledgments: None,
        data_availability: None,
        supplementary_materials: vec![],
    };

    let converter = PmcMarkdownConverter::new();
    let markdown = converter.convert(&minimal_article);

    assert!(!markdown.is_empty());
    assert!(markdown.contains("# Minimal Test Article"));
    assert!(markdown.contains("**Journal:** Test Journal"));

    // Test with article containing special characters
    let special_article = PmcFullText {
        pmcid: "PMC111111".to_string(),
        pmid: Some("12345".to_string()),
        title: "Article with <em>HTML</em> & Special Characters".to_string(),
        authors: vec![Author::from_full_name(
            "Dr. John O'Reilly & Associates".to_string(),
        )],
        journal: JournalInfo::new("Special Characters Journal".to_string()),
        pub_date: "2023".to_string(),
        doi: Some("10.1000/test<>special".to_string()),
        sections: vec![],
        references: vec![],
        article_type: Some("research-article".to_string()),
        keywords: vec![
            "test & validation".to_string(),
            "<script>alert('xss')</script>".to_string(),
        ],
        funding: vec![],
        conflict_of_interest: None,
        acknowledgments: None,
        data_availability: None,
        supplementary_materials: vec![],
    };

    let markdown = converter.convert(&special_article);
    debug!(markdown = %markdown, "Generated markdown for special characters test");
    assert!(
        markdown.contains("Article with"),
        "Should contain cleaned title text"
    );
    assert!(
        markdown.contains("Special Characters"),
        "Should contain cleaned title text"
    );
    assert!(
        markdown.contains("Dr. John"),
        "Should contain cleaned author text"
    );
    assert!(
        markdown.contains("Associates"),
        "Should contain cleaned author text"
    );
    assert!(
        !markdown.contains("<em>") && !markdown.contains("</em>"),
        "Should not contain HTML tags"
    );
    assert!(
        !markdown.contains("<script>"),
        "Should not contain script tags"
    );
}
