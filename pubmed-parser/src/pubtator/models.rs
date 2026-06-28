//! Domain models for PubTator3 annotations in the BioC JSON format.
//!
//! [PubTator3] is NCBI's AI-powered biomedical text-mining service. Its export
//! API returns annotated documents in the [BioC] JSON format, where each
//! document is split into passages (title, abstract, body sections) and each
//! passage carries entity annotations (genes, diseases, chemicals, species,
//! variants, …) with normalized database identifiers, plus relations between
//! entities.
//!
//! These types model that format faithfully while exposing ergonomic accessors
//! (e.g. [`BioCAnnotation::entity_type`], [`BioCDocument::annotations`]) so
//! callers rarely need to reach into the raw `infons` maps.
//!
//! [PubTator3]: https://www.ncbi.nlm.nih.gov/research/pubtator3/
//! [BioC]: https://bioc.sourceforge.net/

use std::collections::BTreeMap;

use serde::de::{Deserializer, Error as DeError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Top-level response from the PubTator3 publications export endpoint.
///
/// The endpoint returns a single JSON object with one key, `PubTator3`, holding
/// the list of annotated documents.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PubTatorResponse {
    /// The annotated documents, one per requested PMID/PMCID.
    #[serde(rename = "PubTator3", default)]
    pub documents: Vec<BioCDocument>,
}

impl PubTatorResponse {
    /// Find a document by its identifier (PMID or PMCID), if present.
    pub fn document(&self, id: &str) -> Option<&BioCDocument> {
        self.documents.iter().find(|d| d.id == id)
    }
}

/// A single annotated document (one article).
///
/// Beyond the BioC-standard `id`, `infons`, `passages`, and `relations`,
/// PubTator3 adds convenience fields such as `pmid`, `pmcid`, `journal`, and
/// `date` at the document level; these are captured when present.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BioCDocument {
    /// Document identifier (the PMID for abstract-level exports, the PMCID for
    /// full-text exports).
    #[serde(default)]
    pub id: String,
    /// PubMed identifier, when reported by PubTator3.
    #[serde(default, deserialize_with = "string_or_number_opt")]
    pub pmid: Option<String>,
    /// PMC identifier, when reported by PubTator3.
    #[serde(default, deserialize_with = "string_or_number_opt")]
    pub pmcid: Option<String>,
    /// Journal citation string, when reported.
    #[serde(default)]
    pub journal: Option<String>,
    /// Publication date string, when reported.
    #[serde(default)]
    pub date: Option<String>,
    /// Document-level key/value metadata.
    #[serde(default)]
    pub infons: BTreeMap<String, Value>,
    /// Passages making up the document (title, abstract, sections, …).
    #[serde(default)]
    pub passages: Vec<BioCPassage>,
    /// Document-level relations between annotated entities.
    #[serde(default)]
    pub relations: Vec<BioCRelation>,
}

impl BioCDocument {
    /// Iterate over every annotation across all passages.
    pub fn annotations(&self) -> impl Iterator<Item = &BioCAnnotation> {
        self.passages.iter().flat_map(|p| p.annotations.iter())
    }

    /// Collect every annotation whose entity type matches `entity_type`.
    pub fn annotations_of_type(
        &self,
        entity_type: EntityType,
    ) -> impl Iterator<Item = &BioCAnnotation> {
        self.annotations()
            .filter(move |a| a.entity_type() == entity_type)
    }

    /// The first passage of the given `type` infon (e.g. `"title"`,
    /// `"abstract"`).
    pub fn passage_of_type(&self, passage_type: &str) -> Option<&BioCPassage> {
        self.passages
            .iter()
            .find(|p| p.passage_type() == Some(passage_type))
    }

    /// The document title text, if a `title` passage is present.
    pub fn title(&self) -> Option<&str> {
        self.passage_of_type("title").map(|p| p.text.as_str())
    }

    /// The document abstract text, if an `abstract` passage is present.
    pub fn abstract_text(&self) -> Option<&str> {
        self.passage_of_type("abstract").map(|p| p.text.as_str())
    }
}

