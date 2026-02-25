//! Custom serde deserializers for complex PubMed XML fields
//!
//! This module provides specialized deserializers for handling complex XML structures
//! that don't map cleanly to standard serde deserialization patterns.

use serde::{Deserialize, Deserializer};
use std::fmt;
use std::result;

/// Represents a deserialized abstract text element with optional label
pub(super) struct AbstractTextWithLabel {
    pub label: Option<String>,
    pub text: String,
}

/// Custom deserializer for AbstractTextElement that handles all content including inline tags
///
/// Note: Inline HTML tags (`<i>`, `<sup>`, `<sub>`, etc.) are stripped during XML preprocessing.
///
/// # Implementation Details
///
/// This deserializer handles both simple string content and complex map structures with
/// `$text` or `$value` keys, as well as attributes like `@Label` for structured abstracts.
pub(super) fn deserialize_abstract_text_with_label<'de, D>(
    deserializer: D,
) -> result::Result<AbstractTextWithLabel, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, MapAccess, Visitor};

    struct AbstractTextVisitor;

    impl<'de> Visitor<'de> for AbstractTextVisitor {
        type Value = AbstractTextWithLabel;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("abstract text content")
        }

        fn visit_str<E>(self, value: &str) -> result::Result<AbstractTextWithLabel, E>
        where
            E: de::Error,
        {
            Ok(AbstractTextWithLabel {
                label: None,
                text: value.to_string(),
            })
        }

        fn visit_string<E>(self, value: String) -> result::Result<AbstractTextWithLabel, E>
        where
            E: de::Error,
        {
            Ok(AbstractTextWithLabel {
                label: None,
                text: value,
            })
        }

        fn visit_map<M>(self, mut map: M) -> result::Result<AbstractTextWithLabel, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut text_parts = Vec::new();
            let mut label = None;
            while let Some(key) = map.next_key::<String>()? {
                if key == "$text" || key == "$value" {
                    let value: String = map.next_value()?;
                    text_parts.push(value);
                } else if key == "@Label" {
                    let value: String = map.next_value()?;
                    label = Some(value);
                } else {
                    // Skip other attributes like @NlmCategory
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
            // Join all text parts (handles mixed content with inline tags)
            Ok(AbstractTextWithLabel {
                label,
                text: text_parts.join(""),
            })
        }
    }

    deserializer.deserialize_any(AbstractTextVisitor)
}

/// Deserialize a boolean from "Y"/"N" string values
///
/// PubMed XML uses "Y" and "N" strings for boolean values in attributes
/// like `MajorTopicYN`.
///
/// # Behavior
///
/// * `Some("Y")` → `true`
/// * `Some("N")` → `false`
/// * `None` → `false`
/// * Any other value → `false`
pub(super) fn deserialize_bool_yn<'de, D>(deserializer: D) -> result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.is_some_and(|s| s == "Y"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::de::from_str;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct TestBoolYN {
        #[serde(rename = "@value", default, deserialize_with = "deserialize_bool_yn")]
        value: bool,
    }

    #[test]
    fn test_deserialize_bool_yn() {
        // Test "Y" → true
        let xml = r#"<TestBoolYN value="Y" />"#;
        let result: TestBoolYN = from_str(xml).unwrap();
        assert!(result.value);

        // Test "N" → false
        let xml = r#"<TestBoolYN value="N" />"#;
        let result: TestBoolYN = from_str(xml).unwrap();
        assert!(!result.value);

        // Test missing attribute → false (should default to false)
        let xml = r#"<TestBoolYN />"#;
        let result: TestBoolYN = from_str(xml).unwrap();
        assert!(!result.value);
    }
}
