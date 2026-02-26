use serde::{Deserialize, Serialize};

// Re-export common types
pub use crate::common::{Affiliation, Author};

/// A labeled section within a structured abstract
///
/// PubMed articles may have structured abstracts with labeled sections such as
/// "BACKGROUND", "METHODS", "RESULTS", and "CONCLUSIONS". This type represents
/// a single section of such a structured abstract.
///
/// # Example
///
/// ```
/// use pubmed_parser::pubmed::AbstractSection;
///
/// let section = AbstractSection {
///     label: "BACKGROUND".to_string(),
///     text: "This study investigates...".to_string(),
/// };
/// ```
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AbstractSection {
    /// Section label (e.g., "BACKGROUND", "METHODS", "RESULTS", "CONCLUSIONS")
    pub label: String,
    /// Text content of the section
    pub text: String,
}

/// Represents a PubMed article with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PubMedArticle {
    /// PubMed ID
    pub pmid: String,
    /// Article title
    pub title: String,
    /// List of authors with detailed metadata
    pub authors: Vec<Author>,
    /// Number of authors (computed from authors list)
    pub author_count: u32,
    /// Journal name
    pub journal: String,
    /// Publication date
    pub pub_date: String,
    /// DOI (Digital Object Identifier)
    pub doi: Option<String>,
    /// PMC ID if available (with PMC prefix, e.g., "PMC7092803")
    pub pmc_id: Option<String>,
    /// Abstract text (if available)
    pub abstract_text: Option<String>,
    /// Structured abstract sections with labels (if available)
    ///
    /// Some PubMed articles have structured abstracts with labeled sections like
    /// "BACKGROUND", "METHODS", "RESULTS", "CONCLUSIONS". When available, this
    /// field contains each section separately. The `abstract_text` field still
    /// contains the full concatenated text.
    pub structured_abstract: Option<Vec<AbstractSection>>,
    /// Article types (e.g., "Clinical Trial", "Review", etc.)
    pub article_types: Vec<String>,
    /// MeSH headings associated with the article
    pub mesh_headings: Option<Vec<MeshHeading>>,
    /// Author-provided keywords
    pub keywords: Option<Vec<String>>,
    /// Chemical substances mentioned in the article
    pub chemical_list: Option<Vec<ChemicalConcept>>,
    /// Journal volume (e.g., "88")
    pub volume: Option<String>,
    /// Journal issue number (e.g., "3")
    pub issue: Option<String>,
    /// Page range (e.g., "123-130")
    pub pages: Option<String>,
    /// Article language (e.g., "eng", "jpn")
    pub language: Option<String>,
    /// ISO journal abbreviation (e.g., "J Biol Chem")
    pub journal_abbreviation: Option<String>,
    /// ISSN (International Standard Serial Number)
    pub issn: Option<String>,
}

/// Database information from EInfo API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseInfo {
    /// Database name (e.g., "pubmed", "pmc")
    pub name: String,
    /// Human-readable menu name
    pub menu_name: String,
    /// Database description
    pub description: String,
    /// Database build version
    pub build: Option<String>,
    /// Number of records in database
    pub count: Option<u64>,
    /// Last update timestamp
    pub last_update: Option<String>,
    /// Available search fields
    pub fields: Vec<FieldInfo>,
    /// Available links to other databases
    pub links: Vec<LinkInfo>,
}

/// Information about a database search field
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FieldInfo {
    /// Short field name (e.g., "titl", "auth")
    pub name: String,
    /// Full field name (e.g., "Title", "Author")
    pub full_name: String,
    /// Field description
    pub description: String,
    /// Number of indexed terms
    pub term_count: Option<u64>,
    /// Whether field contains dates
    pub is_date: bool,
    /// Whether field contains numerical values
    pub is_numerical: bool,
    /// Whether field uses single token indexing
    pub single_token: bool,
    /// Whether field uses hierarchical indexing
    pub hierarchy: bool,
    /// Whether field is hidden from users
    pub is_hidden: bool,
}

/// Information about database links
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkInfo {
    /// Link name
    pub name: String,
    /// Menu display name
    pub menu: String,
    /// Link description
    pub description: String,
    /// Target database
    pub target_db: String,
}

/// Results from ELink API for related article discovery
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RelatedArticles {
    /// Source PMIDs that were queried
    pub source_pmids: Vec<u32>,
    /// Related article PMIDs found
    pub related_pmids: Vec<u32>,
    /// Link type (e.g., "pubmed_pubmed", "pubmed_pubmed_reviews")
    pub link_type: String,
}

/// PMC links discovered through ELink API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PmcLinks {
    /// Source PMIDs that were queried
    pub source_pmids: Vec<u32>,
    /// PMC IDs that have full text available
    pub pmc_ids: Vec<String>,
}

/// Citation information from ELink API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Citations {
    /// Source PMIDs that were queried
    pub source_pmids: Vec<u32>,
    /// PMIDs of articles that cite the source articles
    pub citing_pmids: Vec<u32>,
    /// Link type (e.g., "pubmed_pubmed_citedin")
    pub link_type: String,
}

