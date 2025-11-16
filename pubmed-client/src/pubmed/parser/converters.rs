//! Conversion logic from XML types to public API models
//!
//! This module handles the transformation of internal XML deserialization types
//! into the clean public API models exposed by the library.

use super::extractors::{extract_country_from_text, extract_email_from_text, format_author_name};
use super::xml_types::*;
use crate::error::{PubMedError, Result};
use crate::pubmed::models::{
    Affiliation, Author, ChemicalConcept, MeshHeading, MeshQualifier, MeshTerm, PubMedArticle,
};
use tracing::debug;

impl PubmedArticleXml {
    /// Extract PMC ID from PubmedData ArticleIdList
    fn extract_pmc_id(pubmed_data: &Option<PubmedData>) -> Option<String> {
        pubmed_data
            .as_ref()?
            .article_id_list
            .as_ref()?
            .ids
            .iter()
            .find(|id| id.id_type.to_lowercase() == "pmc")
            .map(|id| id.value.clone())
    }

    /// Convert XML article to public API model
    pub(super) fn into_article(self, pmid: &str) -> Result<PubMedArticle> {
        let medline = self.medline_citation;
        let article = medline.article;

        // Extract PMC ID from PubmedData
        let pmc_id = Self::extract_pmc_id(&self.pubmed_data);

        // Extract title
        let title = article
            .article_title
            .ok_or_else(|| PubMedError::ArticleNotFound {
                pmid: pmid.to_string(),
            })?;

        // Extract authors
        let authors = article
            .author_list
            .map_or(Vec::new(), |list| list.into_authors());

        // Extract journal
        let journal = article
            .journal
            .as_ref()
            .and_then(|j| j.title.clone())
            .unwrap_or_default();

        // Extract publication date
        let pub_date = article.journal.as_ref().map_or(String::new(), |j| {
            j.journal_issue
                .as_ref()
                .and_then(|ji| ji.pub_date.as_ref())
                .map_or(String::new(), |pd| pd.to_string())
        });

        // Extract DOI
        let doi = article.elocation_ids.and_then(|ids| {
            ids.into_iter()
                .find(|id| id.eid_type.as_deref() == Some("doi"))
                .map(|id| id.value)
        });

        // Extract abstract
        let abstract_text = article.abstract_section.and_then(|s| s.to_string_opt());

        // Extract article types
        let article_types = article
            .publication_type_list
            .map_or(Vec::new(), |list| list.into_types());

        // Extract MeSH headings
        let mesh_headings = medline
            .mesh_heading_list
            .and_then(|list| list.into_headings());

        // Extract keywords
        let keywords = medline.keyword_list.and_then(|list| list.into_keywords());

        // Extract chemical list
        let chemical_list = medline.chemical_list.and_then(|list| list.into_chemicals());

        let author_count = authors.len() as u32;

        debug!(
            authors_parsed = authors.len(),
            has_abstract = abstract_text.is_some(),
            journal = %journal,
            mesh_terms_count = mesh_headings.as_ref().map_or(0, |h| h.len()),
            keywords_count = keywords.as_ref().map_or(0, |k| k.len()),
            chemicals_count = chemical_list.as_ref().map_or(0, |c| c.len()),
            "Completed XML parsing"
        );

        Ok(PubMedArticle {
            pmid: pmid.to_string(),
            title,
            authors,
            author_count,
            journal,
            pub_date,
            doi,
            pmc_id,
            abstract_text,
            article_types,
            mesh_headings,
            keywords,
            chemical_list,
        })
    }
}

impl AuthorList {
    /// Convert XML author list to public API author vector
    pub(super) fn into_authors(self) -> Vec<Author> {
        self.authors
            .unwrap_or_default()
            .into_iter()
            .filter_map(|a| a.into_author())
            .collect()
    }
}

