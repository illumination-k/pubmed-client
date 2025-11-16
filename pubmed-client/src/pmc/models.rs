use serde::{Deserialize, Serialize};

// Re-export common types
pub use crate::common::{Affiliation, Author};

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
    /// File path to the extracted figure image (when available)
    pub file_path: Option<String>,
    /// File name from the XML href attribute (when available)
    pub file_name: Option<String>,
}

/// Represents an extracted figure with both XML metadata and file path
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtractedFigure {
    /// Figure metadata from XML
    pub figure: Figure,
    /// Actual file path where the figure was extracted
    pub extracted_file_path: String,
    /// File size in bytes
    pub file_size: Option<u64>,
    /// Image dimensions (width, height) if available
    pub dimensions: Option<(u32, u32)>,
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

/// Represents supplementary material in the article
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SupplementaryMaterial {
    /// Supplementary material ID
    pub id: String,
    /// Content type (e.g., "local-data")
    pub content_type: Option<String>,
    /// Title/caption of the supplementary material
    pub title: Option<String>,
    /// Description or additional caption
    pub description: Option<String>,
    /// File URL or path (from xlink:href)
    pub file_url: Option<String>,
    /// File extension/type inferred from URL
    pub file_type: Option<String>,
    /// Position attribute (e.g., "float")
    pub position: Option<String>,
}

impl SupplementaryMaterial {
    /// Create a new SupplementaryMaterial instance
    pub fn new(id: String) -> Self {
        Self {
            id,
            content_type: None,
            title: None,
            description: None,
            file_url: None,
            file_type: None,
            position: None,
        }
    }

    /// Check if this supplementary material is a tar file
    pub fn is_tar_file(&self) -> bool {
        if let Some(url) = &self.file_url {
            url.ends_with(".tar")
                || url.ends_with(".tar.gz")
                || url.ends_with(".tar.bz2")
                || url.ends_with(".tgz")
        } else {
            false
        }
    }

    /// Get the file extension from the URL
    pub fn get_file_extension(&self) -> Option<String> {
        if let Some(url) = &self.file_url {
            if let Some(filename) = url.split('/').next_back() {
                if let Some(dot_index) = filename.rfind('.') {
                    return Some(filename[dot_index + 1..].to_lowercase());
                }
            }
        }
        None
    }

    /// Check if this is an archive file (zip, tar, etc.)
    pub fn is_archive(&self) -> bool {
        if let Some(ext) = self.get_file_extension() {
            matches!(
                ext.as_str(),
                "zip" | "tar" | "gz" | "bz2" | "tgz" | "rar" | "7z"
            )
        } else {
            false
        }
    }
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
    /// Supplementary materials
    pub supplementary_materials: Vec<SupplementaryMaterial>,
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
            supplementary_materials: Vec::new(),
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

    /// Get all tar files from supplementary materials
    pub fn get_tar_files(&self) -> Vec<&SupplementaryMaterial> {
        self.supplementary_materials
            .iter()
            .filter(|material| material.is_tar_file())
            .collect()
    }

    /// Get all archive files from supplementary materials
    pub fn get_archive_files(&self) -> Vec<&SupplementaryMaterial> {
        self.supplementary_materials
            .iter()
            .filter(|material| material.is_archive())
            .collect()
    }

    /// Check if the article has supplementary materials
    pub fn has_supplementary_materials(&self) -> bool {
        !self.supplementary_materials.is_empty()
    }

    /// Get supplementary materials by content type
    pub fn get_supplementary_materials_by_type(
        &self,
        content_type: &str,
    ) -> Vec<&SupplementaryMaterial> {
        self.supplementary_materials
            .iter()
            .filter(|material| {
                material
                    .content_type
                    .as_ref()
                    .is_some_and(|ct| ct == content_type)
            })
            .collect()
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
                        journal_part.push_str(&format!(" ({year})"));
                    }
                }
                if let Some(volume) = &self.volume {
                    if !volume.trim().is_empty() {
                        journal_part.push_str(&format!(" {volume}"));
                        if let Some(issue) = &self.issue {
                            if !issue.trim().is_empty() {
                                journal_part.push_str(&format!("({issue})"));
                            }
                        }
                    }
                }
                if let Some(pages) = &self.pages {
                    if !pages.trim().is_empty() {
                        journal_part.push_str(&format!(": {pages}"));
                    }
                }
                parts.push(journal_part);
            }
        }

        // If no meaningful parts found, use the reference ID as fallback
        let result = parts.join(". ");
        if result.trim().is_empty() {
            let id = &self.id;
            format!("Reference {id}")
        } else {
            result
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
            file_path: None,
            file_name: None,
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
            Author::from_full_name("Smith, J.".to_string()),
            Author::from_full_name("Doe, A.".to_string()),
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

    #[test]
    fn test_supplementary_material_creation() {
        let mut material = SupplementaryMaterial::new("supp1".to_string());
        material.file_url = Some("https://example.com/data.tar.gz".to_string());
        material.content_type = Some("local-data".to_string());
        material.title = Some("Supplementary Data".to_string());

        assert_eq!(material.id, "supp1");
        assert!(material.is_tar_file());
        assert!(material.is_archive());
        assert_eq!(material.get_file_extension(), Some("gz".to_string()));
    }

    #[test]
    fn test_tar_file_detection() {
        let mut material = SupplementaryMaterial::new("tar1".to_string());

        // Test various tar file extensions
        material.file_url = Some("data.tar".to_string());
        assert!(material.is_tar_file());

        material.file_url = Some("data.tar.gz".to_string());
        assert!(material.is_tar_file());

        material.file_url = Some("data.tar.bz2".to_string());
        assert!(material.is_tar_file());

        material.file_url = Some("data.tgz".to_string());
        assert!(material.is_tar_file());

        // Test non-tar files
        material.file_url = Some("data.zip".to_string());
        assert!(!material.is_tar_file());
        assert!(material.is_archive());

        material.file_url = Some("data.pdf".to_string());
        assert!(!material.is_tar_file());
        assert!(!material.is_archive());
    }

    #[test]
    fn test_pmc_full_text_with_supplementary_materials() {
        let mut article = PmcFullText::new("PMC1234567".to_string());

        let mut tar_material = SupplementaryMaterial::new("supp1".to_string());
        tar_material.file_url = Some("dataset.tar.gz".to_string());
        tar_material.content_type = Some("local-data".to_string());

        let mut zip_material = SupplementaryMaterial::new("supp2".to_string());
        zip_material.file_url = Some("figures.zip".to_string());
        zip_material.content_type = Some("local-data".to_string());

        article.supplementary_materials.push(tar_material);
        article.supplementary_materials.push(zip_material);

        assert!(article.has_supplementary_materials());
        assert_eq!(article.supplementary_materials.len(), 2);

        let tar_files = article.get_tar_files();
        assert_eq!(tar_files.len(), 1);
        assert_eq!(tar_files[0].id, "supp1");

        let archive_files = article.get_archive_files();
        assert_eq!(archive_files.len(), 2);

        let local_data_materials = article.get_supplementary_materials_by_type("local-data");
        assert_eq!(local_data_materials.len(), 2);
    }
}