/// Search result with WebEnv session information for history server pagination
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// List of PMIDs matching the search query
    pub pmids: Vec<String>,
    /// Total number of results matching the query
    pub total_count: usize,
    /// WebEnv session identifier for history server
    pub webenv: Option<String>,
    /// Query key for history server
    pub query_key: Option<String>,
    /// How PubMed interpreted and translated the search query
    ///
    /// For example, searching "asthma" might be translated to:
    /// `"asthma"[MeSH Terms] OR "asthma"[All Fields]`
    ///
    /// This is useful for debugging search queries and understanding
    /// how PubMed's automatic term mapping works.
    pub query_translation: Option<String>,
}

impl SearchResult {
    /// Get the history session if WebEnv and query_key are available
    ///
    /// Returns `Some(HistorySession)` if both webenv and query_key are present,
    /// `None` otherwise.
    pub fn history_session(&self) -> Option<HistorySession> {
        match (&self.webenv, &self.query_key) {
            (Some(webenv), Some(query_key)) => Some(HistorySession {
                webenv: webenv.clone(),
                query_key: query_key.clone(),
            }),
            _ => None,
        }
    }

    /// Check if this result has history session information
    pub fn has_history(&self) -> bool {
        self.webenv.is_some() && self.query_key.is_some()
    }
}

/// History server session information for paginated fetching
///
/// This represents a session on NCBI's history server that can be used
/// to efficiently fetch large result sets in batches without re-running
/// the search query.
///
/// # Note
///
/// WebEnv sessions typically expire after 1 hour of inactivity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistorySession {
    /// WebEnv session identifier
    pub webenv: String,
    /// Query key within the session
    pub query_key: String,
}

/// Result from EPost API for uploading PMIDs to the NCBI History server
///
/// EPost stores a list of UIDs (PMIDs) on the History server and returns
/// WebEnv/query_key identifiers. These can then be used with `fetch_from_history()`
/// to retrieve article metadata, or combined with other E-utility calls.
///
/// # Example
///
/// ```ignore
/// use pubmed_client::PubMedClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = PubMedClient::new();
///
///     // Upload a list of PMIDs to the history server
///     let result = client.epost(&["31978945", "33515491", "25760099"]).await?;
///
///     // Use the session to fetch articles
///     let session = result.history_session();
///     let articles = client.fetch_from_history(&session, 0, 100).await?;
///     println!("Fetched {} articles", articles.len());
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct EPostResult {
    /// WebEnv session identifier for the uploaded IDs
    pub webenv: String,
    /// Query key for the uploaded IDs within the session
    pub query_key: String,
}

impl EPostResult {
    /// Convert to a HistorySession for use with `fetch_from_history()`
    ///
    /// This is a convenience method that creates a `HistorySession` from the
    /// EPost result, which can then be passed to `fetch_from_history()`.
    pub fn history_session(&self) -> HistorySession {
        HistorySession {
            webenv: self.webenv.clone(),
            query_key: self.query_key.clone(),
        }
    }
}

/// Medical Subject Heading (MeSH) qualifier/subheading
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeshQualifier {
    /// Qualifier name (e.g., "drug therapy", "genetics")
    pub qualifier_name: String,
    /// Unique identifier for the qualifier
    pub qualifier_ui: String,
    /// Whether this qualifier is a major topic
    pub major_topic: bool,
}

/// Medical Subject Heading (MeSH) descriptor term
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeshTerm {
    /// Descriptor name (e.g., "Diabetes Mellitus, Type 2")
    pub descriptor_name: String,
    /// Unique identifier for the descriptor
    pub descriptor_ui: String,
    /// Whether this term is a major topic of the article
    pub major_topic: bool,
    /// Associated qualifiers/subheadings
    pub qualifiers: Vec<MeshQualifier>,
}

/// Supplemental MeSH concept (for substances, diseases, etc.)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SupplementalConcept {
    /// Concept name
    pub name: String,
    /// Unique identifier
    pub ui: String,
    /// Concept type (e.g., "Disease", "Drug")
    pub concept_type: Option<String>,
}

/// Chemical substance mentioned in the article
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChemicalConcept {
    /// Chemical name
    pub name: String,
    /// Registry number (e.g., CAS number)
    pub registry_number: Option<String>,
    /// Chemical UI
    pub ui: Option<String>,
}

/// Complete MeSH heading information for an article
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MeshHeading {
    /// MeSH descriptor terms
    pub mesh_terms: Vec<MeshTerm>,
    /// Supplemental concepts
    pub supplemental_concepts: Vec<SupplementalConcept>,
}

