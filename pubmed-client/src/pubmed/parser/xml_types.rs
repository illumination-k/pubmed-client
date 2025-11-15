//! Internal XML deserialization types for PubMed XML schema
//!
//! This module contains serde-compatible structs that map directly to the PubMed XML schema.
//! These types are internal implementation details and should not be exposed in the public API.
//!
//! The structs in this module use quick-xml's serde deserializer with field renaming
//! to match the XML element and attribute names.

use super::deserializers::{deserialize_abstract_text, deserialize_bool_yn};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::result;

/// Root element for PubMed article sets
#[derive(Debug, Deserialize)]
#[serde(rename = "PubmedArticleSet")]
pub(super) struct PubmedArticleSet {
    #[serde(rename = "PubmedArticle", default)]
    pub articles: Vec<PubmedArticleXml>,
}

/// Article identifier (PMID, PMC, DOI, etc.)
#[derive(Debug, Deserialize)]
pub(super) struct ArticleId {
    #[serde(rename = "@IdType")]
    pub id_type: String,
    #[serde(rename = "$text")]
    pub value: String,
}

/// List of article identifiers
#[derive(Debug, Deserialize)]
pub(super) struct ArticleIdList {
    #[serde(rename = "ArticleId", default)]
    pub ids: Vec<ArticleId>,
}

/// PubMed data section containing additional article identifiers
#[derive(Debug, Deserialize)]
pub(super) struct PubmedData {
    #[serde(rename = "ArticleIdList")]
    pub article_id_list: Option<ArticleIdList>,
}

/// Top-level PubMed article element
#[derive(Debug, Deserialize)]
pub(super) struct PubmedArticleXml {
    #[serde(rename = "MedlineCitation")]
    pub medline_citation: MedlineCitation,
    #[serde(rename = "PubmedData")]
    pub pubmed_data: Option<PubmedData>,
}

/// MEDLINE citation containing article metadata
#[derive(Debug, Deserialize)]
pub(super) struct MedlineCitation {
    #[serde(rename = "PMID")]
    pub pmid: Option<PmidXml>,
    #[serde(rename = "Article")]
    pub article: Article,
    #[serde(rename = "MeshHeadingList")]
    pub mesh_heading_list: Option<MeshHeadingList>,
    #[serde(rename = "ChemicalList")]
    pub chemical_list: Option<ChemicalList>,
    #[serde(rename = "KeywordList")]
    pub keyword_list: Option<KeywordList>,
}

/// PubMed ID element
#[derive(Debug, Deserialize)]
pub(super) struct PmidXml {
    #[serde(rename = "$text")]
    pub value: String,
}

/// Article element containing core metadata
#[derive(Debug, Deserialize)]
pub(super) struct Article {
    #[serde(rename = "Journal")]
    pub journal: Option<Journal>,
    #[serde(rename = "ArticleTitle")]
    pub article_title: Option<String>,
    #[serde(rename = "Abstract")]
    pub abstract_section: Option<AbstractSection>,
    #[serde(rename = "AuthorList")]
    pub author_list: Option<AuthorList>,
    #[serde(rename = "PublicationTypeList")]
    pub publication_type_list: Option<PublicationTypeList>,
    #[serde(rename = "ELocationID")]
    pub elocation_ids: Option<Vec<ELocationID>>,
}

/// Journal information
#[derive(Debug, Deserialize)]
pub(super) struct Journal {
    #[serde(rename = "Title")]
    pub title: Option<String>,
    #[serde(rename = "JournalIssue")]
    pub journal_issue: Option<JournalIssue>,
}

/// Journal issue containing publication date
#[derive(Debug, Deserialize)]
pub(super) struct JournalIssue {
    #[serde(rename = "PubDate")]
    pub pub_date: Option<PubDate>,
}

/// Publication date with flexible format support
#[derive(Debug, Deserialize)]
pub(super) struct PubDate {
    #[serde(rename = "Year")]
    pub year: Option<String>,
    #[serde(rename = "Month")]
    pub month: Option<String>,
    #[serde(rename = "Day")]
    pub day: Option<String>,
    #[serde(rename = "MedlineDate")]
    pub medline_date: Option<String>,
}

impl fmt::Display for PubDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = if let Some(ref medline_date) = self.medline_date {
            medline_date.clone()
        } else {
            let mut date_parts = Vec::new();
            if let Some(ref year) = self.year {
                date_parts.push(year.clone());
            }
            if let Some(ref month) = self.month {
                date_parts.push(month.clone());
            }
            if let Some(ref day) = self.day {
                date_parts.push(day.clone());
            }
            date_parts.join(" ")
        };
        write!(f, "{}", result)
    }
}

/// Abstract section containing text elements
#[derive(Debug, Deserialize)]
pub(super) struct AbstractSection {
    #[serde(rename = "AbstractText", default)]
    pub abstract_texts: Vec<AbstractTextElement>,
}

impl AbstractSection {
    /// Convert abstract sections to a single string
    ///
    /// Joins all abstract text elements with spaces, or returns None if empty.
    pub fn to_string_opt(&self) -> Option<String> {
        if self.abstract_texts.is_empty() {
            None
        } else {
            Some(
                self.abstract_texts
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        }
    }
}

/// Abstract text element (may include Label attribute for structured abstracts)
#[derive(Debug)]
pub(super) struct AbstractTextElement {
    pub text: String,
}

impl<'de> Deserialize<'de> for AbstractTextElement {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let text = deserialize_abstract_text(deserializer)?;
        Ok(AbstractTextElement { text })
    }
}

