//! Custom serde deserialization helpers for Europe PMC JSON responses.
//!
//! Europe PMC is inconsistent about whether some scalar fields are encoded as
//! JSON strings or JSON numbers (for example `pubYear` is a string in the
//! `search` endpoint but a number in the `references` endpoint). These helpers
//! normalize such fields into `Option<String>` so the domain models stay simple.

use serde::{Deserialize, Deserializer};

/// Deserialize a field that may be a string, a number, a bool, or null into an
/// `Option<String>`. Missing fields should be paired with `#[serde(default)]`.
pub(crate) fn opt_string_flex<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        other => Some(other.to_string()),
    })
}
