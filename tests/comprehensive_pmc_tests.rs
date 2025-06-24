use rstest::*;
use std::fs;
use std::path::{Path, PathBuf};

use pubmed_client_rs::pmc::PmcXmlParser;

/// テストデータディレクトリから全てのXMLファイルを取得する関数
fn get_xml_files() -> Vec<PathBuf> {
    let xml_dir = Path::new("tests/test_data/pmc_xml");

    if !xml_dir.exists() {
        return Vec::new();
    }

    let mut xml_files = Vec::new();

    if let Ok(entries) = fs::read_dir(xml_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                xml_files.push(path);
            }
        }
    }

    // ファイル名でソート
    xml_files.sort();
    xml_files
}

/// テストケースの構造体
#[derive(Debug)]
struct XmlTestCase {
    file_path: PathBuf,
    pmcid: String,
}

impl XmlTestCase {
    fn new(file_path: PathBuf) -> Self {
        let pmcid = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self { file_path, pmcid }
    }

    fn filename(&self) -> &str {
        self.file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.xml")
    }
}

/// 全XMLファイルを返すフィクスチャ
#[fixture]
fn xml_test_cases() -> Vec<XmlTestCase> {
    get_xml_files().into_iter().map(XmlTestCase::new).collect()
}

#[rstest]
fn test_xml_parsing_basic_validity(#[from(xml_test_cases)] test_cases: Vec<XmlTestCase>) {
    for test_case in &test_cases {
        println!("Testing basic parsing for: {}", test_case.filename());

        let xml_content = fs::read_to_string(&test_case.file_path)
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", test_case.file_path));

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);

        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );

        let article = result.unwrap();

        // 基本的な検証
        assert_eq!(
            article.pmcid,
            test_case.pmcid,
            "PMCID mismatch for {}",
            test_case.filename()
        );
        assert!(
            !article.title.is_empty(),
            "Title should not be empty for {}",
            test_case.filename()
        );
        assert!(
            !article.journal.title.is_empty(),
            "Journal title should not be empty for {}",
            test_case.filename()
        );

        println!("✓ {}: Basic validation passed", test_case.filename());
    }
}

#[rstest]
fn test_xml_parsing_metadata_extraction(#[from(xml_test_cases)] test_cases: Vec<XmlTestCase>) {
    let mut total_tested = 0;
    let mut articles_with_keywords = 0;
    let mut articles_with_funding = 0;
    let mut articles_with_doi = 0;
    let mut articles_with_pmid = 0;

    for test_case in &test_cases {
        println!("Testing metadata extraction for: {}", test_case.filename());

        let xml_content = fs::read_to_string(&test_case.file_path)
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", test_case.file_path));

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );

        let article = result.unwrap();
        total_tested += 1;

        // メタデータ統計
        if !article.keywords.is_empty() {
            articles_with_keywords += 1;
            println!(
                "  Keywords ({}): {:?}",
                article.keywords.len(),
                article.keywords
            );
        }

        if !article.funding.is_empty() {
            articles_with_funding += 1;
            println!("  Funding sources: {}", article.funding.len());
            for funding in &article.funding {
                println!("    - {}", funding.source);
                if let Some(award_id) = &funding.award_id {
                    println!("      Award ID: {}", award_id);
                }
            }
        }

        if let Some(doi) = &article.doi {
            articles_with_doi += 1;
            println!("  DOI: {}", doi);
            assert!(
                doi.contains("10.") || doi.starts_with("doi:"),
                "Invalid DOI format in {}: {}",
                test_case.filename(),
                doi
            );
        }

        if let Some(pmid) = &article.pmid {
            articles_with_pmid += 1;
            println!("  PMID: {}", pmid);
            assert!(
                pmid.parse::<u32>().is_ok(),
                "PMID should be numeric in {}: {}",
                test_case.filename(),
                pmid
            );
        }

        if let Some(article_type) = &article.article_type {
            println!("  Article type: {}", article_type);
        }

        if let Some(coi) = &article.conflict_of_interest {
            println!("  Conflict of interest: {} chars", coi.len());
        }

        if let Some(ack) = &article.acknowledgments {
            println!("  Acknowledgments: {} chars", ack.len());
        }

        println!("✓ {}: Metadata extraction tested", test_case.filename());
    }

    // 統計サマリー
    println!("\n=== Metadata Extraction Summary ===");
    println!("Total files tested: {}", total_tested);
    println!(
        "Articles with keywords: {} ({:.1}%)",
        articles_with_keywords,
        (articles_with_keywords as f64 / total_tested as f64) * 100.0
    );
    println!(
        "Articles with funding: {} ({:.1}%)",
        articles_with_funding,
        (articles_with_funding as f64 / total_tested as f64) * 100.0
    );
    println!(
        "Articles with DOI: {} ({:.1}%)",
        articles_with_doi,
        (articles_with_doi as f64 / total_tested as f64) * 100.0
    );
    println!(
        "Articles with PMID: {} ({:.1}%)",
        articles_with_pmid,
        (articles_with_pmid as f64 / total_tested as f64) * 100.0
    );
}