impl fmt::Display for AbstractTextElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

/// List of authors
#[derive(Debug, Deserialize)]
pub(super) struct AuthorList {
    #[serde(rename = "Author")]
    pub authors: Option<Vec<AuthorXml>>,
}

/// Author element with name components and affiliations
#[derive(Debug, Deserialize)]
pub(super) struct AuthorXml {
    #[serde(rename = "LastName")]
    pub last_name: Option<String>,
    #[serde(rename = "ForeName")]
    pub fore_name: Option<String>,
    #[serde(rename = "Initials")]
    pub initials: Option<String>,
    #[serde(rename = "Suffix")]
    pub suffix: Option<String>,
    #[serde(rename = "AffiliationInfo")]
    pub affiliation_info: Option<Vec<AffiliationInfo>>,
    #[serde(rename = "Identifier")]
    pub identifiers: Option<Vec<Identifier>>,
    #[serde(rename = "CollectiveName")]
    pub collective_name: Option<String>,
}

/// Affiliation information for an author
#[derive(Debug, Deserialize)]
pub(super) struct AffiliationInfo {
    #[serde(rename = "Affiliation")]
    pub affiliation: Option<String>,
}

/// Author identifier (ORCID, etc.)
#[derive(Debug, Deserialize)]
pub(super) struct Identifier {
    #[serde(rename = "$text")]
    pub value: String,
    #[serde(rename = "@Source")]
    pub source: Option<String>,
}

/// List of publication types
#[derive(Debug, Deserialize)]
pub(super) struct PublicationTypeList {
    #[serde(rename = "PublicationType")]
    pub publication_types: Option<Vec<PublicationType>>,
}

/// Publication type (e.g., "Journal Article", "Clinical Trial")
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum PublicationType {
    Simple(String),
    Complex {
        #[serde(rename = "$text")]
        text: String,
        #[serde(rename = "@UI")]
        #[allow(dead_code)]
        ui: Option<String>,
    },
}

impl fmt::Display for PublicationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PublicationType::Simple(s) => write!(f, "{}", s),
            PublicationType::Complex { text, .. } => write!(f, "{}", text),
        }
    }
}

/// Electronic location identifier (DOI, etc.)
#[derive(Debug, Deserialize)]
pub(super) struct ELocationID {
    #[serde(rename = "$text")]
    pub value: String,
    #[serde(rename = "@EIdType")]
    pub eid_type: Option<String>,
}

/// List of MeSH headings
#[derive(Debug, Deserialize)]
pub(super) struct MeshHeadingList {
    #[serde(rename = "MeshHeading")]
    pub mesh_headings: Option<Vec<MeshHeadingXml>>,
}

/// MeSH heading with descriptor and qualifiers
#[derive(Debug, Deserialize)]
pub(super) struct MeshHeadingXml {
    #[serde(rename = "DescriptorName")]
    pub descriptor_name: Option<DescriptorName>,
    #[serde(rename = "QualifierName")]
    pub qualifier_names: Option<Vec<QualifierName>>,
}

/// MeSH descriptor name
#[derive(Debug, Deserialize)]
pub(super) struct DescriptorName {
    #[serde(rename = "$text")]
    pub text: String,
    #[serde(rename = "@UI")]
    pub ui: Option<String>,
    #[serde(rename = "@MajorTopicYN", deserialize_with = "deserialize_bool_yn")]
    pub major_topic_yn: bool,
}

/// MeSH qualifier/subheading name
#[derive(Debug, Deserialize)]
pub(super) struct QualifierName {
    #[serde(rename = "$text")]
    pub text: String,
    #[serde(rename = "@UI")]
    pub ui: Option<String>,
    #[serde(rename = "@MajorTopicYN", deserialize_with = "deserialize_bool_yn")]
    pub major_topic_yn: bool,
}

/// List of chemical substances
#[derive(Debug, Deserialize)]
pub(super) struct ChemicalList {
    #[serde(rename = "Chemical")]
    pub chemicals: Option<Vec<ChemicalXml>>,
}

/// Chemical substance information
#[derive(Debug, Deserialize)]
pub(super) struct ChemicalXml {
    #[serde(rename = "RegistryNumber")]
    pub registry_number: Option<String>,
    #[serde(rename = "NameOfSubstance")]
    pub name_of_substance: Option<NameOfSubstance>,
}

/// Name of a chemical substance
#[derive(Debug, Deserialize)]
pub(super) struct NameOfSubstance {
    #[serde(rename = "$text")]
    pub text: String,
    #[serde(rename = "@UI")]
    pub ui: Option<String>,
}

/// List of keywords
#[derive(Debug, Deserialize)]
pub(super) struct KeywordList {
    #[serde(rename = "Keyword")]
    pub keywords: Option<Vec<KeywordElement>>,
}

/// Keyword element
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum KeywordElement {
    Simple(String),
    Complex {
        #[serde(rename = "$text")]
        text: String,
        #[serde(rename = "@MajorTopicYN")]
        #[allow(dead_code)]
        major_topic_yn: Option<String>,
    },
}

impl fmt::Display for KeywordElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeywordElement::Simple(s) => write!(f, "{}", s),
            KeywordElement::Complex { text, .. } => write!(f, "{}", text),
        }
    }
}
