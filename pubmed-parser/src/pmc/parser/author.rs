use crate::common::xml_utils::strip_inline_html_tags;
use crate::common::{Affiliation, Author};
use crate::error::{ParseError, Result};
use quick_xml::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;

/// XML structure for contrib-group element
#[derive(Debug, Deserialize)]
#[serde(rename = "contrib-group")]
struct ContribGroup {
    #[serde(rename = "contrib", default)]
    contribs: Vec<Contrib>,
}

/// XML structure for contrib element
#[derive(Debug, Deserialize)]
struct Contrib {
    #[serde(rename = "@corresp", default)]
    corresp: Option<String>,

    #[serde(rename = "contrib-id", default)]
    contrib_ids: Vec<ContribId>,

    #[serde(rename = "name", default)]
    name: Option<Name>,

    #[serde(rename = "collab", default)]
    collab: Option<String>,

    #[serde(rename = "email", default)]
    email: Option<String>,

    #[serde(rename = "role", default)]
    roles: Vec<String>,

    #[serde(rename = "xref", default)]
    xrefs: Vec<Xref>,

    #[serde(rename = "aff", default)]
    affs: Vec<Aff>,
}

/// XML structure for contrib-id element
#[derive(Debug, Deserialize)]
struct ContribId {
    #[serde(rename = "@contrib-id-type")]
    contrib_id_type: Option<String>,

    #[serde(rename = "$text")]
    value: Option<String>,
}

/// XML structure for name element
#[derive(Debug, Deserialize)]
struct Name {
    #[serde(rename = "@name-style", default)]
    #[allow(dead_code)]
    name_style: Option<String>,

    #[serde(rename = "surname", default)]
    surname: Option<String>,

    #[serde(rename = "given-names", default)]
    given_names: Option<String>,

    #[serde(rename = "suffix", default)]
    suffix: Option<String>,
}

/// XML structure for xref element
#[derive(Debug, Deserialize)]
struct Xref {
    #[serde(rename = "@ref-type")]
    ref_type: Option<String>,

    #[serde(rename = "@rid")]
    rid: Option<String>,
}

/// XML structure for aff element
#[derive(Debug, Deserialize)]
struct Aff {
    #[serde(rename = "@id")]
    id: Option<String>,

    #[serde(rename = "$text", default)]
    text: Option<String>,

    #[serde(rename = "institution", default)]
    institutions: Vec<Institution>,

    #[serde(rename = "institution-wrap", default)]
    institution_wraps: Vec<InstitutionWrap>,

    #[serde(rename = "addr-line", default)]
    addr_lines: Vec<String>,

    #[serde(rename = "country", default)]
    countries: Vec<String>,
}

/// XML structure for institution element (may carry a `content-type` such as "dept")
#[derive(Debug, Deserialize)]
struct Institution {
    #[serde(rename = "@content-type", default)]
    content_type: Option<String>,

    #[serde(rename = "$text", default)]
    value: Option<String>,
}

/// XML structure for institution-wrap element (JATS wraps `<institution>` children)
#[derive(Debug, Deserialize)]
struct InstitutionWrap {
    #[serde(rename = "institution", default)]
    institutions: Vec<Institution>,
}

/// Extract authors from PMC XML content
pub(crate) fn extract_authors(content: &str) -> Result<Vec<Author>> {
    // Build an index of `<aff id="...">` blocks so `<xref ref-type="aff">` rids
    // can be resolved to real institution/department/address/country text. In
    // JATS, `<aff>` elements are usually siblings of `<contrib-group>` inside
    // `<article-meta>`, so index the whole `<front>` slice, not just contrib-group.
    let aff_index = build_affiliation_index(content);

    // Find and extract the contrib-group section
    if let Some(contrib_start) = content.find("<contrib-group>") {
        if let Some(contrib_end) = content[contrib_start..].find("</contrib-group>") {
            let contrib_section =
                &content[contrib_start..contrib_start + contrib_end + "</contrib-group>".len()];

            // Try to deserialize the contrib-group (strip inline HTML tags first)
            let cleaned_section = strip_inline_html_tags(contrib_section);
            match from_str::<ContribGroup>(&cleaned_section) {
                Ok(contrib_group) => {
                    let authors = contrib_group
                        .contribs
                        .into_iter()
                        .filter_map(|contrib| parse_contrib_to_author(contrib, &aff_index))
                        .collect();
                    Ok(authors)
                }
                Err(e) => {
                    // Log the error but continue with empty authors rather than failing completely
                    tracing::warn!(
                        "Failed to parse contrib-group XML ({}), continuing with empty authors",
                        e
                    );
                    Ok(Vec::new())
                }
            }
        } else {
            Err(ParseError::XmlError(
                "Found contrib-group start tag but no matching end tag".to_string(),
            ))
        }
    } else {
        // No contrib-group found - return empty vector as success
        Ok(Vec::new())
    }
}

