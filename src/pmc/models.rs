use serde::{Deserialize, Serialize};

/// Represents an author with detailed information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Author {
    /// Given names (first name, middle names)
    pub given_names: Option<String>,
    /// Surname (last name)
    pub surname: Option<String>,
    /// Full name (formatted)
    pub full_name: String,
    /// Affiliations
    pub affiliations: Vec<Affiliation>,
    /// ORCID ID
    pub orcid: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Author roles/contributions
    pub roles: Vec<String>,
    /// Corresponding author flag
    pub is_corresponding: bool,
}

/// Represents an institutional affiliation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Affiliation {
    /// Affiliation ID
    pub id: Option<String>,
    /// Institution name
    pub institution: String,
    /// Department
    pub department: Option<String>,
    /// Address
    pub address: Option<String>,
    /// Country
    pub country: Option<String>,
}

/// Represents journal information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JournalInfo {
    /// Journal title
    pub title: String,
    /// Journal abbreviation
    pub abbreviation: Option<String>,
    /// ISSN (print)
    pub issn_print: Option<String>,
    /// ISSN (electronic)
    pub issn_electronic: Option<String>,
    /// Publisher name
    pub publisher: Option<String>,
    /// Volume
    pub volume: Option<String>,
    /// Issue
    pub issue: Option<String>,
}

/// Represents funding information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FundingInfo {
    /// Funding source/agency
    pub source: String,
    /// Grant/award ID
    pub award_id: Option<String>,
    /// Funding statement
    pub statement: Option<String>,
}

/// Represents a figure in the article
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Figure {
    /// Figure ID
    pub id: String,
    /// Figure label (e.g., "Figure 1")
    pub label: Option<String>,
    /// Figure caption
    pub caption: String,
    /// Alt text description
    pub alt_text: Option<String>,
    /// Figure type (e.g., "figure", "scheme", "chart")
    pub fig_type: Option<String>,
}

/// Represents a table in the article
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Table {
    /// Table ID
    pub id: String,
    /// Table label (e.g., "Table 1")
    pub label: Option<String>,
    /// Table caption
    pub caption: String,
    /// Table footnotes
    pub footnotes: Vec<String>,
}

/// Represents a full-text article from PMC
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PmcFullText {
    /// PMC ID (e.g., "PMC1234567")
    pub pmcid: String,
    /// PubMed ID (if available)
    pub pmid: Option<String>,
    /// Article title
    pub title: String,
    /// List of authors with detailed information
    pub authors: Vec<Author>,
    /// Journal information
    pub journal: JournalInfo,
    /// Publication date
    pub pub_date: String,
    /// DOI (Digital Object Identifier)
    pub doi: Option<String>,
    /// Article sections with content
    pub sections: Vec<ArticleSection>,
    /// List of references
    pub references: Vec<Reference>,
    /// Article type (if available)
    pub article_type: Option<String>,
    /// Keywords
    pub keywords: Vec<String>,
    /// Funding information
    pub funding: Vec<FundingInfo>,
    /// Conflict of interest statement
    pub conflict_of_interest: Option<String>,
    /// Acknowledgments
    pub acknowledgments: Option<String>,
    /// Data availability statement
    pub data_availability: Option<String>,
}

/// Represents a section of an article
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleSection {
    /// Type of section (e.g., "abstract", "introduction", "methods")
    pub section_type: String,
    /// Section title (if available)
    pub title: Option<String>,
    /// Section content
    pub content: String,
    /// Nested subsections
    pub subsections: Vec<ArticleSection>,
    /// Section ID (if available)
    pub id: Option<String>,
    /// Figures in this section
    pub figures: Vec<Figure>,
    /// Tables in this section
    pub tables: Vec<Table>,
}

/// Represents a reference citation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reference {
    /// Reference identifier
    pub id: String,
    /// Reference title
    pub title: Option<String>,
    /// List of authors with detailed information
    pub authors: Vec<Author>,
    /// Journal name
    pub journal: Option<String>,
    /// Publication year
    pub year: Option<String>,
    /// Volume
    pub volume: Option<String>,
    /// Issue
    pub issue: Option<String>,
    /// Page range
    pub pages: Option<String>,
    /// PubMed ID (if available)
    pub pmid: Option<String>,
    /// DOI (if available)
    pub doi: Option<String>,
    /// Reference type (e.g., "journal", "book", "web")
    pub ref_type: Option<String>,
}

