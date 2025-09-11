use crate::error::{PubMedError, Result};
use crate::pubmed::models::{
    Affiliation, Author, ChemicalConcept, MeshHeading, MeshQualifier, MeshTerm, PubMedArticle,
};
use quick_xml::de::from_str;
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::result;
use tracing::{debug, instrument};

#[derive(Debug, Deserialize)]
#[serde(rename = "PubmedArticleSet")]
struct PubmedArticleSet {
    #[serde(rename = "PubmedArticle")]
    articles: Vec<PubmedArticleXml>,
}

#[derive(Debug, Deserialize)]
struct PubmedArticleXml {
    #[serde(rename = "MedlineCitation")]
    medline_citation: MedlineCitation,
}

impl PubmedArticleXml {
    fn into_article(self, pmid: &str) -> Result<PubMedArticle> {
        let medline = self.medline_citation;
        let article = medline.article;

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
            abstract_text,
            article_types,
            mesh_headings,
            keywords,
            chemical_list,
        })
    }
}

#[derive(Debug, Deserialize)]
struct MedlineCitation {
    #[serde(rename = "PMID")]
    pmid: Option<PmidXml>,
    #[serde(rename = "Article")]
    article: Article,
    #[serde(rename = "MeshHeadingList")]
    mesh_heading_list: Option<MeshHeadingList>,
    #[serde(rename = "ChemicalList")]
    chemical_list: Option<ChemicalList>,
    #[serde(rename = "KeywordList")]
    keyword_list: Option<KeywordList>,
}

#[derive(Debug, Deserialize)]
struct PmidXml {
    #[serde(rename = "$text")]
    value: String,
}

#[derive(Debug, Deserialize)]
struct Article {
    #[serde(rename = "Journal")]
    journal: Option<Journal>,
    #[serde(rename = "ArticleTitle")]
    article_title: Option<String>,
    #[serde(rename = "Abstract")]
    abstract_section: Option<AbstractSection>,
    #[serde(rename = "AuthorList")]
    author_list: Option<AuthorList>,
    #[serde(rename = "PublicationTypeList")]
    publication_type_list: Option<PublicationTypeList>,
    #[serde(rename = "ELocationID")]
    elocation_ids: Option<Vec<ELocationID>>,
}

#[derive(Debug, Deserialize)]
struct Journal {
    #[serde(rename = "Title")]
    title: Option<String>,
    #[serde(rename = "JournalIssue")]
    journal_issue: Option<JournalIssue>,
}

#[derive(Debug, Deserialize)]
struct JournalIssue {
    #[serde(rename = "PubDate")]
    pub_date: Option<PubDate>,
}