/// Convert a Contrib to an Author, resolving affiliation xrefs against `aff_index`.
fn parse_contrib_to_author(
    contrib: Contrib,
    aff_index: &HashMap<String, Affiliation>,
) -> Option<Author> {
    // A contributor is either an individual (`<name>`) or a collaboration/group
    // (`<collab>`, e.g. a consortium). Prefer the personal name; fall back to
    // collab so group authors are not silently dropped.
    let Some(name) = contrib.name else {
        let collab = contrib.collab?;
        let collab = collab.trim();
        if collab.is_empty() {
            return None;
        }
        let mut author = Author::collaboration(collab.to_string());
        author.is_corresponding = contrib.corresp.as_deref() == Some("yes")
            || contrib
                .xrefs
                .iter()
                .any(|x| x.ref_type.as_deref() == Some("corresp"));
        return Some(author);
    };

    let mut author = Author::new(name.surname.clone(), name.given_names.clone());
    author.suffix = name.suffix;

    // Extract ORCID from contrib-id tags
    for contrib_id in &contrib.contrib_ids {
        if let Some(id_type) = &contrib_id.contrib_id_type
            && id_type == "orcid"
            && let Some(value) = &contrib_id.value
        {
            let clean_orcid = value.trim();
            if clean_orcid.contains("orcid.org") || !clean_orcid.is_empty() {
                author.orcid = Some(clean_orcid.to_string());
                break;
            }
        }
    }

    // Set email
    author.email = contrib.email.map(|e| e.trim().to_string());

    // Set corresponding author flag (check both corresp="yes" attribute and <xref ref-type="corresp">)
    author.is_corresponding = contrib.corresp.map(|c| c == "yes").unwrap_or(false)
        || contrib
            .xrefs
            .iter()
            .any(|x| x.ref_type.as_deref() == Some("corresp"));

    // Set roles
    author.roles = contrib
        .roles
        .into_iter()
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty())
        .collect();

    // Extract affiliations from xrefs
    let mut affiliations = Vec::new();

    // Process xref affiliations: resolve each rid against the `<aff>` index.
    for xref in &contrib.xrefs {
        if let Some(ref_type) = &xref.ref_type
            && ref_type == "aff"
            && let Some(rid) = &xref.rid
        {
            if let Some(resolved) = aff_index.get(rid) {
                affiliations.push(resolved.clone());
            } else {
                // The referenced `<aff>` was not found (or had no resolvable
                // content); keep the rid so the reference is not silently lost.
                affiliations.push(Affiliation {
                    id: Some(rid.clone()),
                    institution: None,
                    department: None,
                    address: None,
                    country: None,
                });
            }
        }
    }

    // Process direct affiliations (inline `<aff>` inside `<contrib>`).
    for aff in &contrib.affs {
        let free_text = aff
            .text
            .as_deref()
            .map(clean_affiliation_text)
            .unwrap_or_default();
        if let Some(resolved) = resolve_affiliation(Some(aff), aff.id.clone(), &free_text) {
            affiliations.push(resolved);
        }
    }

    author.affiliations = affiliations;

    Some(author)
}