impl PubMedArticle {
    /// Get all major MeSH terms from the article
    ///
    /// # Returns
    ///
    /// A vector of major MeSH term names
    ///
    /// # Example
    ///
    /// ```
    /// # use pubmed_parser::pubmed::PubMedArticle;
    /// # let article = PubMedArticle {
    /// #     pmid: "123".to_string(),
    /// #     title: "Test".to_string(),
    /// #     authors: vec![],
    /// #     author_count: 0,
    /// #     journal: "Test Journal".to_string(),
    /// #     pub_date: "2023".to_string(),
    /// #     doi: None,
    /// #     pmc_id: None,
    /// #     abstract_text: None,
    /// #     structured_abstract: None,
    /// #     article_types: vec![],
    /// #     mesh_headings: None,
    /// #     keywords: None,
    /// #     chemical_list: None,
    /// #     volume: None, issue: None, pages: None,
    /// #     language: None, journal_abbreviation: None, issn: None,
    /// # };
    /// let major_terms = article.get_major_mesh_terms();
    /// ```
    pub fn get_major_mesh_terms(&self) -> Vec<String> {
        let mut major_terms = Vec::new();

        if let Some(mesh_headings) = &self.mesh_headings {
            for heading in mesh_headings {
                for term in &heading.mesh_terms {
                    if term.major_topic {
                        major_terms.push(term.descriptor_name.clone());
                    }
                }
            }
        }

        major_terms
    }