impl PmcFullText {
    /// Create a new PmcFullText instance
    pub fn new(pmcid: String) -> Self {
        Self {
            pmcid,
            pmid: None,
            title: String::new(),
            authors: Vec::new(),
            journal: JournalInfo {
                title: String::new(),
                abbreviation: None,
                issn_print: None,
                issn_electronic: None,
                publisher: None,
                volume: None,
                issue: None,
            },
            pub_date: String::new(),
            doi: None,
            sections: Vec::new(),
            references: Vec::new(),
            article_type: None,
            keywords: Vec::new(),
            funding: Vec::new(),
            conflict_of_interest: None,
            acknowledgments: None,
            data_availability: None,
        }
    }

    /// Check if the article has full text content
    pub fn has_content(&self) -> bool {
        !self.sections.is_empty() || !self.title.is_empty()
    }

    /// Get the total number of sections (including subsections)
    pub fn total_sections(&self) -> usize {
        fn count_sections(sections: &[ArticleSection]) -> usize {
            sections.iter().fold(0, |acc, section| {
                acc + 1 + count_sections(&section.subsections)
            })
        }
        count_sections(&self.sections)
    }

    /// Get all section content as a single string
    pub fn get_full_text(&self) -> String {
        fn collect_content(sections: &[ArticleSection]) -> String {
            sections
                .iter()
                .map(|section| {
                    let mut content = section.content.clone();
                    if !section.subsections.is_empty() {
                        content.push('\n');
                        content.push_str(&collect_content(&section.subsections));
                    }
                    content
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        }
        collect_content(&self.sections)
    }
}

impl ArticleSection {
    /// Create a new ArticleSection instance
    pub fn new(section_type: String, content: String) -> Self {
        Self {
            section_type,
            title: None,
            content,
            subsections: Vec::new(),
            id: None,
            figures: Vec::new(),
            tables: Vec::new(),
        }
    }

    /// Create a new ArticleSection with title
    pub fn with_title(section_type: String, title: String, content: String) -> Self {
        Self {
            section_type,
            title: Some(title),
            content,
            subsections: Vec::new(),
            id: None,
            figures: Vec::new(),
            tables: Vec::new(),
        }
    }

    /// Create a new ArticleSection with ID
    pub fn with_id(section_type: String, content: String, id: String) -> Self {
        Self {
            section_type,
            title: None,
            content,
            subsections: Vec::new(),
            id: Some(id),
            figures: Vec::new(),
            tables: Vec::new(),
        }
    }

    /// Add a subsection
    pub fn add_subsection(&mut self, subsection: ArticleSection) {
        self.subsections.push(subsection);
    }

    /// Check if section has content
    pub fn has_content(&self) -> bool {
        !self.content.trim().is_empty() || !self.subsections.is_empty()
    }
}

impl Reference {
    /// Create a new Reference instance
    pub fn new(id: String) -> Self {
        Self {
            id,
            title: None,
            authors: Vec::new(),
            journal: None,
            year: None,
            volume: None,
            issue: None,
            pages: None,
            pmid: None,
            doi: None,
            ref_type: None,
        }
    }

    /// Create a basic reference with minimal information
    pub fn basic(id: String, title: Option<String>, journal: Option<String>) -> Self {
        Self {
            id,
            title,
            authors: Vec::new(),
            journal,
            year: None,
            volume: None,
            issue: None,
            pages: None,
            pmid: None,
            doi: None,
            ref_type: None,
        }
    }

    /// Format reference as citation string
    pub fn format_citation(&self) -> String {
        let mut parts = Vec::new();

        if !self.authors.is_empty() {
            let author_names: Vec<String> = self
                .authors
                .iter()
                .map(|author| author.full_name.clone())
                .filter(|name| !name.trim().is_empty())
                .collect();
            if !author_names.is_empty() {
                parts.push(author_names.join(", "));
            }
        }

        if let Some(title) = &self.title {
            if !title.trim().is_empty() {
                parts.push(title.clone());
            }
        }

        if let Some(journal) = &self.journal {
            if !journal.trim().is_empty() {
                let mut journal_part = journal.clone();
                if let Some(year) = &self.year {
                    if !year.trim().is_empty() && year != "n.d." {
                        journal_part.push_str(&format!(" ({})", year));
                    }
                }
                if let Some(volume) = &self.volume {
                    if !volume.trim().is_empty() {
                        journal_part.push_str(&format!(" {}", volume));
                        if let Some(issue) = &self.issue {
                            if !issue.trim().is_empty() {
                                journal_part.push_str(&format!("({})", issue));
                            }
                        }
                    }
                }
                if let Some(pages) = &self.pages {
                    if !pages.trim().is_empty() {
                        journal_part.push_str(&format!(": {}", pages));
                    }
                }
                parts.push(journal_part);
            }
        }

        // If no meaningful parts found, use the reference ID as fallback
        let result = parts.join(". ");
        if result.trim().is_empty() {
            format!("Reference {}", self.id)
        } else {
            result
        }
    }
}

impl Author {
    /// Create a new Author instance
    pub fn new(full_name: String) -> Self {
        Self {
            given_names: None,
            surname: None,
            full_name,
            affiliations: Vec::new(),
            orcid: None,
            email: None,
            roles: Vec::new(),
            is_corresponding: false,
        }
    }

    /// Create an author with separated names
    pub fn with_names(given_names: Option<String>, surname: Option<String>) -> Self {
        let full_name = match (&given_names, &surname) {
            (Some(given), Some(sur)) => format!("{} {}", given, sur),
            (Some(given), None) => given.clone(),
            (None, Some(sur)) => sur.clone(),
            (None, None) => "Unknown Author".to_string(),
        };

        Self {
            given_names,
            surname,
            full_name,
            affiliations: Vec::new(),
            orcid: None,
            email: None,
            roles: Vec::new(),
            is_corresponding: false,
        }
    }
}

impl Affiliation {
    /// Create a new Affiliation instance
    pub fn new(institution: String) -> Self {
        Self {
            id: None,
            institution,
            department: None,
            address: None,
            country: None,
        }
    }
}

impl JournalInfo {
    /// Create a new JournalInfo instance
    pub fn new(title: String) -> Self {
        Self {
            title,
            abbreviation: None,
            issn_print: None,
            issn_electronic: None,
            publisher: None,
            volume: None,
            issue: None,
        }
    }
}

impl FundingInfo {
    /// Create a new FundingInfo instance
    pub fn new(source: String) -> Self {
        Self {
            source,
            award_id: None,
            statement: None,
        }
    }
}

impl Figure {
    /// Create a new Figure instance
    pub fn new(id: String, caption: String) -> Self {
        Self {
            id,
            label: None,
            caption,
            alt_text: None,
            fig_type: None,
        }
    }
}

impl Table {
    /// Create a new Table instance
    pub fn new(id: String, caption: String) -> Self {
        Self {
            id,
            label: None,
            caption,
            footnotes: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmc_full_text_creation() {
        let article = PmcFullText::new("PMC1234567".to_string());
        assert_eq!(article.pmcid, "PMC1234567");
        assert!(!article.has_content());
        assert_eq!(article.total_sections(), 0);
    }

    #[test]
    fn test_article_section_creation() {
        let mut section =
            ArticleSection::new("abstract".to_string(), "This is an abstract.".to_string());
        assert!(section.has_content());
        assert_eq!(section.subsections.len(), 0);

        let subsection = ArticleSection::new("method".to_string(), "Method details.".to_string());
        section.add_subsection(subsection);
        assert_eq!(section.subsections.len(), 1);
    }

    #[test]
    fn test_reference_formatting() {
        let mut reference = Reference::new("ref1".to_string());
        reference.authors = vec![
            Author::new("Smith, J.".to_string()),
            Author::new("Doe, A.".to_string()),
        ];
        reference.title = Some("Test Article".to_string());
        reference.journal = Some("Test Journal".to_string());
        reference.year = Some("2023".to_string());

        let citation = reference.format_citation();
        assert!(citation.contains("Smith, J., Doe, A."));
        assert!(citation.contains("Test Article"));
        assert!(citation.contains("Test Journal (2023)"));
    }

    #[test]
    fn test_full_text_content() {
        let mut article = PmcFullText::new("PMC1234567".to_string());

        let section1 = ArticleSection::new("abstract".to_string(), "Abstract content.".to_string());
        let section2 = ArticleSection::new(
            "introduction".to_string(),
            "Introduction content.".to_string(),
        );

        article.sections.push(section1);
        article.sections.push(section2);

        assert!(article.has_content());
        assert_eq!(article.total_sections(), 2);

        let full_text = article.get_full_text();
        assert!(full_text.contains("Abstract content."));
        assert!(full_text.contains("Introduction content."));
    }
}
