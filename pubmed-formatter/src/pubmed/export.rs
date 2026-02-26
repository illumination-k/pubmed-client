//! Citation export formats for PubMed articles
//!
//! This module provides functionality to export PubMed article metadata to various
//! standard citation formats commonly used in academic research:
//!
//! - **BibTeX** - Used by LaTeX and many reference managers
//! - **RIS** - Used by Zotero, Mendeley, EndNote, and many others
//! - **CSL-JSON** - Citation Style Language JSON format
//! - **NBIB** - MEDLINE/PubMed native format

use pubmed_parser::pubmed::models::PubMedArticle;
use serde_json::{json, Value};

/// Generate a BibTeX citation key from article metadata
fn generate_bibtex_key(article: &PubMedArticle) -> String {
    let first_author = article
        .authors
        .first()
        .map(|a| {
            a.full_name
                .split_whitespace()
                .next()
                .unwrap_or("Unknown")
                .to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let year = article
        .pub_date
        .split_whitespace()
        .find(|s| s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or("0000");

    format!("{}{}_pmid{}", first_author, year, article.pmid)
}

/// Escape special BibTeX characters
fn escape_bibtex(s: &str) -> String {
    s.replace('&', r"\&")
        .replace('%', r"\%")
        .replace('_', r"\_")
        .replace('#', r"\#")
        .replace('{', r"\{")
        .replace('}', r"\}")
}

/// Trait for exporting PubMed articles to various citation formats
pub trait ExportFormat {
    /// Export the article metadata as a BibTeX entry
    fn to_bibtex(&self) -> String;

    /// Export the article metadata in RIS format
    fn to_ris(&self) -> String;

    /// Export the article metadata as CSL-JSON
    fn to_csl_json(&self) -> Value;

    /// Export the article metadata in MEDLINE/NBIB format
    fn to_nbib(&self) -> String;
}

impl ExportFormat for PubMedArticle {
    fn to_bibtex(&self) -> String {
        let key = generate_bibtex_key(self);
        let mut lines = Vec::new();

        lines.push(format!("@article{{{key},"));

        lines.push(format!("  title = {{{}}},", escape_bibtex(&self.title)));

        if !self.authors.is_empty() {
            let authors: Vec<String> = self
                .authors
                .iter()
                .map(|a| {
                    // BibTeX prefers "Surname, GivenNames" format
                    if let (Some(surname), Some(given)) = (&a.surname, &a.given_names) {
                        escape_bibtex(&format!("{surname}, {given}"))
                    } else {
                        escape_bibtex(&a.full_name)
                    }
                })
                .collect();
            lines.push(format!("  author = {{{}}},", authors.join(" and ")));
        }

        lines.push(format!("  journal = {{{}}},", escape_bibtex(&self.journal)));
        lines.push(format!("  year = {{{}}},", self.pub_date));

        if let Some(ref volume) = self.volume {
            lines.push(format!("  volume = {{{volume}}},"));
        }
        if let Some(ref issue) = self.issue {
            lines.push(format!("  number = {{{issue}}},"));
        }
        if let Some(ref pages) = self.pages {
            lines.push(format!("  pages = {{{pages}}},"));
        }
        if let Some(ref doi) = self.doi {
            lines.push(format!("  doi = {{{doi}}},"));
        }
        lines.push(format!("  pmid = {{{}}},", self.pmid));
        if let Some(ref pmc_id) = self.pmc_id {
            lines.push(format!("  pmcid = {{{pmc_id}}},"));
        }
        if let Some(ref issn) = self.issn {
            lines.push(format!("  issn = {{{issn}}},"));
        }
        if let Some(ref lang) = self.language {
            lines.push(format!("  language = {{{lang}}},"));
        }

        lines.push("}".to_string());

        lines.join("\n")
    }

    fn to_ris(&self) -> String {
        let mut lines = Vec::new();

        lines.push("TY  - JOUR".to_string());
        lines.push(format!("TI  - {}", self.title));

        for author in &self.authors {
            // RIS prefers "Surname, GivenNames" format
            if let (Some(surname), Some(given)) = (&author.surname, &author.given_names) {
                lines.push(format!("AU  - {surname}, {given}"));
            } else {
                lines.push(format!("AU  - {}", author.full_name));
            }
        }

        lines.push(format!("JO  - {}", self.journal));
        if let Some(ref abbr) = self.journal_abbreviation {
            lines.push(format!("JA  - {abbr}"));
        }

        lines.push(format!("PY  - {}", self.pub_date));

        if let Some(ref volume) = self.volume {
            lines.push(format!("VL  - {volume}"));
        }
        if let Some(ref issue) = self.issue {
            lines.push(format!("IS  - {issue}"));
        }
        if let Some(ref pages) = self.pages {
            // RIS uses SP (start page) and EP (end page)
            if let Some((start, end)) = pages.split_once('-') {
                lines.push(format!("SP  - {}", start.trim()));
                lines.push(format!("EP  - {}", end.trim()));
            } else {
                lines.push(format!("SP  - {pages}"));
            }
        }
        if let Some(ref doi) = self.doi {
            lines.push(format!("DO  - {doi}"));
        }
        lines.push(format!("AN  - PMID:{}", self.pmid));
        if let Some(ref pmc_id) = self.pmc_id {
            lines.push(format!("C1  - {pmc_id}"));
        }
        if let Some(ref issn) = self.issn {
            lines.push(format!("SN  - {issn}"));
        }
        if let Some(ref lang) = self.language {
            lines.push(format!("LA  - {lang}"));
        }
        if let Some(ref abstract_text) = self.abstract_text {
            lines.push(format!("AB  - {abstract_text}"));
        }
        for kw in self.keywords.as_deref().unwrap_or(&[]) {
            lines.push(format!("KW  - {kw}"));
        }

        lines.push("ER  - ".to_string());
        lines.join("\n")
    }

    fn to_csl_json(&self) -> Value {
        let mut csl = json!({
            "type": "article-journal",
            "id": format!("pmid:{}", self.pmid),
            "title": self.title,
            "container-title": self.journal,
        });

        // Authors
        if !self.authors.is_empty() {
            let authors: Vec<Value> = self
                .authors
                .iter()
                .map(|a| {
                    // Prefer structured name fields, fall back to splitting full_name
                    if a.surname.is_some() || a.given_names.is_some() {
                        let mut name = json!({});
                        if let Some(ref surname) = a.surname {
                            name["family"] = json!(surname);
                        }
                        if let Some(ref given) = a.given_names {
                            name["given"] = json!(given);
                        }
                        if let Some(ref suffix) = a.suffix {
                            name["suffix"] = json!(suffix);
                        }
                        name
                    } else {
                        let parts: Vec<&str> = a.full_name.rsplitn(2, ' ').collect();
                        if parts.len() == 2 {
                            json!({
                                "family": parts[0],
                                "given": parts[1]
                            })
                        } else {
                            json!({ "literal": a.full_name })
                        }
                    }
                })
                .collect();
            csl["author"] = Value::Array(authors);
        }

        // Date
        let year = self
            .pub_date
            .split_whitespace()
            .find(|s| s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()));
        if let Some(year) = year {
            csl["issued"] = json!({
                "date-parts": [[year.parse::<i32>().unwrap_or(0)]]
            });
        }

        if let Some(ref volume) = self.volume {
            csl["volume"] = json!(volume);
        }
        if let Some(ref issue) = self.issue {
            csl["issue"] = json!(issue);
        }
        if let Some(ref pages) = self.pages {
            csl["page"] = json!(pages);
        }
        if let Some(ref doi) = self.doi {
            csl["DOI"] = json!(doi);
        }
        csl["PMID"] = json!(self.pmid);
        if let Some(ref pmc_id) = self.pmc_id {
            csl["PMCID"] = json!(pmc_id);
        }
        if let Some(ref issn) = self.issn {
            csl["ISSN"] = json!(issn);
        }
        if let Some(ref lang) = self.language {
            csl["language"] = json!(lang);
        }
        if let Some(ref abstract_text) = self.abstract_text {
            csl["abstract"] = json!(abstract_text);
        }
        if let Some(ref abbr) = self.journal_abbreviation {
            csl["container-title-short"] = json!(abbr);
        }

        csl
    }

    fn to_nbib(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("PMID- {}", self.pmid));
        lines.push(format!("TI  - {}", self.title));

        for author in &self.authors {
            // NBIB uses FAU for full name and AU for abbreviated name
            lines.push(format!("FAU - {}", author.full_name));
            if let (Some(surname), Some(initials)) = (&author.surname, &author.initials) {
                lines.push(format!("AU  - {surname} {initials}"));
            }
        }

        lines.push(format!("TA  - {}", self.journal));
        if let Some(ref abbr) = self.journal_abbreviation {
            lines.push(format!("JT  - {abbr}"));
        }

        lines.push(format!("DP  - {}", self.pub_date));

        if let Some(ref volume) = self.volume {
            lines.push(format!("VI  - {volume}"));
        }
        if let Some(ref issue) = self.issue {
            lines.push(format!("IP  - {issue}"));
        }
        if let Some(ref pages) = self.pages {
            lines.push(format!("PG  - {pages}"));
        }
        if let Some(ref doi) = self.doi {
            lines.push(format!("AID - {doi} [doi]"));
        }
        if let Some(ref pmc_id) = self.pmc_id {
            lines.push(format!("PMC - {pmc_id}"));
        }
        if let Some(ref issn) = self.issn {
            lines.push(format!("IS  - {issn}"));
        }
        if let Some(ref lang) = self.language {
            lines.push(format!("LA  - {lang}"));
        }
        if let Some(ref abstract_text) = self.abstract_text {
            lines.push(format!("AB  - {abstract_text}"));
        }
        for pt in &self.article_types {
            lines.push(format!("PT  - {pt}"));
        }

        lines.join("\n")
    }
}

/// Export multiple articles as a single BibTeX string
pub fn articles_to_bibtex(articles: &[PubMedArticle]) -> String {
    articles
        .iter()
        .map(|a| a.to_bibtex())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Export multiple articles as a single RIS string
pub fn articles_to_ris(articles: &[PubMedArticle]) -> String {
    articles
        .iter()
        .map(|a| a.to_ris())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Export multiple articles as a CSL-JSON array
pub fn articles_to_csl_json(articles: &[PubMedArticle]) -> Value {
    Value::Array(articles.iter().map(|a| a.to_csl_json()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pubmed_parser::common::{Affiliation, Author};

    fn create_test_article() -> PubMedArticle {
        PubMedArticle {
            pmid: "33515491".to_string(),
            title: "Effectiveness of COVID-19 Vaccines".to_string(),
            authors: vec![
                Author {
                    surname: Some("Smith".to_string()),
                    given_names: Some("John".to_string()),
                    initials: Some("J".to_string()),
                    suffix: None,
                    full_name: "John Smith".to_string(),
                    orcid: None,
                    email: None,
                    is_corresponding: false,
                    roles: vec![],
                    affiliations: vec![Affiliation {
                        id: None,
                        institution: Some("Harvard University".to_string()),
                        department: None,
                        address: None,
                        country: None,
                    }],
                },
                Author {
                    surname: Some("Doe".to_string()),
                    given_names: Some("Jane".to_string()),
                    initials: Some("J".to_string()),
                    suffix: None,
                    full_name: "Jane Doe".to_string(),
                    orcid: None,
                    email: None,
                    is_corresponding: false,
                    roles: vec![],
                    affiliations: vec![],
                },
            ],
            author_count: 2,
            journal: "The Lancet".to_string(),
            pub_date: "2021".to_string(),
            doi: Some("10.1016/S0140-6736(21)00234-8".to_string()),
            pmc_id: Some("PMC7906746".to_string()),
            abstract_text: Some("Background: COVID-19 vaccines have been developed...".to_string()),
            structured_abstract: None,
            article_types: vec!["Journal Article".to_string()],
            mesh_headings: None,
            keywords: Some(vec!["COVID-19".to_string(), "Vaccine".to_string()]),
            chemical_list: None,
            volume: Some("397".to_string()),
            issue: Some("10275".to_string()),
            pages: Some("671-681".to_string()),
            language: Some("eng".to_string()),
            journal_abbreviation: Some("Lancet".to_string()),
            issn: Some("0140-6736".to_string()),
        }
    }

    #[test]
    fn test_bibtex_export() {
        let article = create_test_article();
        let bibtex = article.to_bibtex();

        assert!(bibtex.starts_with("@article{John2021_pmid33515491,"));
        assert!(bibtex.contains("title = {Effectiveness of COVID-19 Vaccines}"));
        assert!(bibtex.contains("author = {Smith, John and Doe, Jane}"));
        assert!(bibtex.contains("journal = {The Lancet}"));
        assert!(bibtex.contains("year = {2021}"));
        assert!(bibtex.contains("volume = {397}"));
        assert!(bibtex.contains("doi = {10.1016/S0140-6736(21)00234-8}"));
        assert!(bibtex.contains("pmid = {33515491}"));
        assert!(bibtex.contains("pmcid = {PMC7906746}"));
        assert!(bibtex.ends_with('}'));
    }

    #[test]
    fn test_ris_export() {
        let article = create_test_article();
        let ris = article.to_ris();

        assert!(ris.starts_with("TY  - JOUR"));
        assert!(ris.contains("TI  - Effectiveness of COVID-19 Vaccines"));
        assert!(ris.contains("AU  - Smith, John"));
        assert!(ris.contains("AU  - Doe, Jane"));
        assert!(ris.contains("JO  - The Lancet"));
        assert!(ris.contains("PY  - 2021"));
        assert!(ris.contains("VL  - 397"));
        assert!(ris.contains("SP  - 671"));
        assert!(ris.contains("EP  - 681"));
        assert!(ris.contains("DO  - 10.1016/S0140-6736(21)00234-8"));
        assert!(ris.contains("KW  - COVID-19"));
        assert!(ris.contains("KW  - Vaccine"));
        assert!(ris.ends_with("ER  - "));
    }

    #[test]
    fn test_csl_json_export() {
        let article = create_test_article();
        let csl = article.to_csl_json();

        assert_eq!(csl["type"], "article-journal");
        assert_eq!(csl["title"], "Effectiveness of COVID-19 Vaccines");
        assert_eq!(csl["container-title"], "The Lancet");
        assert_eq!(csl["volume"], "397");
        assert_eq!(csl["issue"], "10275");
        assert_eq!(csl["page"], "671-681");
        assert_eq!(csl["DOI"], "10.1016/S0140-6736(21)00234-8");
        assert_eq!(csl["PMID"], "33515491");
        assert_eq!(csl["PMCID"], "PMC7906746");
        assert_eq!(csl["language"], "eng");
        assert_eq!(csl["container-title-short"], "Lancet");

        // Check authors (uses structured surname/given_names fields)
        let authors = csl["author"].as_array().unwrap();
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0]["family"], "Smith");
        assert_eq!(authors[0]["given"], "John");
        assert_eq!(authors[1]["family"], "Doe");
        assert_eq!(authors[1]["given"], "Jane");
    }

    #[test]
    fn test_nbib_export() {
        let article = create_test_article();
        let nbib = article.to_nbib();

        assert!(nbib.contains("PMID- 33515491"));
        assert!(nbib.contains("TI  - Effectiveness of COVID-19 Vaccines"));
        assert!(nbib.contains("FAU - John Smith"));
        assert!(nbib.contains("AU  - Smith J"));
        assert!(nbib.contains("FAU - Jane Doe"));
        assert!(nbib.contains("AU  - Doe J"));
        assert!(nbib.contains("TA  - The Lancet"));
        assert!(nbib.contains("DP  - 2021"));
        assert!(nbib.contains("VI  - 397"));
        assert!(nbib.contains("AID - 10.1016/S0140-6736(21)00234-8 [doi]"));
        assert!(nbib.contains("PMC - PMC7906746"));
    }

    #[test]
    fn test_batch_bibtex_export() {
        let article = create_test_article();
        let articles = vec![article.clone(), article];
        let bibtex = articles_to_bibtex(&articles);

        // Should contain two entries separated by blank line
        let entries: Vec<&str> = bibtex.split("\n\n").collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_batch_csl_json_export() {
        let article = create_test_article();
        let articles = vec![article.clone(), article];
        let csl = articles_to_csl_json(&articles);

        assert!(csl.is_array());
        assert_eq!(csl.as_array().unwrap().len(), 2);
    }
}