    /// Check if the article has a specific MeSH term
    ///
    /// # Arguments
    ///
    /// * `term` - The MeSH term to check for
    ///
    /// # Returns
    ///
    /// `true` if the article has the specified MeSH term, `false` otherwise
    ///
    /// # Example
    ///
    /// ```
    /// # use pubmed_parser::pubmed::PubMedArticle;
    /// # let article = PubMedArticle {
    /// #     pmid: "123".to_string(),
    /// #     title: "Test".to_string(),
    /// #     authors: vec![],
    /// #     author_count: 0,
    /// #     journal: "Test Journal".to_string(),
    /// #     pub_date: "2023".to_string(),
    /// #     doi: None,
    /// #     pmc_id: None,
    /// #     abstract_text: None,
    /// #     structured_abstract: None,
    /// #     article_types: vec![],
    /// #     mesh_headings: None,
    /// #     keywords: None,
    /// #     chemical_list: None,
    /// #     volume: None, issue: None, pages: None,
    /// #     language: None, journal_abbreviation: None, issn: None,
    /// # };
    /// let has_diabetes = article.has_mesh_term("Diabetes Mellitus");
    /// ```
    pub fn has_mesh_term(&self, term: &str) -> bool {
        if let Some(mesh_headings) = &self.mesh_headings {
            for heading in mesh_headings {
                for mesh_term in &heading.mesh_terms {
                    if mesh_term.descriptor_name.eq_ignore_ascii_case(term) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Get all MeSH terms from the article
    ///
    /// # Returns
    ///
    /// A vector of all MeSH term names
    pub fn get_all_mesh_terms(&self) -> Vec<String> {
        let mut terms = Vec::new();

        if let Some(mesh_headings) = &self.mesh_headings {
            for heading in mesh_headings {
                for term in &heading.mesh_terms {
                    terms.push(term.descriptor_name.clone());
                }
            }
        }

        terms
    }

    /// Get corresponding authors from the article
    ///
    /// # Returns
    ///
    /// A vector of references to authors marked as corresponding
    pub fn get_corresponding_authors(&self) -> Vec<&Author> {
        self.authors
            .iter()
            .filter(|author| author.is_corresponding)
            .collect()
    }

    /// Get authors affiliated with a specific institution
    ///
    /// # Arguments
    ///
    /// * `institution` - Institution name to search for (case-insensitive substring match)
    ///
    /// # Returns
    ///
    /// A vector of references to authors with matching affiliations
    pub fn get_authors_by_institution(&self, institution: &str) -> Vec<&Author> {
        let institution_lower = institution.to_lowercase();
        self.authors
            .iter()
            .filter(|author| {
                author.affiliations.iter().any(|affil| {
                    affil
                        .institution
                        .as_ref()
                        .is_some_and(|inst| inst.to_lowercase().contains(&institution_lower))
                })
            })
            .collect()
    }

    /// Get all unique countries from author affiliations
    ///
    /// # Returns
    ///
    /// A vector of unique country names
    pub fn get_author_countries(&self) -> Vec<String> {
        use std::collections::HashSet;
        let mut countries: HashSet<String> = HashSet::new();

        for author in &self.authors {
            for affiliation in &author.affiliations {
                if let Some(country) = &affiliation.country {
                    countries.insert(country.clone());
                }
            }
        }

        countries.into_iter().collect()
    }

    /// Get authors with ORCID identifiers
    ///
    /// # Returns
    ///
    /// A vector of references to authors who have ORCID IDs
    pub fn get_authors_with_orcid(&self) -> Vec<&Author> {
        self.authors
            .iter()
            .filter(|author| author.orcid.is_some())
            .collect()
    }

    /// Check if the article has international collaboration
    ///
    /// # Returns
    ///
    /// `true` if authors are from multiple countries
    pub fn has_international_collaboration(&self) -> bool {
        self.get_author_countries().len() > 1
    }

    /// Calculate MeSH term similarity between two articles
    ///
    /// # Arguments
    ///
    /// * `other` - The other article to compare with
    ///
    /// # Returns
    ///
    /// A similarity score between 0.0 and 1.0 based on Jaccard similarity
    ///
    /// # Example
    ///
    /// ```
    /// # use pubmed_parser::pubmed::PubMedArticle;
    /// # let article1 = PubMedArticle {
    /// #     pmid: "123".to_string(),
    /// #     title: "Test".to_string(),
    /// #     authors: vec![],
    /// #     author_count: 0,
    /// #     journal: "Test Journal".to_string(),
    /// #     pub_date: "2023".to_string(),
    /// #     doi: None,
    /// #     pmc_id: None,
    /// #     abstract_text: None,
    /// #     structured_abstract: None,
    /// #     article_types: vec![],
    /// #     mesh_headings: None,
    /// #     keywords: None,
    /// #     chemical_list: None,
    /// #     volume: None, issue: None, pages: None,
    /// #     language: None, journal_abbreviation: None, issn: None,
    /// # };
    /// # let article2 = article1.clone();
    /// let similarity = article1.mesh_term_similarity(&article2);
    /// ```
    pub fn mesh_term_similarity(&self, other: &PubMedArticle) -> f64 {
        use std::collections::HashSet;

        let terms1: HashSet<String> = self
            .get_all_mesh_terms()
            .into_iter()
            .map(|t| t.to_lowercase())
            .collect();

        let terms2: HashSet<String> = other
            .get_all_mesh_terms()
            .into_iter()
            .map(|t| t.to_lowercase())
            .collect();

        if terms1.is_empty() && terms2.is_empty() {
            return 0.0;
        }

        let intersection = terms1.intersection(&terms2).count();
        let union = terms1.union(&terms2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Get MeSH qualifiers for a specific term
    ///
    /// # Arguments
    ///
    /// * `term` - The MeSH term to get qualifiers for
    ///
    /// # Returns
    ///
    /// A vector of qualifier names for the specified term
    pub fn get_mesh_qualifiers(&self, term: &str) -> Vec<String> {
        let mut qualifiers = Vec::new();

        if let Some(mesh_headings) = &self.mesh_headings {
            for heading in mesh_headings {
                for mesh_term in &heading.mesh_terms {
                    if mesh_term.descriptor_name.eq_ignore_ascii_case(term) {
                        for qualifier in &mesh_term.qualifiers {
                            qualifiers.push(qualifier.qualifier_name.clone());
                        }
                    }
                }
            }
        }

        qualifiers
    }

    /// Check if the article has any MeSH terms
    ///
    /// # Returns
    ///
    /// `true` if the article has MeSH terms, `false` otherwise
    pub fn has_mesh_terms(&self) -> bool {
        self.mesh_headings
            .as_ref()
            .map(|h| !h.is_empty())
            .unwrap_or(false)
    }

    /// Get chemicals mentioned in the article
    ///
    /// # Returns
    ///
    /// A vector of chemical names
    pub fn get_chemical_names(&self) -> Vec<String> {
        self.chemical_list
            .as_ref()
            .map(|chemicals| chemicals.iter().map(|c| c.name.clone()).collect())
            .unwrap_or_default()
    }
}

// ================================================================================================
// ESpell API types
// ================================================================================================

/// Represents a segment of the spelled query from the ESpell API
///
/// Each segment is either an original (unchanged) part of the query or a
/// replacement (corrected spelling suggestion).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum SpelledQuerySegment {
    /// A term or separator that was not changed
    Original(String),
    /// A corrected/suggested replacement term
    Replaced(String),
}

/// Result from the ESpell API providing spelling suggestions
///
/// ESpell provides spelling suggestions for terms within a single text query
/// in a given database. It acts as a preprocessing/spell-check tool to improve
/// search accuracy before executing actual searches.
///
/// # Example
///
/// ```ignore
/// use pubmed_client::PubMedClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = PubMedClient::new();
///     let result = client.spell_check("asthmaa OR alergies").await?;
///     println!("Original: {}", result.query);
///     println!("Corrected: {}", result.corrected_query);
///     Ok(())
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpellCheckResult {
    /// The database that was queried
    pub database: String,
    /// The original query string as submitted
    pub query: String,
    /// The full corrected/suggested query as a plain string
    pub corrected_query: String,
    /// Detailed segments showing which terms were replaced vs. kept
    pub spelled_query: Vec<SpelledQuerySegment>,
}

impl SpellCheckResult {
    /// Check if the query had any spelling corrections.
    ///
    /// Returns `true` only when NCBI provided a non-empty corrected query that differs from
    /// the original. The NCBI ESpell API returns an empty `<CorrectedQuery/>` element when
    /// no corrections are available.
    pub fn has_corrections(&self) -> bool {
        !self.corrected_query.is_empty() && self.query != self.corrected_query
    }

    /// Get only the replaced (corrected) terms
    pub fn replacements(&self) -> Vec<&str> {
        self.spelled_query
            .iter()
            .filter_map(|segment| match segment {
                SpelledQuerySegment::Replaced(s) => Some(s.as_str()),
                SpelledQuerySegment::Original(_) => None,
            })
            .collect()
    }
}

// ================================================================================================
// ECitMatch API types
// ================================================================================================

/// Input for a single citation match query
///
/// Used with the ECitMatch API to find PMIDs from citation information.
/// Each field corresponds to a part of the citation string sent to the API.
///
/// # Example
///
/// ```
/// use pubmed_parser::pubmed::CitationQuery;
///
/// let query = CitationQuery::new(
///     "proc natl acad sci u s a",
///     "1991",
///     "88",
///     "3248",
///     "mann bj",
///     "Art1",
/// );
/// ```
#[derive(Debug, Clone)]
pub struct CitationQuery {
    /// Journal title abbreviation (e.g., "proc natl acad sci u s a")
    pub journal: String,
    /// Publication year (e.g., "1991")
    pub year: String,
    /// Volume number (e.g., "88")
    pub volume: String,
    /// First page number (e.g., "3248")
    pub first_page: String,
    /// Author name (e.g., "mann bj")
    pub author_name: String,
    /// User-defined key for identifying results (e.g., "Art1")
    pub key: String,
}

impl CitationQuery {
    /// Create a new citation query
    pub fn new(
        journal: &str,
        year: &str,
        volume: &str,
        first_page: &str,
        author_name: &str,
        key: &str,
    ) -> Self {
        Self {
            journal: journal.to_string(),
            year: year.to_string(),
            volume: volume.to_string(),
            first_page: first_page.to_string(),
            author_name: author_name.to_string(),
            key: key.to_string(),
        }
    }

    /// Format this citation as a bdata string for the ECitMatch API
    pub fn to_bdata(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|",
            self.journal.replace(' ', "+"),
            self.year,
            self.volume,
            self.first_page,
            self.author_name.replace(' ', "+"),
            self.key,
        )
    }
}

/// Status of a citation match result
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum CitationMatchStatus {
    /// A unique PMID was found for the citation
    Found,
    /// No PMID could be found for the citation
    NotFound,
    /// Multiple PMIDs matched the citation (ambiguous)
    Ambiguous,
}

/// Result of a single citation match from the ECitMatch API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CitationMatch {
    /// Journal title from the query
    pub journal: String,
    /// Year from the query
    pub year: String,
    /// Volume from the query
    pub volume: String,
    /// First page from the query
    pub first_page: String,
    /// Author name from the query
    pub author_name: String,
    /// User-defined key from the query
    pub key: String,
    /// Matched PMID (if found)
    pub pmid: Option<String>,
    /// Match status
    pub status: CitationMatchStatus,
}

/// Results from ECitMatch API for batch citation matching
///
/// Contains the results of matching multiple citations to PMIDs.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CitationMatches {
    /// List of citation match results
    pub matches: Vec<CitationMatch>,
}

impl CitationMatches {
    /// Get only the successfully matched citations
    pub fn found(&self) -> Vec<&CitationMatch> {
        self.matches
            .iter()
            .filter(|m| m.status == CitationMatchStatus::Found)
            .collect()
    }

    /// Get the number of successful matches
    pub fn found_count(&self) -> usize {
        self.matches
            .iter()
            .filter(|m| m.status == CitationMatchStatus::Found)
            .count()
    }
}

// ================================================================================================
// EGQuery API types
// ================================================================================================

/// Record count for a single NCBI database from the EGQuery API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DatabaseCount {
    /// Internal database name (e.g., "pubmed", "pmc", "nuccore")
    pub db_name: String,
    /// Human-readable database name (e.g., "PubMed", "PMC", "Nucleotide")
    pub menu_name: String,
    /// Number of records matching the query in this database
    pub count: u64,
    /// Status of the query for this database (e.g., "Ok")
    pub status: String,
}

/// Results from EGQuery API for global database search
///
/// Contains the number of records matching a query across all NCBI Entrez databases.
///
/// # Example
///
/// ```ignore
/// use pubmed_client::PubMedClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = PubMedClient::new();
///     let results = client.global_query("asthma").await?;
///     for db in &results.results {
///         if db.count > 0 {
///             println!("{}: {} records", db.menu_name, db.count);
///         }
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalQueryResults {
    /// The query term that was searched
    pub term: String,
    /// Results for each NCBI database
    pub results: Vec<DatabaseCount>,
}

impl GlobalQueryResults {
    /// Get results for databases with matching records (count > 0)
    pub fn non_zero(&self) -> Vec<&DatabaseCount> {
        self.results.iter().filter(|r| r.count > 0).collect()
    }