/// A contiguous span of text within a document, with its own annotations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BioCPassage {
    /// Passage-level metadata (notably `type`, `section_type`, `journal`, …).
    #[serde(default)]
    pub infons: BTreeMap<String, Value>,
    /// Character offset of this passage from the start of the document.
    #[serde(default)]
    pub offset: u32,
    /// The passage text.
    #[serde(default)]
    pub text: String,
    /// Entity annotations located within this passage.
    #[serde(default)]
    pub annotations: Vec<BioCAnnotation>,
    /// Relations scoped to this passage.
    #[serde(default)]
    pub relations: Vec<BioCRelation>,
}

impl BioCPassage {
    /// The passage `type` infon (e.g. `"title"`, `"abstract"`), if set.
    pub fn passage_type(&self) -> Option<&str> {
        infon_str(&self.infons, "type")
    }

    /// The passage `section_type` infon, if set (full-text exports only).
    pub fn section_type(&self) -> Option<&str> {
        infon_str(&self.infons, "section_type")
    }
}

/// A single entity annotation: a mention of a bio-entity in the text.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BioCAnnotation {
    /// Annotation identifier, unique within its passage.
    #[serde(default)]
    pub id: String,
    /// Annotation metadata, including `type`, `identifier`, `normalized_id`,
    /// `database`, and `name`.
    #[serde(default)]
    pub infons: BTreeMap<String, Value>,
    /// The exact surface text that was annotated.
    #[serde(default)]
    pub text: String,
    /// Character spans this annotation covers (usually exactly one).
    #[serde(default)]
    pub locations: Vec<BioCLocation>,
}

impl BioCAnnotation {
    /// The entity type of this annotation (gene, disease, chemical, …).
    pub fn entity_type(&self) -> EntityType {
        infon_str(&self.infons, "type")
            .map(EntityType::from_type_str)
            .unwrap_or(EntityType::Other(String::new()))
    }

    /// The source database identifier for the normalized entity
    /// (e.g. `"MESH:D004317"`, `"672"`), if the mention was normalized.
    pub fn identifier(&self) -> Option<&str> {
        infon_str(&self.infons, "identifier")
    }

    /// The normalizing database name (e.g. `"ncbi_mesh"`, `"ncbi_gene"`).
    pub fn database(&self) -> Option<&str> {
        infon_str(&self.infons, "database")
    }

    /// The canonical entity name assigned by PubTator3, if any.
    pub fn name(&self) -> Option<&str> {
        infon_str(&self.infons, "name")
    }

    /// Whether PubTator3 considered this mention successfully normalized.
    pub fn is_normalized(&self) -> bool {
        self.infons
            .get("valid")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// The start offset of the first location, if any.
    pub fn start(&self) -> Option<u32> {
        self.locations.first().map(|l| l.offset)
    }

    /// The end offset (exclusive) of the first location, if any.
    pub fn end(&self) -> Option<u32> {
        self.locations.first().map(|l| l.offset + l.length)
    }
}

/// A character span: an offset from the document start and a length.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BioCLocation {
    /// Character offset from the start of the document.
    pub offset: u32,
    /// Length of the span in characters.
    pub length: u32,
}

/// A relation between annotated entities (e.g. a chemical that treats a
/// disease, two co-administered drugs).
///
/// PubTator3 stores the participating entities and the relation type inside the
/// `infons` map (`role1`, `role2`, `type`, `score`); the ergonomic accessors
/// surface those without manual map traversal.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BioCRelation {
    /// Relation identifier within its scope.
    #[serde(default)]
    pub id: String,
    /// Relation metadata (`type`, `score`, `role1`, `role2`, …).
    #[serde(default)]
    pub infons: BTreeMap<String, Value>,
    /// BioC reference nodes linking the relation to annotations.
    #[serde(default)]
    pub nodes: Vec<BioCNode>,
}

impl BioCRelation {
    /// The relation type (e.g. `"Treat"`, `"Cause"`, `"Cotreatment"`), if set.
    pub fn relation_type(&self) -> Option<&str> {
        infon_str(&self.infons, "type")
    }

    /// The confidence score reported by PubTator3, parsed from its string form.
    pub fn score(&self) -> Option<f64> {
        match self.infons.get("score") {
            Some(Value::String(s)) => s.parse().ok(),
            Some(Value::Number(n)) => n.as_f64(),
            _ => None,
        }
    }

    /// The first participating entity, parsed from the `role1` infon.
    pub fn role1(&self) -> Option<RelationRole> {
        RelationRole::from_infon(self.infons.get("role1"))
    }