impl AuthorXml {
    /// Convert XML author to public API author model
    pub(super) fn into_author(self) -> Option<Author> {
        // Handle collective names
        if let Some(collective_name) = self.collective_name {
            return Some(Author {
                surname: None,
                given_names: None,
                initials: None,
                suffix: None,
                full_name: collective_name,
                affiliations: Vec::new(),
                orcid: None,
                email: None,
                is_corresponding: false,
                roles: Vec::new(),
            });
        }

        let full_name = format_author_name(&self.last_name, &self.fore_name, &self.initials);

        if full_name.trim().is_empty() || full_name == "Unknown Author" {
            None
        } else {
            // Extract affiliations and email from affiliation info
            let (affiliations, email) = self
                .affiliation_info
                .unwrap_or_default()
                .into_iter()
                .filter_map(|info| info.affiliation)
                .fold((Vec::new(), None), |(mut affs, mut email_acc), text| {
                    let email = extract_email_from_text(&text);
                    let country = extract_country_from_text(&text);

                    // Use first email found
                    if email_acc.is_none() && email.is_some() {
                        email_acc = email.clone();
                    }

                    affs.push(Affiliation {
                        id: None,
                        institution: if text.is_empty() {
                            None
                        } else {
                            Some(text.to_string())
                        },
                        department: None,
                        address: None,
                        country,
                    });
                    (affs, email_acc)
                });

            let orcid = self.identifiers.and_then(|ids| {
                ids.into_iter()
                    .find(|id| id.source.as_deref() == Some("ORCID"))
                    .map(|id| id.value)
            });

            Some(Author {
                surname: self.last_name,
                given_names: self.fore_name,
                initials: self.initials,
                suffix: self.suffix,
                full_name,
                affiliations,
                orcid,
                email,
                is_corresponding: false,
                roles: Vec::new(),
            })
        }
    }
}

impl PublicationTypeList {
    /// Convert XML publication type list to string vector
    pub(super) fn into_types(self) -> Vec<String> {
        self.publication_types
            .unwrap_or_default()
            .into_iter()
            .map(|pt| pt.to_string())
            .collect()
    }
}

impl MeshHeadingList {
    /// Convert XML MeSH heading list to public API model
    pub(super) fn into_headings(self) -> Option<Vec<MeshHeading>> {
        self.mesh_headings.and_then(|headings| {
            let result: Vec<MeshHeading> = headings
                .into_iter()
                .filter_map(|h| h.into_heading())
                .collect();
            if result.is_empty() {
                None
            } else {
                Some(result)
            }
        })
    }
}

impl MeshHeadingXml {
    /// Convert XML MeSH heading to public API model
    pub(super) fn into_heading(self) -> Option<MeshHeading> {
        self.descriptor_name.map(|descriptor| {
            let qualifiers = self
                .qualifier_names
                .unwrap_or_default()
                .into_iter()
                .map(|q| q.into_qualifier())
                .collect();

            MeshHeading {
                mesh_terms: vec![MeshTerm {
                    descriptor_name: descriptor.text,
                    descriptor_ui: descriptor.ui.unwrap_or_default(),
                    major_topic: descriptor.major_topic_yn,
                    qualifiers,
                }],
                supplemental_concepts: Vec::new(),
            }
        })
    }
}

impl QualifierName {
    /// Convert XML qualifier to public API model
    pub(super) fn into_qualifier(self) -> MeshQualifier {
        MeshQualifier {
            qualifier_name: self.text,
            qualifier_ui: self.ui.unwrap_or_default(),
            major_topic: self.major_topic_yn,
        }
    }
}

impl ChemicalList {
    /// Convert XML chemical list to public API model
    pub(super) fn into_chemicals(self) -> Option<Vec<ChemicalConcept>> {
        self.chemicals.and_then(|chemicals| {
            let result: Vec<ChemicalConcept> = chemicals
                .into_iter()
                .filter_map(|c| c.into_chemical())
                .collect();
            if result.is_empty() {
                None
            } else {
                Some(result)
            }
        })
    }
}

impl ChemicalXml {
    /// Convert XML chemical to public API model
    pub(super) fn into_chemical(self) -> Option<ChemicalConcept> {
        self.name_of_substance.map(|name| ChemicalConcept {
            name: name.text,
            registry_number: self.registry_number.filter(|r| !r.is_empty() && r != "0"),
            ui: name.ui,
        })
    }
}

impl KeywordList {
    /// Convert XML keyword list to string vector
    pub(super) fn into_keywords(self) -> Option<Vec<String>> {
        self.keywords.and_then(|keywords| {
            let result: Vec<String> = keywords.into_iter().map(|k| k.to_string()).collect();
            if result.is_empty() {
                None
            } else {
                Some(result)
            }
        })
    }
}
