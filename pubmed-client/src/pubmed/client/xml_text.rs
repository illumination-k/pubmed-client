//! Lightweight string-slicing helpers for the simple, flat E-utilities XML
//! responses (ESpell, EGQuery).
//!
//! These responses are shallow and sparse, so `find()`-based slicing is simpler
//! and faster than a full Reader-based parse. Kept private to the client crate.

/// Extract the trimmed text between the first occurrence of `start` and `end`.
///
/// Returns `None` if either tag is not found.
pub(crate) fn extract_text_between(content: &str, start: &str, end: &str) -> Option<String> {
    let start_pos = content.find(start)? + start.len();
    let end_pos = content[start_pos..].find(end)? + start_pos;
    Some(content[start_pos..end_pos].trim().to_string())
}

/// Extract the trimmed text between every occurrence of `start` and `end`.
///
/// Empty segments are skipped.
pub(crate) fn extract_all_text_between(content: &str, start: &str, end: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut pos = 0;

    while let Some(start_pos) = content[pos..].find(start) {
        let start_pos = pos + start_pos + start.len();
        if let Some(end_pos) = content[start_pos..].find(end) {
            let end_pos = start_pos + end_pos;
            let text = content[start_pos..end_pos].trim().to_string();
            if !text.is_empty() {
                results.push(text);
            }
            pos = end_pos;
        } else {
            break;
        }
    }

    results
}