    /// Get the count for a specific database
    pub fn count_for(&self, db_name: &str) -> Option<u64> {
        self.results
            .iter()
            .find(|r| r.db_name == db_name)
            .map(|r| r.count)
    }
}

// ================================================================================================
// ESummary API types
// ================================================================================================

/// Lightweight article summary from the ESummary API
///
/// Contains basic metadata (title, authors, journal, dates) without the full
/// abstract, MeSH terms, or chemical lists that EFetch provides. Use this when
/// you only need basic bibliographic information for a large number of articles.
///
/// # Example
///
/// ```ignore
/// use pubmed_client::PubMedClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = PubMedClient::new();
///     let summaries = client.fetch_summaries(&["31978945", "33515491"]).await?;
///     for summary in &summaries {
///         println!("{}: {} ({})", summary.pmid, summary.title, summary.pub_date);
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleSummary {
    /// PubMed ID
    pub pmid: String,
    /// Article title
    pub title: String,
    /// Author names (e.g., ["Zhu N", "Zhang D", "Wang W"])
    pub authors: Vec<String>,
    /// Journal name (source field)
    pub journal: String,
    /// Full journal name (e.g., "The New England journal of medicine")
    pub full_journal_name: String,
    /// Publication date (e.g., "2020 Feb")
    pub pub_date: String,
    /// Electronic publication date (e.g., "2020 Jan 24")
    pub epub_date: String,
    /// DOI (Digital Object Identifier)
    pub doi: Option<String>,
    /// PMC ID if available (e.g., "PMC7092803")
    pub pmc_id: Option<String>,
    /// Journal volume (e.g., "382")
    pub volume: String,
    /// Journal issue (e.g., "8")
    pub issue: String,
    /// Page range (e.g., "727-733")
    pub pages: String,
    /// Languages (e.g., ["eng"])
    pub languages: Vec<String>,
    /// Publication types (e.g., ["Journal Article", "Review"])
    pub pub_types: Vec<String>,
    /// ISSN
    pub issn: String,
    /// Electronic ISSN
    pub essn: String,
    /// Sorted publication date (e.g., "2020/02/20 00:00")
    pub sort_pub_date: String,
    /// PMC reference count (number of citing articles in PMC)
    pub pmc_ref_count: u64,
    /// Record status (e.g., "PubMed - indexed for MEDLINE")
    pub record_status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::format_author_name;

    fn create_test_author() -> Author {
        Author {
            surname: Some("Doe".to_string()),
            given_names: Some("John A".to_string()),
            initials: Some("JA".to_string()),
            suffix: None,
            full_name: "John A Doe".to_string(),
            affiliations: vec![
                Affiliation {
                    id: None,
                    institution: Some("Harvard Medical School".to_string()),
                    department: Some("Department of Medicine".to_string()),
                    address: Some("Boston, MA".to_string()),
                    country: Some("USA".to_string()),
                },
                Affiliation {
                    id: None,
                    institution: Some("Massachusetts General Hospital".to_string()),
                    department: None,
                    address: Some("Boston, MA".to_string()),
                    country: Some("USA".to_string()),
                },
            ],
            orcid: Some("0000-0001-2345-6789".to_string()),
            email: Some("john.doe@hms.harvard.edu".to_string()),
            is_corresponding: true,
            roles: vec![
                "Conceptualization".to_string(),
                "Writing - original draft".to_string(),
            ],
        }
    }

    fn create_test_article_with_mesh() -> PubMedArticle {
        PubMedArticle {
            pmid: "12345".to_string(),
            title: "Test Article".to_string(),
            authors: vec![create_test_author()],
            author_count: 1,
            journal: "Test Journal".to_string(),
            pub_date: "2023".to_string(),
            doi: None,
            pmc_id: None,
            abstract_text: None,
            structured_abstract: None,
            article_types: vec![],
            mesh_headings: Some(vec![
                MeshHeading {
                    mesh_terms: vec![MeshTerm {
                        descriptor_name: "Diabetes Mellitus, Type 2".to_string(),
                        descriptor_ui: "D003924".to_string(),
                        major_topic: true,
                        qualifiers: vec![
                            MeshQualifier {
                                qualifier_name: "drug therapy".to_string(),
                                qualifier_ui: "Q000188".to_string(),
                                major_topic: false,
                            },
                            MeshQualifier {
                                qualifier_name: "genetics".to_string(),
                                qualifier_ui: "Q000235".to_string(),
                                major_topic: true,
                            },
                        ],
                    }],
                    supplemental_concepts: vec![],
                },
                MeshHeading {
                    mesh_terms: vec![MeshTerm {
                        descriptor_name: "Hypertension".to_string(),
                        descriptor_ui: "D006973".to_string(),
                        major_topic: false,
                        qualifiers: vec![],
                    }],
                    supplemental_concepts: vec![],
                },
            ]),
            keywords: Some(vec!["diabetes".to_string(), "treatment".to_string()]),
            chemical_list: Some(vec![ChemicalConcept {
                name: "Metformin".to_string(),
                registry_number: Some("657-24-9".to_string()),
                ui: Some("D008687".to_string()),
            }]),
            volume: Some("45".to_string()),
            issue: Some("3".to_string()),
            pages: Some("123-130".to_string()),
            language: Some("eng".to_string()),
            journal_abbreviation: Some("Test J".to_string()),
            issn: Some("1234-5678".to_string()),
        }
    }

    #[test]
    fn test_get_major_mesh_terms() {
        let article = create_test_article_with_mesh();
        let major_terms = article.get_major_mesh_terms();

        assert_eq!(major_terms.len(), 1);
        assert_eq!(major_terms[0], "Diabetes Mellitus, Type 2");
    }

    #[test]
    fn test_has_mesh_term() {
        let article = create_test_article_with_mesh();

        assert!(article.has_mesh_term("Diabetes Mellitus, Type 2"));
        assert!(article.has_mesh_term("DIABETES MELLITUS, TYPE 2")); // Case insensitive
        assert!(article.has_mesh_term("Hypertension"));
        assert!(!article.has_mesh_term("Cancer"));
    }

    #[test]
    fn test_get_all_mesh_terms() {
        let article = create_test_article_with_mesh();
        let all_terms = article.get_all_mesh_terms();

        assert_eq!(all_terms.len(), 2);
        assert!(all_terms.contains(&"Diabetes Mellitus, Type 2".to_string()));
        assert!(all_terms.contains(&"Hypertension".to_string()));
    }

    #[test]
    fn test_mesh_term_similarity() {
        let article1 = create_test_article_with_mesh();
        let mut article2 = create_test_article_with_mesh();

        // Same article should have similarity of 1.0
        let similarity = article1.mesh_term_similarity(&article2);
        assert_eq!(similarity, 1.0);

        // Different MeSH terms
        article2.mesh_headings = Some(vec![MeshHeading {
            mesh_terms: vec![
                MeshTerm {
                    descriptor_name: "Diabetes Mellitus, Type 2".to_string(),
                    descriptor_ui: "D003924".to_string(),
                    major_topic: true,
                    qualifiers: vec![],
                },
                MeshTerm {
                    descriptor_name: "Obesity".to_string(),
                    descriptor_ui: "D009765".to_string(),
                    major_topic: false,
                    qualifiers: vec![],
                },
            ],
            supplemental_concepts: vec![],
        }]);

        let similarity = article1.mesh_term_similarity(&article2);
        // Should have partial similarity (1 common term out of 3 unique terms)
        assert!(similarity > 0.0 && similarity < 1.0);
        assert_eq!(similarity, 1.0 / 3.0); // Jaccard similarity

        // No MeSH terms
        let article3 = PubMedArticle {
            pmid: "54321".to_string(),
            title: "Test".to_string(),
            authors: vec![],
            author_count: 0,
            journal: "Test".to_string(),
            pub_date: "2023".to_string(),
            doi: None,
            pmc_id: None,
            abstract_text: None,
            structured_abstract: None,
            article_types: vec![],
            mesh_headings: None,
            keywords: None,
            chemical_list: None,
            volume: None,
            issue: None,
            pages: None,
            language: None,
            journal_abbreviation: None,
            issn: None,
        };

        assert_eq!(article1.mesh_term_similarity(&article3), 0.0);
    }

    #[test]
    fn test_get_mesh_qualifiers() {
        let article = create_test_article_with_mesh();

        let qualifiers = article.get_mesh_qualifiers("Diabetes Mellitus, Type 2");
        assert_eq!(qualifiers.len(), 2);
        assert!(qualifiers.contains(&"drug therapy".to_string()));
        assert!(qualifiers.contains(&"genetics".to_string()));

        let qualifiers = article.get_mesh_qualifiers("Hypertension");
        assert_eq!(qualifiers.len(), 0);

        let qualifiers = article.get_mesh_qualifiers("Nonexistent Term");
        assert_eq!(qualifiers.len(), 0);
    }

    #[test]
    fn test_has_mesh_terms() {
        let article = create_test_article_with_mesh();
        assert!(article.has_mesh_terms());

        let mut article_no_mesh = article.clone();
        article_no_mesh.mesh_headings = None;
        assert!(!article_no_mesh.has_mesh_terms());

        let mut article_empty_mesh = article.clone();
        article_empty_mesh.mesh_headings = Some(vec![]);
        assert!(!article_empty_mesh.has_mesh_terms());
    }

    #[test]
    fn test_get_chemical_names() {
        let article = create_test_article_with_mesh();
        let chemicals = article.get_chemical_names();

        assert_eq!(chemicals.len(), 1);
        assert_eq!(chemicals[0], "Metformin");

        let mut article_no_chemicals = article.clone();
        article_no_chemicals.chemical_list = None;
        let chemicals = article_no_chemicals.get_chemical_names();
        assert_eq!(chemicals.len(), 0);
    }

    #[test]
    fn test_author_creation() {
        let author = Author::new(Some("Smith".to_string()), Some("Jane".to_string()));
        assert_eq!(author.surname, Some("Smith".to_string()));
        assert_eq!(author.given_names, Some("Jane".to_string()));
        assert_eq!(author.full_name, "Jane Smith");
        assert!(!author.has_orcid());
        assert!(!author.is_corresponding);
    }

    #[test]
    fn test_author_affiliations() {
        let author = create_test_author();

        assert!(author.is_affiliated_with("Harvard"));
        assert!(author.is_affiliated_with("Massachusetts General"));
        assert!(!author.is_affiliated_with("Stanford"));

        let primary = author.primary_affiliation().unwrap();
        assert_eq!(
            primary.institution,
            Some("Harvard Medical School".to_string())
        );

        assert!(author.has_orcid());
        assert!(author.is_corresponding);
    }

    #[test]
    fn test_get_corresponding_authors() {
        let article = create_test_article_with_mesh();
        let corresponding = article.get_corresponding_authors();

        assert_eq!(corresponding.len(), 1);
        assert_eq!(corresponding[0].full_name, "John A Doe");
    }

    #[test]
    fn test_get_authors_by_institution() {
        let article = create_test_article_with_mesh();

        let harvard_authors = article.get_authors_by_institution("Harvard");
        assert_eq!(harvard_authors.len(), 1);

        let stanford_authors = article.get_authors_by_institution("Stanford");
        assert_eq!(stanford_authors.len(), 0);
    }

    #[test]
    fn test_get_author_countries() {
        let article = create_test_article_with_mesh();
        let countries = article.get_author_countries();

        assert_eq!(countries.len(), 1);
        assert!(countries.contains(&"USA".to_string()));
    }

    #[test]
    fn test_international_collaboration() {
        let article = create_test_article_with_mesh();
        assert!(!article.has_international_collaboration());

        // Create article with international authors
        let mut international_article = article.clone();
        let mut uk_author = create_test_author();
        uk_author.affiliations[0].country = Some("UK".to_string());
        international_article.authors.push(uk_author);
        international_article.author_count = 2;

        assert!(international_article.has_international_collaboration());
    }

    #[test]
    fn test_get_authors_with_orcid() {
        let article = create_test_article_with_mesh();
        let authors_with_orcid = article.get_authors_with_orcid();

        assert_eq!(authors_with_orcid.len(), 1);
        assert_eq!(
            authors_with_orcid[0].orcid,
            Some("0000-0001-2345-6789".to_string())
        );
    }

    #[test]
    fn test_format_author_name() {
        assert_eq!(
            format_author_name(&Some("Smith".to_string()), &Some("John".to_string()), &None),
            "John Smith"
        );

        assert_eq!(
            format_author_name(&Some("Doe".to_string()), &None, &Some("J".to_string())),
            "J Doe"
        );

        assert_eq!(
            format_author_name(&Some("Johnson".to_string()), &None, &None),
            "Johnson"
        );

        assert_eq!(
            format_author_name(&None, &Some("Jane".to_string()), &None),
            "Jane"
        );

        assert_eq!(format_author_name(&None, &None, &None), "Unknown Author");
    }

    #[test]
    fn test_spell_check_result_has_corrections() {
        let result = SpellCheckResult {
            database: "pubmed".to_string(),
            query: "asthmaa".to_string(),
            corrected_query: "asthma".to_string(),
            spelled_query: vec![SpelledQuerySegment::Replaced("asthma".to_string())],
        };
        assert!(result.has_corrections());

        let no_correction = SpellCheckResult {
            database: "pubmed".to_string(),
            query: "asthma".to_string(),
            corrected_query: "asthma".to_string(),
            spelled_query: vec![SpelledQuerySegment::Original("asthma".to_string())],
        };
        assert!(!no_correction.has_corrections());
    }

    #[test]
    fn test_spell_check_result_replacements() {
        let result = SpellCheckResult {
            database: "pubmed".to_string(),
            query: "asthmaa OR alergies".to_string(),
            corrected_query: "asthma or allergies".to_string(),
            spelled_query: vec![
                SpelledQuerySegment::Original("".to_string()),
                SpelledQuerySegment::Replaced("asthma".to_string()),
                SpelledQuerySegment::Original(" OR ".to_string()),
                SpelledQuerySegment::Replaced("allergies".to_string()),
            ],
        };
        let replacements = result.replacements();
        assert_eq!(replacements.len(), 2);
        assert_eq!(replacements[0], "asthma");
        assert_eq!(replacements[1], "allergies");
    }

    #[test]
    fn test_bibliographic_fields_on_article() {
        let article = create_test_article_with_mesh();

        assert_eq!(article.volume, Some("45".to_string()));
        assert_eq!(article.issue, Some("3".to_string()));
        assert_eq!(article.pages, Some("123-130".to_string()));
        assert_eq!(article.language, Some("eng".to_string()));
        assert_eq!(article.journal_abbreviation, Some("Test J".to_string()));
        assert_eq!(article.issn, Some("1234-5678".to_string()));
    }
}