#[rstest]
fn test_xml_parsing_author_analysis(#[from(xml_test_cases)] test_cases: Vec<XmlTestCase>) {
    let mut total_authors = 0;
    let mut authors_with_orcid = 0;
    let mut authors_with_email = 0;
    let mut authors_with_roles = 0;
    let mut corresponding_authors = 0;

    for test_case in &test_cases {
        println!("Testing author analysis for: {}", test_case.filename());

        let xml_content = fs::read_to_string(&test_case.file_path)
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", test_case.file_path));

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );

        let article = result.unwrap();

        println!("  Authors: {}", article.authors.len());
        total_authors += article.authors.len();

        for (i, author) in article.authors.iter().enumerate() {
            assert!(
                !author.full_name.is_empty(),
                "Author {} full name should not be empty in {}",
                i + 1,
                test_case.filename()
            );

            if author.orcid.is_some() {
                authors_with_orcid += 1;
                println!("    Author {}: ORCID = {:?}", i + 1, author.orcid);
            }

            if author.email.is_some() {
                authors_with_email += 1;
                if let Some(email) = &author.email {
                    assert!(
                        email.contains("@"),
                        "Invalid email format for author {} in {}: {}",
                        i + 1,
                        test_case.filename(),
                        email
                    );
                }
            }

            if !author.roles.is_empty() {
                authors_with_roles += 1;
                println!("    Author {}: Roles = {:?}", i + 1, author.roles);
            }

            if author.is_corresponding {
                corresponding_authors += 1;
                println!("    Author {}: Corresponding author", i + 1);
            }
        }

        println!("✓ {}: Author analysis completed", test_case.filename());
    }

    // 統計サマリー
    println!("\n=== Author Analysis Summary ===");
    println!("Total authors: {}", total_authors);
    if total_authors > 0 {
        println!(
            "Authors with ORCID: {} ({:.1}%)",
            authors_with_orcid,
            (authors_with_orcid as f64 / total_authors as f64) * 100.0
        );
        println!(
            "Authors with email: {} ({:.1}%)",
            authors_with_email,
            (authors_with_email as f64 / total_authors as f64) * 100.0
        );
        println!(
            "Authors with roles: {} ({:.1}%)",
            authors_with_roles,
            (authors_with_roles as f64 / total_authors as f64) * 100.0
        );
        println!(
            "Corresponding authors: {} ({:.1}%)",
            corresponding_authors,
            (corresponding_authors as f64 / total_authors as f64) * 100.0
        );
    }
}

#[rstest]
fn test_xml_parsing_content_structure(#[from(xml_test_cases)] test_cases: Vec<XmlTestCase>) {
    let mut total_sections = 0;
    let mut sections_with_figures = 0;
    let mut sections_with_tables = 0;
    let mut sections_with_subsections = 0;
    let mut total_figures = 0;
    let mut total_tables = 0;

    for test_case in &test_cases {
        println!("Testing content structure for: {}", test_case.filename());

        let xml_content = fs::read_to_string(&test_case.file_path)
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", test_case.file_path));

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );

        let article = result.unwrap();

        println!("  Sections: {}", article.sections.len());
        total_sections += article.sections.len();

        for (i, section) in article.sections.iter().enumerate() {
            println!(
                "    Section {}: {} ({})",
                i + 1,
                section.title.as_ref().unwrap_or(&"No title".to_string()),
                section.section_type
            );

            if !section.figures.is_empty() {
                sections_with_figures += 1;
                total_figures += section.figures.len();
                println!("      Figures: {}", section.figures.len());

                for figure in &section.figures {
                    assert!(
                        !figure.id.is_empty(),
                        "Figure ID should not be empty in section {} of {}",
                        i + 1,
                        test_case.filename()
                    );
                    assert!(
                        !figure.caption.is_empty(),
                        "Figure caption should not be empty in section {} of {}",
                        i + 1,
                        test_case.filename()
                    );
                }
            }

            if !section.tables.is_empty() {
                sections_with_tables += 1;
                total_tables += section.tables.len();
                println!("      Tables: {}", section.tables.len());

                for table in &section.tables {
                    assert!(
                        !table.id.is_empty(),
                        "Table ID should not be empty in section {} of {}",
                        i + 1,
                        test_case.filename()
                    );
                    assert!(
                        !table.caption.is_empty(),
                        "Table caption should not be empty in section {} of {}",
                        i + 1,
                        test_case.filename()
                    );
                }
            }

            if !section.subsections.is_empty() {
                sections_with_subsections += 1;
                println!("      Subsections: {}", section.subsections.len());
            }

            // セクションコンテンツの基本検証
            if !section.content.is_empty() {
                // 最小限の意味のあるコンテンツがあることを確認
                assert!(
                    section.content.len() > 5,
                    "Section content too short in section {} of {}",
                    i + 1,
                    test_case.filename()
                );
            }
        }

        println!("✓ {}: Content structure analyzed", test_case.filename());
    }

    // 統計サマリー
    println!("\n=== Content Structure Summary ===");
    println!("Total sections: {}", total_sections);
    if total_sections > 0 {
        println!(
            "Sections with figures: {} ({:.1}%)",
            sections_with_figures,
            (sections_with_figures as f64 / total_sections as f64) * 100.0
        );
        println!(
            "Sections with tables: {} ({:.1}%)",
            sections_with_tables,
            (sections_with_tables as f64 / total_sections as f64) * 100.0
        );
        println!(
            "Sections with subsections: {} ({:.1}%)",
            sections_with_subsections,
            (sections_with_subsections as f64 / total_sections as f64) * 100.0
        );
    }
    println!("Total figures: {}", total_figures);
    println!("Total tables: {}", total_tables);
}