/// Build an index mapping each `<aff id="...">` to its resolved [`Affiliation`].
///
/// Scans `content` (typically the `<front>` slice) for `<aff>` blocks and
/// resolves each one individually. `<aff-alternatives>` wrappers are ignored
/// (the inner `<aff>` blocks are matched directly). Blocks without an `id`, or
/// with no resolvable content, are skipped.
fn build_affiliation_index(content: &str) -> HashMap<String, Affiliation> {
    use regex::Regex;
    use std::sync::OnceLock;

    // Match `<aff>`/`<aff ...>` ... `</aff>` non-greedily. The `[ >]` after
    // `aff` prevents matching `<aff-alternatives>` (`aff` is never nested).
    static AFF_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    let re = AFF_REGEX.get_or_init(|| Regex::new(r"(?s)<aff[ >].*?</aff>").ok());
    let Some(re) = re else {
        return HashMap::new();
    };

    let mut index = HashMap::new();
    for m in re.find_iter(content) {
        let block = m.as_str();
        let Some(id) = extract_aff_id(block) else {
            continue;
        };
        // Try structured deserialization (institution/addr-line/country). This
        // can fail on affiliations with mixed free text interrupted by child
        // elements (e.g. inline `<email>`), so tolerate the error and rely on
        // the raw free-text fallback instead.
        let structured = from_str::<Aff>(&strip_inline_html_tags(block)).ok();
        let free_text = aff_free_text(block);
        if let Some(resolved) =
            resolve_affiliation(structured.as_ref(), Some(id.clone()), &free_text)
        {
            index.insert(id, resolved);
        }
    }
    index
}

