use crate::error::{PubMedError, Result};
use reqwest::Client;
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

/// Client for interacting with PMC (PubMed Central) API
#[derive(Clone)]
pub struct PmcClient {
    client: Client,
    base_url: String,
}

impl PmcClient {
    /// Create a new PMC client
    ///
    /// # Example
    ///
    /// ```
    /// use pubmed_client::PmcClient;
    ///
    /// let client = PmcClient::new();
    /// ```
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://eutils.ncbi.nlm.nih.gov/entrez/eutils".to_string(),
        }
    }

    /// Create a new PMC client with custom HTTP client
    ///
    /// # Arguments
    ///
    /// * `client` - Custom reqwest client with specific configuration
    pub fn with_client(client: Client) -> Self {
        Self {
            client,
            base_url: "https://eutils.ncbi.nlm.nih.gov/entrez/eutils".to_string(),
        }
    }

    /// Fetch full text from PMC using PMCID
    ///
    /// # Arguments
    ///
    /// * `pmcid` - PMC ID (with or without "PMC" prefix)
    ///
    /// # Returns
    ///
    /// Returns a `Result<PmcFullText>` containing the structured full text
    ///
    /// # Errors
    ///
    /// * `PubMedError::PmcNotAvailable` - If PMC full text is not available
    /// * `PubMedError::RequestError` - If the HTTP request fails
    /// * `PubMedError::XmlError` - If XML parsing fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PmcClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     let full_text = client.fetch_full_text("PMC7906746").await?;
    ///     println!("Title: {}", full_text.title);
    ///     println!("Sections: {}", full_text.sections.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn fetch_full_text(&self, pmcid: &str) -> Result<PmcFullText> {
        // Remove PMC prefix if present and validate
        let clean_pmcid = pmcid.trim_start_matches("PMC");
        if clean_pmcid.is_empty() || !clean_pmcid.chars().all(|c| c.is_ascii_digit()) {
            return Err(PubMedError::InvalidPmid {
                pmid: pmcid.to_string(),
            });
        }

        let fetch_url = format!(
            "{}/efetch.fcgi?db=pmc&id=PMC{}&retmode=xml",
            self.base_url, clean_pmcid
        );

        let response = self.client.get(&fetch_url).send().await?;

        if !response.status().is_success() {
            return Err(PubMedError::ApiError {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ),
            });
        }

        let xml_content = response.text().await?;

        // Parse XML and extract structured data
        self.parse_pmc_xml(&xml_content, &format!("PMC{}", clean_pmcid))
    }

    /// Check if PMC full text is available for a given PMID
    ///
    /// # Arguments
    ///
    /// * `pmid` - PubMed ID
    ///
    /// # Returns
    ///
    /// Returns `Result<Option<String>>` containing the PMCID if available
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pubmed_client::PmcClient;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = PmcClient::new();
    ///     if let Some(pmcid) = client.check_pmc_availability("33515491").await? {
    ///         println!("PMC available: {}", pmcid);
    ///         let full_text = client.fetch_full_text(&pmcid).await?;
    ///         println!("Title: {}", full_text.title);
    ///     } else {
    ///         println!("PMC not available");
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn check_pmc_availability(&self, pmid: &str) -> Result<Option<String>> {
        // Validate PMID format
        if pmid.trim().is_empty() || !pmid.chars().all(|c| c.is_ascii_digit()) {
            return Err(PubMedError::InvalidPmid {
                pmid: pmid.to_string(),
            });
        }

        let link_url = format!(
            "{}/elink.fcgi?dbfrom=pubmed&db=pmc&id={}&retmode=json",
            self.base_url, pmid
        );

        let response = self.client.get(&link_url).send().await?;

        if !response.status().is_success() {
            return Err(PubMedError::ApiError {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response
                        .status()
                        .canonical_reason()
                        .unwrap_or("Unknown error")
                ),
            });
        }

        let link_result: serde_json::Value = response.json().await?;

        // Extract PMCID from response
        if let Some(linksets) = link_result["linksets"].as_array() {
            for linkset in linksets {
                if let Some(linksetdbs) = linkset["linksetdbs"].as_array() {
                    for linksetdb in linksetdbs {
                        if linksetdb["dbto"] == "pmc" {
                            if let Some(links) = linksetdb["links"].as_array() {
                                if let Some(pmcid) = links.first() {
                                    return Ok(Some(format!("PMC{}", pmcid)));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Parse PMC XML content into structured data
    fn parse_pmc_xml(&self, xml_content: &str, pmcid: &str) -> Result<PmcFullText> {
        // Extract title
        let title = self
            .extract_text_between(xml_content, "<article-title>", "</article-title>")
            .unwrap_or_else(|| "Unknown Title".to_string());

        // Extract authors
        let authors = self.extract_authors_simple(xml_content);

        // Extract journal
        let journal = self
            .extract_text_between(xml_content, "<journal-title>", "</journal-title>")
            .unwrap_or_else(|| "Unknown Journal".to_string());

        // Extract publication date
        let pub_date = self.extract_pub_date_simple(xml_content);

        // Extract DOI
        let doi = self.extract_doi_simple(xml_content);

        // Extract PMID
        let pmid = self.extract_pmid_simple(xml_content);

        // Extract sections
        let sections = self.extract_sections_simple(xml_content);

        // Extract references
        let references = self.extract_references_simple(xml_content);

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

    // Helper methods for XML parsing (simplified implementation)
    fn extract_text_between(&self, content: &str, start: &str, end: &str) -> Option<String> {
        let start_pos = content.find(start)? + start.len();
        let end_pos = content[start_pos..].find(end)? + start_pos;
        Some(content[start_pos..end_pos].trim().to_string())
    }

    fn extract_authors_simple(&self, content: &str) -> Vec<String> {
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

    fn extract_pub_date_simple(&self, content: &str) -> String {
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

    fn extract_doi_simple(&self, content: &str) -> Option<String> {
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

    fn extract_pmid_simple(&self, content: &str) -> Option<String> {
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

    fn extract_sections_simple(&self, content: &str) -> Vec<ArticleSection> {
        let mut sections = Vec::new();

        if let Some(body_start) = content.find("<body>") {
            if let Some(body_end) = content[body_start..].find("</body>") {
                let body_content = &content[body_start + 6..body_start + body_end];

                let mut para_content = String::new();
                let mut pos = 0;

                while let Some(p_start) = body_content[pos..].find("<p ") {
                    let p_start = pos + p_start;
                    if let Some(content_start) = body_content[p_start..].find(">") {
                        let content_start = p_start + content_start + 1;
                        if let Some(p_end) = body_content[content_start..].find("</p>") {
                            let p_end = content_start + p_end;
                            let paragraph = &body_content[content_start..p_end];
                            let clean_text = self.strip_xml_tags(paragraph);
                            para_content.push_str(&clean_text);
                            para_content.push('\n');
                            pos = p_end;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if !para_content.is_empty() {
                    sections.push(ArticleSection {
                        section_type: "body".to_string(),
                        title: Some("Main Content".to_string()),
                        content: para_content.trim().to_string(),
                        subsections: Vec::new(),
                    });
                }
            }
        }

        sections
    }

    fn extract_references_simple(&self, content: &str) -> Vec<Reference> {
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

                            references.push(Reference {
                                id,
                                title,
                                authors: Vec::new(), // Simplified implementation
                                journal,
                                year,
                                pmid: None,
                                doi: None,
                            });

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

impl Default for PmcClient {
    fn default() -> Self {
        Self::new()
    }
}
