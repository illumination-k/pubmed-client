use serde::{Deserialize, Serialize};

/// Represents a full-text article from PMC
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PmcFullText {
    /// PMC ID (e.g., "PMC1234567")
    pub pmcid: String,
    /// PubMed ID (if available)
    pub pmid: Option<String>,
    /// Article title
    pub title: String,
    /// List of authors
    pub authors: Vec<String>,
    /// Journal name
    pub journal: String,
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
}

/// Represents a reference citation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Reference {
    /// Reference identifier
    pub id: String,
    /// Reference title
    pub title: Option<String>,
    /// List of authors
    pub authors: Vec<String>,
    /// Journal name
    pub journal: Option<String>,
    /// Publication year
    pub year: Option<String>,
    /// PubMed ID (if available)
    pub pmid: Option<String>,
    /// DOI (if available)
    pub doi: Option<String>,
}

impl PmcFullText {
    /// Create a new PmcFullText instance
    pub fn new(pmcid: String) -> Self {
        Self {
            pmcid,
            pmid: None,
            title: String::new(),
            authors: Vec::new(),
            journal: String::new(),
            pub_date: String::new(),
            doi: None,
            sections: Vec::new(),
            references: Vec::new(),
            article_type: None,
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
        }
    }

    /// Create a new ArticleSection with title
    pub fn with_title(section_type: String, title: String, content: String) -> Self {
        Self {
            section_type,
            title: Some(title),
            content,
            subsections: Vec::new(),
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
            pmid: None,
            doi: None,
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
            pmid: None,
            doi: None,
        }
    }

    /// Format reference as citation string
    pub fn format_citation(&self) -> String {
        let mut parts = Vec::new();

        if !self.authors.is_empty() {
            parts.push(self.authors.join(", "));
        }

        if let Some(title) = &self.title {
            parts.push(title.clone());
        }

        if let Some(journal) = &self.journal {
            let mut journal_part = journal.clone();
            if let Some(year) = &self.year {
                journal_part.push_str(&format!(" ({})", year));
            }
            parts.push(journal_part);
        }

        parts.join(". ")
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
        reference.authors = vec!["Smith, J.".to_string(), "Doe, A.".to_string()];
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
