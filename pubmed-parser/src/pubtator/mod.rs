//! PubTator3 BioC parsing and data models.
//!
//! Pure, network-free parsing of the JSON responses returned by NCBI's
//! [PubTator3] text-mining API:
//!
//! - [`parse_biocjson`] parses the publications export endpoint (BioC JSON) into
//!   a [`PubTatorResponse`] of annotated [`BioCDocument`]s.
//! - [`parse_entity_matches`] parses the entity autocomplete endpoint into a
//!   list of [`EntityMatch`] suggestions.
//!
//! [PubTator3]: https://www.ncbi.nlm.nih.gov/research/pubtator3/

mod models;

pub use models::{
    BioCAnnotation, BioCDocument, BioCLocation, BioCNode, BioCPassage, BioCRelation, EntityMatch,
    EntityType, PubTatorResponse, RelationRole,
};

use crate::error::Result;

/// Parse a PubTator3 BioC JSON export response.
///
/// Accepts the body returned by
/// `…/research/pubtator3-api/publications/export/biocjson`.
///
/// # Errors
///
/// Returns [`crate::ParseError::JsonError`] if the body is not valid BioC JSON.
pub fn parse_biocjson(json: &str) -> Result<PubTatorResponse> {
    Ok(serde_json::from_str(json)?)
}

/// Parse a PubTator3 entity autocomplete response.
///
/// Accepts the JSON array returned by
/// `…/research/pubtator3-api/entity/autocomplete/`.
///
/// # Errors
///
/// Returns [`crate::ParseError::JsonError`] if the body is not a valid JSON
/// array of entity matches.
pub fn parse_entity_matches(json: &str) -> Result<Vec<EntityMatch>> {
    Ok(serde_json::from_str(json)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIOCJSON: &str = include_str!("../../../test_data/pubtator/biocjson_two_abstracts.json");
    const AUTOCOMPLETE: &str = include_str!("../../../test_data/pubtator/autocomplete_covid.json");

    #[test]
    fn test_parse_biocjson_documents() {
        let response = parse_biocjson(BIOCJSON).expect("fixture should parse");
        assert_eq!(response.documents.len(), 2);

        let doc = response
            .document("29355051")
            .expect("PMID 29355051 should be present");
        assert!(doc.title().unwrap().contains("Lycium barbarum"));
        assert!(doc.abstract_text().is_some());
    }

    #[test]
    fn test_annotation_accessors() {
        let response = parse_biocjson(BIOCJSON).unwrap();
        let doc = response.document("29355051").unwrap();

        // The title passage annotates Doxorubicin as a normalized chemical.
        let dox = doc
            .annotations()
            .find(|a| a.text == "Doxorubicin")
            .expect("Doxorubicin annotation");
        assert_eq!(dox.entity_type(), EntityType::Chemical);
        assert_eq!(dox.identifier(), Some("MESH:D004317"));
        assert_eq!(dox.database(), Some("ncbi_mesh"));
        assert!(dox.is_normalized());
        assert_eq!(dox.start(), Some(104));
        assert_eq!(dox.end(), Some(115));
    }

    #[test]
    fn test_annotations_of_type_filter() {
        let response = parse_biocjson(BIOCJSON).unwrap();
        let doc = response.document("29355051").unwrap();

        let species: Vec<_> = doc
            .annotations_of_type(EntityType::Species)
            .map(|a| a.text.as_str())
            .collect();
        assert!(species.contains(&"Mice"));
        assert!(species.contains(&"Lycium barbarum"));
    }

    #[test]
    fn test_unnormalized_annotation_is_not_valid() {
        let response = parse_biocjson(BIOCJSON).unwrap();
        let doc = response.document("29355051").unwrap();
        let immuno = doc
            .annotations()
            .find(|a| a.text == "Immunotoxicity")
            .expect("Immunotoxicity annotation");
        assert_eq!(immuno.entity_type(), EntityType::Disease);
        assert!(!immuno.is_normalized());
        assert_eq!(immuno.identifier(), None);
    }

    #[test]
    fn test_relation_accessors() {
        let response = parse_biocjson(BIOCJSON).unwrap();
        // PMID 28483577 carries a Cotreatment relation between two chemicals.
        let doc = response.document("28483577").unwrap();
        let relation = doc
            .relations
            .iter()
            .find(|r| r.relation_type() == Some("Cotreatment"))
            .expect("Cotreatment relation");

        let role1 = relation.role1().expect("role1");
        let role2 = relation.role2().expect("role2");
        assert_eq!(role1.name.as_deref(), Some("Fluticasone"));
        assert_eq!(role2.name.as_deref(), Some("Formoterol Fumarate"));
        assert!(relation.score().unwrap() > 0.0);
    }

    #[test]
    fn test_parse_entity_matches() {
        let matches = parse_entity_matches(AUTOCOMPLETE).expect("fixture should parse");
        assert!(!matches.is_empty());
        let covid = &matches[0];
        assert_eq!(covid.id, "@DISEASE_COVID_19");
        assert_eq!(covid.entity_type(), EntityType::Disease);
        assert_eq!(covid.db, "ncbi_mesh");
    }

    #[test]
    fn test_entity_type_roundtrip() {
        assert_eq!(EntityType::from_type_str("gene"), EntityType::Gene);
        assert_eq!(EntityType::from_type_str("Mutation"), EntityType::Variant);
        assert_eq!(EntityType::Chemical.as_str(), "Chemical");
        assert_eq!(
            EntityType::from_type_str("CellMarker"),
            EntityType::Other("CellMarker".to_string())
        );
    }

    #[test]
    fn test_parse_empty_response() {
        let response = parse_biocjson(r#"{"PubTator3": []}"#).unwrap();
        assert!(response.documents.is_empty());
    }
}