#[rstest]
fn test_xml_parsing_references_analysis(#[from(xml_test_cases)] test_cases: Vec<XmlTestCase>) {
    let mut total_references = 0;
    let mut refs_with_doi = 0;
    let mut refs_with_pmid = 0;
    let mut refs_with_authors = 0;
    let mut refs_with_title = 0;
    let mut refs_with_journal = 0;

    for test_case in &test_cases {
        println!("Testing references analysis for: {}", test_case.filename());

        let xml_content = fs::read_to_string(&test_case.file_path)
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", test_case.file_path));

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );

        let article = result.unwrap();

        println!("  References: {}", article.references.len());
        total_references += article.references.len();

        for (i, reference) in article.references.iter().take(5).enumerate() {
            assert!(
                !reference.id.is_empty(),
                "Reference {} ID should not be empty in {}",
                i + 1,
                test_case.filename()
            );

            if reference.doi.is_some() {
                refs_with_doi += 1;
                if let Some(doi) = &reference.doi {
                    assert!(
                        doi.contains("10.") || doi.starts_with("doi:"),
                        "Invalid DOI format in reference {} of {}: {}",
                        i + 1,
                        test_case.filename(),
                        doi
                    );
                }
            }

            if reference.pmid.is_some() {
                refs_with_pmid += 1;
                if let Some(pmid) = &reference.pmid {
                    assert!(
                        pmid.parse::<u32>().is_ok(),
                        "PMID should be numeric in reference {} of {}: {}",
                        i + 1,
                        test_case.filename(),
                        pmid
                    );
                }
            }

            if !reference.authors.is_empty() {
                refs_with_authors += 1;
            }

            if reference.title.is_some() {
                refs_with_title += 1;
            }

            if reference.journal.is_some() {
                refs_with_journal += 1;
            }

            // 引用文字列の生成テスト
            let citation = reference.format_citation();
            assert!(
                !citation.is_empty(),
                "Citation should not be empty for reference {} in {}",
                i + 1,
                test_case.filename()
            );
        }

        println!("✓ {}: References analysis completed", test_case.filename());
    }

    // 統計サマリー
    println!("\n=== References Analysis Summary ===");
    println!("Total references: {}", total_references);
    if total_references > 0 {
        println!(
            "References with DOI: {} ({:.1}%)",
            refs_with_doi,
            (refs_with_doi as f64 / total_references as f64) * 100.0
        );
        println!(
            "References with PMID: {} ({:.1}%)",
            refs_with_pmid,
            (refs_with_pmid as f64 / total_references as f64) * 100.0
        );
        println!(
            "References with authors: {} ({:.1}%)",
            refs_with_authors,
            (refs_with_authors as f64 / total_references as f64) * 100.0
        );
        println!(
            "References with title: {} ({:.1}%)",
            refs_with_title,
            (refs_with_title as f64 / total_references as f64) * 100.0
        );
        println!(
            "References with journal: {} ({:.1}%)",
            refs_with_journal,
            (refs_with_journal as f64 / total_references as f64) * 100.0
        );
    }
}