#[derive(Debug, Deserialize)]
struct PubDate {
    #[serde(rename = "Year")]
    year: Option<String>,
    #[serde(rename = "Month")]
    month: Option<String>,
    #[serde(rename = "Day")]
    day: Option<String>,
    #[serde(rename = "MedlineDate")]
    medline_date: Option<String>,
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

#[derive(Debug, Deserialize)]
struct AbstractSection {
    #[serde(rename = "AbstractText", default)]
    abstract_texts: Vec<AbstractTextElement>,
}

impl AbstractSection {
    fn to_string_opt(&self) -> Option<String> {
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AbstractTextElement {
    Simple(String),
    Structured {
        #[serde(rename = "$text")]
        text: String,
        #[serde(rename = "@Label")]
        #[allow(dead_code)]
        label: Option<String>,
    },
}

impl fmt::Display for AbstractTextElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AbstractTextElement::Simple(text) => write!(f, "{}", text),
            AbstractTextElement::Structured { text, .. } => write!(f, "{}", text),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AuthorList {
    #[serde(rename = "Author")]
    authors: Option<Vec<AuthorXml>>,
}

impl AuthorList {
    fn into_authors(self) -> Vec<Author> {
        self.authors
            .unwrap_or_default()
            .into_iter()
            .filter_map(|a| a.into_author())
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct AuthorXml {
    #[serde(rename = "LastName")]
    last_name: Option<String>,
    #[serde(rename = "ForeName")]
    fore_name: Option<String>,
    #[serde(rename = "Initials")]
    initials: Option<String>,
    #[serde(rename = "Suffix")]
    suffix: Option<String>,
    #[serde(rename = "AffiliationInfo")]
    affiliation_info: Option<Vec<AffiliationInfo>>,
    #[serde(rename = "Identifier")]
    identifiers: Option<Vec<Identifier>>,
    #[serde(rename = "CollectiveName")]
    collective_name: Option<String>,
}

impl AuthorXml {
    fn into_author(self) -> Option<Author> {
        // Handle collective names
        if let Some(collective_name) = self.collective_name {
            return Some(Author {
                last_name: None,
                fore_name: None,
                first_name: None,
                middle_name: None,
                initials: None,
                suffix: None,
                full_name: collective_name,
                affiliations: Vec::new(),
                orcid: None,
                is_corresponding: false,
                author_roles: Vec::new(),
            });
        }

        let full_name = format_author_name(&self.last_name, &self.fore_name, &self.initials);

        if full_name.trim().is_empty() || full_name == "Unknown Author" {
            None
        } else {
            let affiliations = self
                .affiliation_info
                .unwrap_or_default()
                .into_iter()
                .filter_map(|info| info.affiliation.map(|text| Affiliation::from_text(&text)))
                .collect();

            let orcid = self.identifiers.and_then(|ids| {
                ids.into_iter()
                    .find(|id| id.source.as_deref() == Some("ORCID"))
                    .map(|id| id.value)
            });

            Some(Author {
                last_name: self.last_name,
                fore_name: self.fore_name,
                first_name: None,
                middle_name: None,
                initials: self.initials,
                suffix: self.suffix,
                full_name,
                affiliations,
                orcid,
                is_corresponding: false,
                author_roles: Vec::new(),
            })
        }
    }
}

#[derive(Debug, Deserialize)]
struct AffiliationInfo {
    #[serde(rename = "Affiliation")]
    affiliation: Option<String>,
}

impl Affiliation {
    fn from_text(text: &str) -> Self {
        let text = text.trim();
        let email = extract_email_from_text(text);
        let country = extract_country_from_text(text);

        Affiliation {
            institution: if text.is_empty() {
                None
            } else {
                Some(text.to_string())
            },
            department: None,
            address: None,
            country,
            email,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Identifier {
    #[serde(rename = "$text")]
    value: String,
    #[serde(rename = "@Source")]
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PublicationTypeList {
    #[serde(rename = "PublicationType")]
    publication_types: Option<Vec<PublicationType>>,
}

impl PublicationTypeList {
    fn into_types(self) -> Vec<String> {
        self.publication_types
            .unwrap_or_default()
            .into_iter()
            .map(|pt| pt.to_string())
            .collect()
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PublicationType {
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

#[derive(Debug, Deserialize)]
struct ELocationID {
    #[serde(rename = "$text")]
    value: String,
    #[serde(rename = "@EIdType")]
    eid_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MeshHeadingList {
    #[serde(rename = "MeshHeading")]
    mesh_headings: Option<Vec<MeshHeadingXml>>,
}

impl MeshHeadingList {
    fn into_headings(self) -> Option<Vec<MeshHeading>> {
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

#[derive(Debug, Deserialize)]
struct MeshHeadingXml {
    #[serde(rename = "DescriptorName")]
    descriptor_name: Option<DescriptorName>,
    #[serde(rename = "QualifierName")]
    qualifier_names: Option<Vec<QualifierName>>,
}

impl MeshHeadingXml {
    fn into_heading(self) -> Option<MeshHeading> {
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

#[derive(Debug, Deserialize)]
struct DescriptorName {
    #[serde(rename = "$text")]
    text: String,
    #[serde(rename = "@UI")]
    ui: Option<String>,
    #[serde(rename = "@MajorTopicYN", deserialize_with = "deserialize_bool_yn")]
    major_topic_yn: bool,
}

#[derive(Debug, Deserialize)]
struct QualifierName {
    #[serde(rename = "$text")]
    text: String,
    #[serde(rename = "@UI")]
    ui: Option<String>,
    #[serde(rename = "@MajorTopicYN", deserialize_with = "deserialize_bool_yn")]
    major_topic_yn: bool,
}

impl QualifierName {
    fn into_qualifier(self) -> MeshQualifier {
        MeshQualifier {
            qualifier_name: self.text,
            qualifier_ui: self.ui.unwrap_or_default(),
            major_topic: self.major_topic_yn,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChemicalList {
    #[serde(rename = "Chemical")]
    chemicals: Option<Vec<ChemicalXml>>,
}

impl ChemicalList {
    fn into_chemicals(self) -> Option<Vec<ChemicalConcept>> {
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

#[derive(Debug, Deserialize)]
struct ChemicalXml {
    #[serde(rename = "RegistryNumber")]
    registry_number: Option<String>,
    #[serde(rename = "NameOfSubstance")]
    name_of_substance: Option<NameOfSubstance>,
}

impl ChemicalXml {
    fn into_chemical(self) -> Option<ChemicalConcept> {
        self.name_of_substance.map(|name| ChemicalConcept {
            name: name.text,
            registry_number: self.registry_number.filter(|r| !r.is_empty() && r != "0"),
            ui: name.ui,
        })
    }
}

#[derive(Debug, Deserialize)]
struct NameOfSubstance {
    #[serde(rename = "$text")]
    text: String,
    #[serde(rename = "@UI")]
    ui: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KeywordList {
    #[serde(rename = "Keyword")]
    keywords: Option<Vec<KeywordElement>>,
}

impl KeywordList {
    fn into_keywords(self) -> Option<Vec<String>> {
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum KeywordElement {
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

fn deserialize_bool_yn<'de, D>(deserializer: D) -> result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.is_some_and(|s| s == "Y"))
}

/// Parse article from EFetch XML response
#[instrument(skip(xml), fields(pmid = %pmid, xml_size = xml.len()))]
pub fn parse_article_from_xml(xml: &str, pmid: &str) -> Result<PubMedArticle> {
    // Parse the XML using quick-xml serde
    let article_set: PubmedArticleSet = from_str(xml).map_err(|e| PubMedError::XmlParseError {
        message: format!("Failed to deserialize XML: {}", e),
    })?;

    // Find the article with the matching PMID
    let article_xml = article_set
        .articles
        .into_iter()
        .find(|a| {
            a.medline_citation
                .pmid
                .as_ref()
                .is_some_and(|p| p.value == pmid)
        })
        .ok_or_else(|| PubMedError::ArticleNotFound {
            pmid: pmid.to_string(),
        })?;

    article_xml.into_article(pmid)
}

/// Extract email address from affiliation text
fn extract_email_from_text(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|part| part.contains('@') && part.contains('.'))
        .map(|part| part.trim_end_matches(&['.', ',', ';', ')'][..]).to_string())
        .filter(|email| email.len() > 5)
}

/// Extract country from affiliation text
fn extract_country_from_text(text: &str) -> Option<String> {
    const COUNTRIES: &[&str] = &[
        "USA",
        "United States",
        "US",
        "UK",
        "United Kingdom",
        "England",
        "Scotland",
        "Wales",
        "Canada",
        "Australia",
        "Germany",
        "France",
        "Italy",
        "Spain",
        "Japan",
        "China",
        "India",
        "Brazil",
        "Netherlands",
        "Sweden",
        "Switzerland",
        "Denmark",
        "Norway",
        "Finland",
        "Belgium",
        "Austria",
        "Portugal",
        "Ireland",
        "Israel",
        "South Korea",
        "Singapore",
        "Hong Kong",
        "Taiwan",
        "New Zealand",
        "Mexico",
    ];

    let text_lower = text.to_lowercase();
    COUNTRIES.iter().find_map(|&country| {
        let country_lower = country.to_lowercase();
        if text_lower.ends_with(&country_lower)
            || text_lower.contains(&format!(", {}", country_lower))
        {
            Some(country.to_string())
        } else {
            None
        }
    })
}

/// Format an author name from components
fn format_author_name(
    last_name: &Option<String>,
    fore_name: &Option<String>,
    initials: &Option<String>,
) -> String {
    match (fore_name, last_name) {
        (Some(fore), Some(last)) => format!("{fore} {last}"),
        (None, Some(last)) => {
            if let Some(init) = initials {
                format!("{init} {last}")
            } else {
                last.clone()
            }
        }
        (Some(fore), None) => fore.clone(),
        (None, None) => "Unknown Author".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_term_parsing() {
        let xml = r#"<?xml version="1.0" ?>
<!DOCTYPE PubmedArticleSet PUBLIC "-//NLM//DTD PubMedArticle, 1st January 2023//EN" "https://dtd.nlm.nih.gov/ncbi/pubmed/out/pubmed_230101.dtd">
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation Status="PubMed-not-MEDLINE" Owner="NLM">
        <PMID Version="1">12345678</PMID>
        <Article>
            <ArticleTitle>Test Article with MeSH Terms</ArticleTitle>
            <Abstract>
                <AbstractText>This is a test abstract.</AbstractText>
            </Abstract>
            <AuthorList>
                <Author>
                    <LastName>Doe</LastName>
                    <ForeName>John</ForeName>
                    <Initials>JA</Initials>
                    <AffiliationInfo>
                        <Affiliation>Department of Medicine, Harvard Medical School, Boston, MA, USA. john.doe@hms.harvard.edu</Affiliation>
                    </AffiliationInfo>
                    <Identifier Source="ORCID">0000-0001-2345-6789</Identifier>
                </Author>
            </AuthorList>
            <Journal>
                <Title>Test Journal</Title>
            </Journal>
        </Article>
        <MeshHeadingList>
            <MeshHeading>
                <DescriptorName UI="D003920" MajorTopicYN="Y">Diabetes Mellitus</DescriptorName>
                <QualifierName UI="Q000188" MajorTopicYN="N">drug therapy</QualifierName>
            </MeshHeading>
            <MeshHeading>
                <DescriptorName UI="D007333" MajorTopicYN="N">Insulin</DescriptorName>
            </MeshHeading>
        </MeshHeadingList>
        <ChemicalList>
            <Chemical>
                <RegistryNumber>11061-68-0</RegistryNumber>
                <NameOfSubstance UI="D007328">Insulin</NameOfSubstance>
            </Chemical>
        </ChemicalList>
        <KeywordList>
            <Keyword>diabetes treatment</Keyword>
            <Keyword>insulin therapy</Keyword>
        </KeywordList>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "12345678").unwrap();

        // Test MeSH headings
        assert!(article.mesh_headings.is_some());
        let mesh_headings = article.mesh_headings.as_ref().unwrap();
        assert_eq!(mesh_headings.len(), 2);

        // Test first MeSH heading (major topic with qualifier)
        let first_heading = &mesh_headings[0];
        assert_eq!(first_heading.mesh_terms.len(), 1);
        let diabetes_term = &first_heading.mesh_terms[0];
        assert_eq!(diabetes_term.descriptor_name, "Diabetes Mellitus");
        assert_eq!(diabetes_term.descriptor_ui, "D003920");
        assert!(diabetes_term.major_topic);
        assert_eq!(diabetes_term.qualifiers.len(), 1);
        assert_eq!(diabetes_term.qualifiers[0].qualifier_name, "drug therapy");
        assert_eq!(diabetes_term.qualifiers[0].qualifier_ui, "Q000188");
        assert!(!diabetes_term.qualifiers[0].major_topic);

        // Test second MeSH heading (non-major topic)
        let second_heading = &mesh_headings[1];
        assert_eq!(second_heading.mesh_terms.len(), 1);
        let insulin_term = &second_heading.mesh_terms[0];
        assert_eq!(insulin_term.descriptor_name, "Insulin");
        assert_eq!(insulin_term.descriptor_ui, "D007333");
        assert!(!insulin_term.major_topic);
        assert_eq!(insulin_term.qualifiers.len(), 0);

        // Test chemicals
        assert!(article.chemical_list.is_some());
        let chemicals = article.chemical_list.as_ref().unwrap();
        assert_eq!(chemicals.len(), 1);
        assert_eq!(chemicals[0].name, "Insulin");
        assert_eq!(chemicals[0].registry_number, Some("11061-68-0".to_string()));
        assert_eq!(chemicals[0].ui, Some("D007328".to_string()));

        // Test author parsing
        assert_eq!(article.authors.len(), 1);
        assert_eq!(article.author_count, 1);
        let author = &article.authors[0];
        assert_eq!(author.last_name, Some("Doe".to_string()));
        assert_eq!(author.fore_name, Some("John".to_string()));
        assert_eq!(author.initials, Some("JA".to_string()));
        assert_eq!(author.full_name, "John Doe");
        assert_eq!(author.orcid, Some("0000-0001-2345-6789".to_string()));
        assert_eq!(author.affiliations.len(), 1);
        assert!(author.affiliations[0]
            .institution
            .as_ref()
            .unwrap()
            .contains("Harvard Medical School"));

        // Test keywords
        assert!(article.keywords.is_some());
        let keywords = article.keywords.as_ref().unwrap();
        assert_eq!(keywords.len(), 2);
        assert_eq!(keywords[0], "diabetes treatment");
        assert_eq!(keywords[1], "insulin therapy");
    }

    #[test]
    fn test_structured_abstract_parsing() {
        let xml = r#"
        <PubmedArticleSet>
            <PubmedArticle>
                <MedlineCitation>
                    <PMID>32887691</PMID>
                    <Article>
                        <ArticleTitle>A living WHO guideline on drugs for covid-19.</ArticleTitle>
                        <Abstract>
                            <AbstractText Label="UPDATES">This is the fourteenth version (thirteenth update) of the living guideline, replacing earlier versions.</AbstractText>
                            <AbstractText Label="CLINICAL QUESTION">What is the role of drugs in the treatment of patients with covid-19?</AbstractText>
                            <AbstractText Label="CONTEXT">The evidence base for therapeutics for covid-19 is evolving with numerous randomised controlled trials.</AbstractText>
                        </Abstract>
                        <Journal>
                            <Title>BMJ (Clinical research ed.)</Title>
                            <JournalIssue>
                                <PubDate>
                                    <Year>2020</Year>
                                    <Month>Sep</Month>
                                </PubDate>
                            </JournalIssue>
                        </Journal>
                    </Article>
                </MedlineCitation>
            </PubmedArticle>
        </PubmedArticleSet>"#;

        let result = parse_article_from_xml(xml, "32887691");
        assert!(result.is_ok());

        let article = result.unwrap();
        assert_eq!(article.pmid, "32887691");
        assert_eq!(
            article.title,
            "A living WHO guideline on drugs for covid-19."
        );

        // Verify that all three abstract sections are concatenated
        let abstract_text = article.abstract_text.unwrap();
        assert!(abstract_text.contains("This is the fourteenth version"));
        assert!(abstract_text.contains("What is the role of drugs"));
        assert!(abstract_text.contains("The evidence base for therapeutics"));

        // Verify they are properly concatenated with spaces
        assert!(abstract_text.contains("earlier versions. What is the role"));
        assert!(abstract_text.contains("covid-19? The evidence base"));
    }

    #[test]
    fn test_article_without_mesh_terms() {
        let xml = r#"<?xml version="1.0" ?>
<!DOCTYPE PubmedArticleSet PUBLIC "-//NLM//DTD PubMedArticle, 1st January 2023//EN" "https://dtd.nlm.nih.gov/ncbi/pubmed/out/pubmed_230101.dtd">
<PubmedArticleSet>
<PubmedArticle>
    <MedlineCitation Status="PubMed-not-MEDLINE" Owner="NLM">
        <PMID Version="1">87654321</PMID>
        <Article>
            <ArticleTitle>Article Without MeSH Terms</ArticleTitle>
            <AuthorList>
                <Author>
                    <LastName>Smith</LastName>
                    <ForeName>Jane</ForeName>
                </Author>
            </AuthorList>
            <Journal>
                <Title>Another Journal</Title>
            </Journal>
        </Article>
    </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;

        let article = parse_article_from_xml(xml, "87654321").unwrap();

        assert_eq!(article.authors.len(), 1);
        assert_eq!(article.author_count, 1);
        assert_eq!(article.authors[0].full_name, "Jane Smith");
        assert!(article.mesh_headings.is_none());
        assert!(article.chemical_list.is_none());
        assert!(article.keywords.is_none());
    }

    #[test]
    fn test_extract_email_from_text() {
        assert_eq!(
            extract_email_from_text("Contact john.doe@example.com for details"),
            Some("john.doe@example.com".to_string())
        );

        assert_eq!(
            extract_email_from_text("Email: jane.smith@university.edu."),
            Some("jane.smith@university.edu".to_string())
        );

        assert_eq!(extract_email_from_text("No email here"), None);
    }

    #[test]
    fn test_extract_country_from_text() {
        assert_eq!(
            extract_country_from_text("Harvard Medical School, Boston, MA, USA"),
            Some("USA".to_string())
        );

        assert_eq!(
            extract_country_from_text("University of Oxford, Oxford, UK"),
            Some("UK".to_string())
        );

        assert_eq!(extract_country_from_text("Local Institution"), None);
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
    }
}
