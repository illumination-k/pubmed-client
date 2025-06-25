use serde::{Deserialize, Serialize};

/// Represents a PubMed article with metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PubMedArticle {
    /// PubMed ID
    pub pmid: String,
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
    /// Abstract text (if available)
    pub abstract_text: Option<String>,
    /// Article types (e.g., "Clinical Trial", "Review", etc.)
    pub article_types: Vec<String>,
    /// MeSH headings associated with the article
    pub mesh_headings: Option<Vec<MeshHeading>>,
    /// Author-provided keywords
    pub keywords: Option<Vec<String>>,
    /// Chemical substances mentioned in the article
    pub chemical_list: Option<Vec<ChemicalConcept>>,
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
    /// # use pubmed_client_rs::pubmed::PubMedArticle;
    /// # let article = PubMedArticle {
    /// #     pmid: "123".to_string(),
    /// #     title: "Test".to_string(),
    /// #     authors: vec![],
    /// #     journal: "Test Journal".to_string(),
    /// #     pub_date: "2023".to_string(),
    /// #     doi: None,
    /// #     abstract_text: None,
    /// #     article_types: vec![],
    /// #     mesh_headings: None,
    /// #     keywords: None,
    /// #     chemical_list: None,
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
    /// # use pubmed_client_rs::pubmed::PubMedArticle;
    /// # let article = PubMedArticle {
    /// #     pmid: "123".to_string(),
    /// #     title: "Test".to_string(),
    /// #     authors: vec![],
    /// #     journal: "Test Journal".to_string(),
    /// #     pub_date: "2023".to_string(),
    /// #     doi: None,
    /// #     abstract_text: None,
    /// #     article_types: vec![],
    /// #     mesh_headings: None,
    /// #     keywords: None,
    /// #     chemical_list: None,
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
    /// # use pubmed_client_rs::pubmed::PubMedArticle;
    /// # let article1 = PubMedArticle {
    /// #     pmid: "123".to_string(),
    /// #     title: "Test".to_string(),
    /// #     authors: vec![],
    /// #     journal: "Test Journal".to_string(),
    /// #     pub_date: "2023".to_string(),
    /// #     doi: None,
    /// #     abstract_text: None,
    /// #     article_types: vec![],
    /// #     mesh_headings: None,
    /// #     keywords: None,
    /// #     chemical_list: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_article_with_mesh() -> PubMedArticle {
        PubMedArticle {
            pmid: "12345".to_string(),
            title: "Test Article".to_string(),
            authors: vec!["John Doe".to_string()],
            journal: "Test Journal".to_string(),
            pub_date: "2023".to_string(),
            doi: None,
            abstract_text: None,
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
            journal: "Test".to_string(),
            pub_date: "2023".to_string(),
            doi: None,
            abstract_text: None,
            article_types: vec![],
            mesh_headings: None,
            keywords: None,
            chemical_list: None,
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
}