#[rstest]
fn test_xml_parsing_journal_information(#[from(xml_test_cases)] test_cases: Vec<XmlTestCase>) {
    let mut journals_with_issn_electronic = 0;
    let mut journals_with_issn_print = 0;
    let mut journals_with_publisher = 0;
    let mut journals_with_volume = 0;
    let mut journals_with_issue = 0;

    for test_case in &test_cases {
        println!("Testing journal information for: {}", test_case.filename());

        let xml_content = fs::read_to_string(&test_case.file_path)
            .unwrap_or_else(|_| panic!("Failed to read XML file: {:?}", test_case.file_path));

        let result = PmcXmlParser::parse(&xml_content, &test_case.pmcid);
        assert!(
            result.is_ok(),
            "Failed to parse XML file: {}",
            test_case.filename()
        );

        let article = result.unwrap();
        let journal = &article.journal;

        println!("  Journal: {}", journal.title);

        if let Some(abbrev) = &journal.abbreviation {
            println!("    Abbreviation: {}", abbrev);
        }

        if let Some(issn_electronic) = &journal.issn_electronic {
            journals_with_issn_electronic += 1;
            println!("    ISSN Electronic: {}", issn_electronic);
            assert!(
                issn_electronic.len() >= 8,
                "ISSN electronic format check failed for {}: {}",
                test_case.filename(),
                issn_electronic
            );
        }

        if let Some(issn_print) = &journal.issn_print {
            journals_with_issn_print += 1;
            println!("    ISSN Print: {}", issn_print);
            assert!(
                issn_print.len() >= 8,
                "ISSN print format check failed for {}: {}",
                test_case.filename(),
                issn_print
            );
        }

        if let Some(publisher) = &journal.publisher {
            journals_with_publisher += 1;
            println!("    Publisher: {}", publisher);
        }

        if let Some(volume) = &journal.volume {
            journals_with_volume += 1;
            println!("    Volume: {}", volume);
        }

        if let Some(issue) = &journal.issue {
            journals_with_issue += 1;
            println!("    Issue: {}", issue);
        }

        println!("✓ {}: Journal information analyzed", test_case.filename());
    }

    let total_journals = test_cases.len();

    // 統計サマリー
    println!("\n=== Journal Information Summary ===");
    println!("Total journals: {}", total_journals);
    if total_journals > 0 {
        println!(
            "Journals with electronic ISSN: {} ({:.1}%)",
            journals_with_issn_electronic,
            (journals_with_issn_electronic as f64 / total_journals as f64) * 100.0
        );
        println!(
            "Journals with print ISSN: {} ({:.1}%)",
            journals_with_issn_print,
            (journals_with_issn_print as f64 / total_journals as f64) * 100.0
        );
        println!(
            "Journals with publisher: {} ({:.1}%)",
            journals_with_publisher,
            (journals_with_publisher as f64 / total_journals as f64) * 100.0
        );
        println!(
            "Journals with volume: {} ({:.1}%)",
            journals_with_volume,
            (journals_with_volume as f64 / total_journals as f64) * 100.0
        );
        println!(
            "Journals with issue: {} ({:.1}%)",
            journals_with_issue,
            (journals_with_issue as f64 / total_journals as f64) * 100.0
        );
    }
}

#[rstest]
#[case("PMC10618641.xml")]
#[case("PMC10653940.xml")]
fn test_specific_xml_files(#[case] filename: &str) {
    let xml_path = Path::new("tests/test_data/pmc_xml").join(filename);

    if !xml_path.exists() {
        // ファイルが存在しない場合はスキップ
        println!("Skipping test for {}: file not found", filename);
        return;
    }

    let xml_content = fs::read_to_string(&xml_path)
        .unwrap_or_else(|_| panic!("Failed to read XML file: {}", filename));

    let pmcid = filename.replace(".xml", "");
    let result = PmcXmlParser::parse(&xml_content, &pmcid);

    assert!(
        result.is_ok(),
        "Failed to parse specific XML file: {}",
        filename
    );

    let article = result.unwrap();

    println!("=== Detailed Analysis for {} ===", filename);
    println!("Title: {}", article.title);
    println!("Authors: {}", article.authors.len());
    println!("Keywords: {}", article.keywords.len());
    println!("Funding: {}", article.funding.len());
    println!("Sections: {}", article.sections.len());
    println!("References: {}", article.references.len());

    // 追加の詳細検証
    assert!(!article.title.is_empty());
    assert!(!article.authors.is_empty());
    assert!(!article.sections.is_empty());

    if !article.keywords.is_empty() {
        println!("Keywords: {:?}", article.keywords);
    }

    if let Some(doi) = &article.doi {
        println!("DOI: {}", doi);
        assert!(doi.contains("10."));
    }

    if let Some(pmid) = &article.pmid {
        println!("PMID: {}", pmid);
        assert!(pmid.parse::<u32>().is_ok());
    }

    println!("✓ {}: Detailed analysis passed", filename);
}