    /// The second participating entity, parsed from the `role2` infon.
    pub fn role2(&self) -> Option<RelationRole> {
        RelationRole::from_infon(self.infons.get("role2"))
    }
}

/// One participant in a [`BioCRelation`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RelationRole {
    /// Database identifier of the entity (e.g. `"MESH:D004317"`).
    #[serde(default)]
    pub identifier: Option<String>,
    /// Raw entity type string as reported by PubTator3.
    #[serde(rename = "type", default)]
    pub entity_type: Option<String>,
    /// Canonical entity name.
    #[serde(default)]
    pub name: Option<String>,
}

impl RelationRole {
    fn from_infon(value: Option<&Value>) -> Option<Self> {
        value.and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// A BioC relation node linking the relation to an annotation by reference.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BioCNode {
    /// The referenced annotation identifier.
    #[serde(default)]
    pub refid: String,
    /// The role this node plays in the relation.
    #[serde(default)]
    pub role: String,
}

/// The category of a bio-entity recognized by PubTator3.
///
/// `from_type_str`/`as_str` use PubTator3's canonical capitalized spellings.
/// Unknown categories are preserved verbatim in [`EntityType::Other`] so no
/// information is lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityType {
    /// A gene or gene product.
    Gene,
    /// A disease or phenotype.
    Disease,
    /// A chemical or drug.
    Chemical,
    /// A species/organism.
    Species,
    /// A sequence variant or mutation.
    Variant,
    /// A cell line.
    CellLine,
    /// Any other entity type, preserving the original string.
    Other(String),
}

impl EntityType {
    /// Parse a PubTator3 `type` string into an [`EntityType`], case-insensitively.
    ///
    /// `"Mutation"` is treated as a synonym for [`EntityType::Variant`].
    pub fn from_type_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "gene" => EntityType::Gene,
            "disease" => EntityType::Disease,
            "chemical" => EntityType::Chemical,
            "species" => EntityType::Species,
            "variant" | "mutation" | "dnamutation" | "proteinmutation" | "snp" => {
                EntityType::Variant
            }
            "cellline" | "cell_line" => EntityType::CellLine,
            _ => EntityType::Other(s.to_string()),
        }
    }

    /// The canonical PubTator3 spelling of this entity type.
    pub fn as_str(&self) -> &str {
        match self {
            EntityType::Gene => "Gene",
            EntityType::Disease => "Disease",
            EntityType::Chemical => "Chemical",
            EntityType::Species => "Species",
            EntityType::Variant => "Variant",
            EntityType::CellLine => "CellLine",
            EntityType::Other(s) => s,
        }
    }
}

/// A single suggestion from the PubTator3 entity autocomplete endpoint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityMatch {
    /// PubTator3 entity accession (e.g. `"@GENE_BRCA1"`, `"@DISEASE_COVID_19"`).
    #[serde(rename = "_id", default)]
    pub id: String,
    /// Entity biotype (e.g. `"gene"`, `"disease"`, `"chemical"`).
    #[serde(default)]
    pub biotype: String,
    /// Source database identifier.
    #[serde(rename = "db_id", default)]
    pub db_id: String,
    /// Source database name (e.g. `"ncbi_gene"`, `"ncbi_mesh"`).
    #[serde(default)]
    pub db: String,
    /// Display name of the entity.
    #[serde(default)]
    pub name: String,
    /// Human-readable description/qualifier, when provided.
    #[serde(default)]
    pub description: String,
    /// HTML-highlighted match explanation returned by the API.
    #[serde(rename = "match", default)]
    pub matched: String,
}

impl EntityMatch {
    /// The entity type implied by this match's `biotype`.
    pub fn entity_type(&self) -> EntityType {
        EntityType::from_type_str(&self.biotype)
    }
}

/// Read a string-valued infon, returning `None` for missing or non-string keys.
fn infon_str<'a>(infons: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a str> {
    infons.get(key).and_then(Value::as_str)
}

/// Deserialize an optional identifier that PubTator3 may emit as either a JSON
/// string or a JSON number (PMIDs arrive as integers, PMCIDs as strings).
fn string_or_number_opt<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<Value>::deserialize(deserializer)? {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(Value::Number(n)) => Ok(Some(n.to_string())),
        Some(other) => Err(DeError::custom(format!(
            "expected string or number identifier, got {other}"
        ))),
    }
}