/// Extract the `id` attribute value from an `<aff ...>` opening tag.
fn extract_aff_id(block: &str) -> Option<String> {
    use regex::Regex;
    use std::sync::OnceLock;

    static ID_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    let re = ID_REGEX.get_or_init(|| Regex::new(r#"<aff[^>]*\bid="([^"]+)""#).ok());
    re.as_ref()?
        .captures(block)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extract cleaned free text from a raw `<aff>` block.
///
/// Drops `<label>` and `<email>` element content (labels are markers, emails are
/// contact data — neither belongs in the institution string), strips all
/// remaining tags, decodes entities, collapses whitespace, and trims a leading
/// label marker plus trailing separators.
fn aff_free_text(block: &str) -> String {
    use crate::common::xml_utils::{decode_xml_entities, strip_xml_tags};
    use regex::Regex;
    use std::borrow::Cow;
    use std::sync::OnceLock;

    // Institution/address text precedes any contact list, so cut the block at
    // the first `<email>`: everything after it is email addresses and the
    // author-initial markers that map to them, not part of the affiliation.
    let block = match block.find("<email") {
        Some(idx) => &block[..idx],
        None => block,
    };

    static DROP_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    let re = DROP_REGEX.get_or_init(|| Regex::new(r"(?s)<(label|email)\b.*?</(label|email)>").ok());

    let without_dropped = match re {
        Some(re) => re.replace_all(block, ""),
        None => Cow::Borrowed(block),
    };
    let stripped = strip_xml_tags(&without_dropped);
    let decoded = decode_xml_entities(&stripped);
    // Collapse internal whitespace runs into single spaces.
    let collapsed = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    // Strip a leading label marker, then any trailing separators.
    clean_affiliation_text(&collapsed)
        .trim_end_matches(|c: char| c == ';' || c == ',' || c.is_whitespace())
        .to_string()
}

/// Resolve an affiliation into an [`Affiliation`], populating structured fields
/// (`institution`, `department`, `address`, `country`) from JATS sub-elements
/// when available and falling back to `free_text` for the institution otherwise.
///
/// Returns `None` if no meaningful content could be extracted.
fn resolve_affiliation(
    aff: Option<&Aff>,
    id: Option<String>,
    free_text: &str,
) -> Option<Affiliation> {
    // Gather `<institution>` elements from both direct children and
    // `<institution-wrap>`. `content-type="dept*"` denotes a department.
    let mut institution_parts = Vec::new();
    let mut department_parts = Vec::new();
    if let Some(aff) = aff {
        let all_institutions = aff.institutions.iter().chain(
            aff.institution_wraps
                .iter()
                .flat_map(|w| w.institutions.iter()),
        );
        for inst in all_institutions {
            let Some(value) = inst.value.as_deref() else {
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            if inst
                .content_type
                .as_deref()
                .is_some_and(|ct| ct.starts_with("dept"))
            {
                department_parts.push(value.to_string());
            } else {
                institution_parts.push(value.to_string());
            }
        }
    }

    let department = join_non_empty(&department_parts);

    let address_parts: Vec<String> = aff
        .map(|a| {
            a.addr_lines
                .iter()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let address = join_non_empty(&address_parts);

    let country = aff.and_then(|a| {
        a.countries
            .iter()
            .map(|c| c.trim())
            .find(|c| !c.is_empty())
            .map(str::to_string)
    });

    // Institution: prefer structured `<institution>` text; otherwise fall back
    // to the affiliation's free text.
    let institution = if !institution_parts.is_empty() {
        join_non_empty(&institution_parts)
    } else {
        let free_text = free_text.trim();
        (!free_text.is_empty()).then(|| free_text.to_string())
    };

    if institution.is_none() && department.is_none() && address.is_none() && country.is_none() {
        return None;
    }

    Some(Affiliation {
        id,
        institution,
        department,
        address,
        country,
    })
}

/// Join non-empty parts with ", ", returning `None` when the slice is empty.
fn join_non_empty(parts: &[String]) -> Option<String> {
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

/// Clean free-text affiliation content by trimming a leading label marker.
///
/// JATS affiliations are often prefixed with a superscript label (e.g. `<sup>1</sup>`
/// or `*`). Formatting/label tags are stripped upstream, leaving the bare label
/// digits/symbols; strip that leading run so the text starts at the institution name.
fn clean_affiliation_text(text: &str) -> String {
    let trimmed = text.trim();
    // Strip a leading label: digits/symbols/punctuation before the first letter.
    let without_label = trimmed.trim_start_matches(|c: char| {
        c.is_ascii_digit() || c.is_whitespace() || "*†‡§¶#,.;:-–—".contains(c)
    });
    without_label.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_collab_group_author() {
        let content = r#"
        <contrib-group>
            <contrib contrib-type="author">
                <name><surname>Doe</surname><given-names>John</given-names></name>
            </contrib>
            <contrib contrib-type="author">
                <collab>The COVID-19 Study Group</collab>
            </contrib>
        </contrib-group>
        "#;
        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 2);
        assert!(!authors[0].is_collaboration());
        assert_eq!(authors[0].surname.as_deref(), Some("Doe"));
        assert!(authors[1].is_collaboration());
        assert_eq!(
            authors[1].collab_name.as_deref(),
            Some("The COVID-19 Study Group")
        );
        assert_eq!(authors[1].full_name, "The COVID-19 Study Group");
    }

    #[test]
    fn test_extract_authors_detailed() {
        let content = r#"
        <contrib-group>
            <contrib corresp="yes">
                <name>
                    <surname>Doe</surname>
                    <given-names>John</given-names>
                </name>
                <email>john.doe@example.com</email>
                <role>Principal Investigator</role>
            </contrib>
        </contrib-group>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].surname, Some("Doe".to_string()));
        assert_eq!(authors[0].given_names, Some("John".to_string()));
        assert!(authors[0].is_corresponding);
        assert_eq!(authors[0].email, Some("john.doe@example.com".to_string()));
        assert_eq!(authors[0].roles, vec!["Principal Investigator"]);
    }

    #[test]
    fn test_extract_orcid_from_contrib_id() {
        let content = r#"
        <contrib-group>
            <contrib corresp="yes">
                <contrib-id contrib-id-type="orcid">https://orcid.org/0000-0002-3066-2940</contrib-id>
                <name name-style="western">
                    <surname>Doe</surname>
                    <given-names>John</given-names>
                </name>
                <email>john.doe@example.com</email>
            </contrib>
        </contrib-group>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].surname, Some("Doe".to_string()));
        assert_eq!(authors[0].given_names, Some("John".to_string()));
        assert_eq!(
            authors[0].orcid,
            Some("https://orcid.org/0000-0002-3066-2940".to_string())
        );
        assert!(authors[0].is_corresponding);
    }

    #[test]
    fn test_extract_orcid_with_xml_tags() {
        let content = r#"
        <contrib-group>
            <contrib>
                <contrib-id contrib-id-type="orcid">https://orcid.org/0000-0001-2345-6789</contrib-id><name name-style="western">
                    <surname>Smith</surname>
                    <given-names>Jane</given-names>
                </name>
            </contrib>
        </contrib-group>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].surname, Some("Smith".to_string()));
        assert_eq!(authors[0].given_names, Some("Jane".to_string()));
        assert_eq!(
            authors[0].orcid,
            Some("https://orcid.org/0000-0001-2345-6789".to_string())
        );
        assert!(!authors[0].is_corresponding);
    }

    #[test]
    fn test_resolve_xref_affiliation_structured() {
        // Structured <aff> with <institution>/<addr-line>/<country>, referenced
        // from the contributor via <xref ref-type="aff">.
        let content = r#"
        <front>
            <article-meta>
                <contrib-group>
                    <contrib contrib-type="author">
                        <name><surname>Doe</surname><given-names>John</given-names></name>
                        <xref ref-type="aff" rid="aff1">1</xref>
                    </contrib>
                    <aff id="aff1">
                        <label>1</label>
                        <institution content-type="dept">Department of Functional Genomics</institution>
                        <institution>Institute of Molecular Biology</institution>
                        <addr-line>Berlin</addr-line>
                        <country>Germany</country>
                    </aff>
                </contrib-group>
            </article-meta>
        </front>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        let aff = &authors[0].affiliations[0];
        assert_eq!(aff.id.as_deref(), Some("aff1"));
        assert_eq!(
            aff.institution.as_deref(),
            Some("Institute of Molecular Biology")
        );
        assert_eq!(
            aff.department.as_deref(),
            Some("Department of Functional Genomics")
        );
        assert_eq!(aff.address.as_deref(), Some("Berlin"));
        assert_eq!(aff.country.as_deref(), Some("Germany"));
    }

    #[test]
    fn test_resolve_xref_affiliation_free_text() {
        // <aff> with a superscript label and free mixed text (no sub-elements).
        let content = r#"
        <front>
            <article-meta>
                <contrib-group>
                    <contrib contrib-type="author">
                        <name><surname>Smith</surname><given-names>Jane</given-names></name>
                        <xref ref-type="aff" rid="aff2"><sup>2</sup></xref>
                    </contrib>
                </contrib-group>
                <aff id="aff2"><sup>2</sup>Department of Functional Genomics, Institute of Bioengineering, Lausanne, Switzerland</aff>
            </article-meta>
        </front>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        let aff = &authors[0].affiliations[0];
        assert_eq!(aff.id.as_deref(), Some("aff2"));
        assert_eq!(
            aff.institution.as_deref(),
            Some(
                "Department of Functional Genomics, Institute of Bioengineering, Lausanne, Switzerland"
            )
        );
    }

    #[test]
    fn test_resolve_xref_affiliation_missing_keeps_rid() {
        // Referenced <aff> is absent: keep the rid as the id, no bogus institution.
        let content = r#"
        <front>
            <article-meta>
                <contrib-group>
                    <contrib contrib-type="author">
                        <name><surname>Doe</surname><given-names>John</given-names></name>
                        <xref ref-type="aff" rid="aff9">9</xref>
                    </contrib>
                </contrib-group>
            </article-meta>
        </front>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        let aff = &authors[0].affiliations[0];
        assert_eq!(aff.id.as_deref(), Some("aff9"));
        assert_eq!(aff.institution, None);
        assert_eq!(aff.department, None);
        assert_eq!(aff.address, None);
        assert_eq!(aff.country, None);
    }

    #[test]
    fn test_extract_multiple_authors_with_orcid() {
        let content = r#"
        <contrib-group>
            <contrib>
                <contrib-id contrib-id-type="orcid">https://orcid.org/0000-0001-1111-1111</contrib-id>
                <name>
                    <surname>First</surname>
                    <given-names>Author</given-names>
                </name>
            </contrib>
            <contrib corresp="yes">
                <contrib-id contrib-id-type="orcid">https://orcid.org/0000-0002-2222-2222</contrib-id>
                <name>
                    <surname>Second</surname>
                    <given-names>Author</given-names>
                </name>
            </contrib>
            <contrib>
                <name>
                    <surname>Third</surname>
                    <given-names>Author</given-names>
                </name>
            </contrib>
        </contrib-group>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 3);

        // First author with ORCID
        assert_eq!(authors[0].surname, Some("First".to_string()));
        assert_eq!(
            authors[0].orcid,
            Some("https://orcid.org/0000-0001-1111-1111".to_string())
        );
        assert!(!authors[0].is_corresponding);

        // Second author with ORCID and corresponding
        assert_eq!(authors[1].surname, Some("Second".to_string()));
        assert_eq!(
            authors[1].orcid,
            Some("https://orcid.org/0000-0002-2222-2222".to_string())
        );
        assert!(authors[1].is_corresponding);

        // Third author without ORCID
        assert_eq!(authors[2].surname, Some("Third".to_string()));
        assert_eq!(authors[2].orcid, None);
        assert!(!authors[2].is_corresponding);
    }
}
