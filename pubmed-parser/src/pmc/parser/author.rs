use crate::common::xml_utils::strip_inline_html_tags;
use crate::common::{Affiliation, Author};
use crate::error::Result;
use crate::pmc::parser::reader_utils::{
    get_attr, make_reader, read_text_content, resolve_general_ref, skip_element,
};
use quick_xml::Reader;
use quick_xml::de::from_str;
use quick_xml::events::Event;
use serde::Deserialize;
use std::collections::HashMap;
use std::mem;

/// XML structure for aff element
#[derive(Debug, Deserialize)]
struct Aff {
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

/// Extract authors from PMC XML content (normally the `<front>` slice).
///
/// Contributors are read with a streaming reader from every `<contrib-group>`
/// (books and chapters split authors over several groups). Contributors whose
/// `contrib-type` is something other than `author` — editors, reviewers,
/// translators — are skipped, as are the member lists nested inside a
/// `<collab>`.
pub(crate) fn extract_authors(content: &str) -> Result<Vec<Author>> {
    // Build an index of `<aff id="...">` blocks so `<xref ref-type="aff">` rids
    // can be resolved to real institution/department/address/country text. In
    // JATS, `<aff>` elements are usually siblings of `<contrib-group>` inside
    // `<article-meta>`, so index the whole `<front>` slice, not just contrib-group.
    let aff_index = build_affiliation_index(content);

    let mut reader = make_reader(content);
    let mut authors: Vec<Author> = Vec::new();
    // `<aff>`s outside any `<contrib>`: per enclosing `<contrib-group>`, and
    // at article level.
    let mut group_start: Option<usize> = None;
    let mut group_affs = Vec::new();
    let mut article_affs = Vec::new();
    // Inside `<aff-alternatives>` only the first (primary-language) `<aff>` counts.
    let mut in_aff_alternatives = false;
    let mut alternative_taken = false;

    loop {
        let tag_start = reader.buffer_position() as usize;
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"contrib" => {
                let is_author = get_attr(e, b"contrib-type")
                    .is_none_or(|t| t.trim().eq_ignore_ascii_case("author"));
                if !is_author {
                    let _ = skip_element(&mut reader, e.name());
                    continue;
                }
                let corresp = get_attr(e, b"corresp").is_some_and(|c| c == "yes");
                if let Some(author) = parse_contrib(&mut reader, content, corresp, &aff_index) {
                    authors.push(author);
                }
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"contrib-group" => {
                group_start = Some(authors.len());
                group_affs.clear();
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"contrib-group" => {
                // An `<aff>` placed in a `<contrib-group>` whose contributors
                // link none (no `<xref>`, no inline `<aff>`) applies to all of them.
                if let Some(start) = group_start.take()
                    && authors[start..].iter().all(|a| a.affiliations.is_empty())
                {
                    for author in &mut authors[start..] {
                        if author.collab_name.is_none() {
                            author.affiliations.clone_from(&group_affs);
                        }
                    }
                }
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"aff-alternatives" => {
                in_aff_alternatives = true;
                alternative_taken = false;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"aff-alternatives" => {
                in_aff_alternatives = false;
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"aff" => {
                let id = get_attr(e, b"id");
                let _ = skip_element(&mut reader, e.name());
                if in_aff_alternatives && mem::replace(&mut alternative_taken, true) {
                    continue;
                }
                let tag_end = reader.buffer_position() as usize;
                if let Some(block) = content.get(tag_start..tag_end)
                    && let Some(resolved) = resolve_aff_block(block, id)
                {
                    if group_start.is_some() {
                        group_affs.push(resolved);
                    } else {
                        article_affs.push(resolved);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!(
                    "XML error while reading contributors ({e}); keeping authors so far"
                );
                break;
            }
            _ => {}
        }
    }

    // Article-level `<aff>`s that no contributor links to (no `<xref>`s at
    // all) belong to every author.
    if !article_affs.is_empty() && authors.iter().all(|a| a.affiliations.is_empty()) {
        for author in &mut authors {
            if author.collab_name.is_none() {
                author.affiliations.clone_from(&article_affs);
            }
        }
    }

    Ok(authors)
}

/// Personal name parts read from `<name>` / `<string-name>`.
struct NameParts {
    surname: Option<String>,
    given_names: Option<String>,
    suffix: Option<String>,
}

/// Parse one author `<contrib>`; the reader has just consumed its start tag.
///
/// `content` is the string the reader was built from, used to slice out the
/// raw markup of inline `<aff>` blocks for [`resolve_aff_block`].
fn parse_contrib(
    reader: &mut Reader<&[u8]>,
    content: &str,
    corresp: bool,
    aff_index: &HashMap<String, Affiliation>,
) -> Option<Author> {
    let mut name: Option<NameParts> = None;
    let mut collab: Option<String> = None;
    let mut orcid = None;
    let mut email = None;
    let mut roles = Vec::new();
    let mut is_corresponding = corresp;
    let mut affiliations = Vec::new();

    loop {
        let tag_start = reader.buffer_position() as usize;
        let e = match reader.read_event() {
            Ok(Event::Start(e)) => e,
            Ok(Event::End(ref e)) if e.name().as_ref() == b"contrib" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => continue,
        };
        let qname = e.name();
        match qname.as_ref() {
            b"contrib-id" => {
                let is_orcid = get_attr(&e, b"contrib-id-type").as_deref() == Some("orcid");
                let value = read_trimmed(reader, b"contrib-id");
                if is_orcid && orcid.is_none() {
                    orcid = value;
                }
            }
            // The first name wins; inside `<name-alternatives>` that is the
            // primary (usually romanized) form.
            tag @ (b"name" | b"string-name") => {
                let parts = parse_name_parts(reader, tag);
                if name.is_none() {
                    name = Some(parts);
                }
            }
            b"collab" => {
                let text = read_collab_text(reader);
                if collab.is_none() && !text.is_empty() {
                    collab = Some(text);
                }
            }
            b"email" => {
                let value = read_trimmed(reader, b"email");
                if email.is_none() {
                    email = value;
                }
            }
            b"role" => roles.extend(read_trimmed(reader, b"role")),
            b"xref" => {
                let ref_type = get_attr(&e, b"ref-type");
                let rid = get_attr(&e, b"rid");
                let _ = skip_element(reader, qname);
                match ref_type.as_deref() {
                    Some("corresp") => is_corresponding = true,
                    // `rid` may list several ids separated by spaces.
                    Some("aff") => {
                        for rid in rid.as_deref().unwrap_or("").split_whitespace() {
                            affiliations.push(aff_index.get(rid).cloned().unwrap_or_else(|| {
                                // The referenced `<aff>` was not found (or had no
                                // resolvable content); keep the rid so the
                                // reference is not silently lost.
                                Affiliation {
                                    id: Some(rid.to_string()),
                                    institution: None,
                                    department: None,
                                    address: None,
                                    country: None,
                                }
                            }));
                        }
                    }
                    _ => {}
                }
            }
            // Inline `<aff>` inside `<contrib>`.
            b"aff" => {
                let id = get_attr(&e, b"id");
                let _ = skip_element(reader, qname);
                let tag_end = reader.buffer_position() as usize;
                if let Some(block) = content.get(tag_start..tag_end)
                    && let Some(resolved) = resolve_aff_block(block, id)
                {
                    affiliations.push(resolved);
                }
            }
            b"bio" | b"author-comment" | b"on-behalf-of" | b"fn" => {
                let _ = skip_element(reader, qname);
            }
            // Wrappers such as `<name-alternatives>` or `<address>`: descend.
            _ => {}
        }
    }

    // A contributor is either an individual (`<name>`) or a collaboration/group
    // (`<collab>`, e.g. a consortium). Prefer the personal name; fall back to
    // collab so group authors are not silently dropped.
    let mut author = match name {
        Some(parts) => {
            let mut author = Author::new(parts.surname, parts.given_names);
            author.suffix = parts.suffix;
            author.orcid = orcid;
            author.email = email;
            author.roles = roles;
            author.affiliations = affiliations;
            author
        }
        None => Author::collaboration(collab?),
    };
    author.is_corresponding = is_corresponding;
    Some(author)
}

/// Parse `<name>` / `<string-name>`; the reader has just consumed its start tag.
fn parse_name_parts(reader: &mut Reader<&[u8]>, tag: &[u8]) -> NameParts {
    let mut parts = NameParts {
        surname: None,
        given_names: None,
        suffix: None,
    };

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let child = e.name().as_ref().to_vec();
                let value = read_trimmed(reader, &child);
                let slot = match child.as_slice() {
                    b"surname" => &mut parts.surname,
                    b"given-names" => &mut parts.given_names,
                    b"suffix" => &mut parts.suffix,
                    _ => continue,
                };
                if slot.is_none() {
                    *slot = value;
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == tag => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    parts
}

/// Read the name of a `<collab>`, leaving out any nested member list
/// (`<contrib-group>`) and footnote markers; the reader has just consumed its
/// start tag.
fn read_collab_text(reader: &mut Reader<&[u8]>) -> String {
    let mut text = String::new();
    let mut depth = 1_u32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"contrib-group" | b"xref" | b"fn" | b"address" | b"email" => {
                    let _ = skip_element(reader, e.name());
                }
                b"collab" => depth += 1,
                _ => {}
            },
            Ok(Event::Text(ref t)) => {
                if let Ok(decoded) = t.decode() {
                    text.push_str(&decoded);
                }
            }
            Ok(Event::GeneralRef(ref r)) => {
                if let Ok(resolved) = resolve_general_ref(r) {
                    text.push_str(&resolved);
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"collab" => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Read the text of `tag` (reader just past its start tag), `None` when empty.
fn read_trimmed(reader: &mut Reader<&[u8]>, tag: &[u8]) -> Option<String> {
    read_text_content(reader, tag)
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Resolve a raw `<aff>...</aff>` block into an [`Affiliation`].
///
/// Structured deserialization (institution/addr-line/country) can fail on
/// affiliations with mixed free text interrupted by child elements (e.g.
/// inline `<email>`), so the error is tolerated and the raw free text is used
/// as the fallback.
fn resolve_aff_block(block: &str, id: Option<String>) -> Option<Affiliation> {
    let structured = from_str::<Aff>(&strip_inline_html_tags(block)).ok();
    let free_text = aff_free_text(block);
    resolve_affiliation(structured.as_ref(), id, &free_text)
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
        if let Some(resolved) = resolve_aff_block(block, Some(id.clone())) {
            index.insert(id, resolved);
        }
    }

    // `<aff-alternatives id="...">` carries the id on the wrapper and holds the
    // same affiliation in several languages; the first `<aff>` is the primary one.
    static ALT_REGEX: OnceLock<Option<Regex>> = OnceLock::new();
    let alt_re = ALT_REGEX.get_or_init(|| {
        Regex::new(r#"(?s)<aff-alternatives[^>]*\bid="([^"]+)"[^>]*>(.*?)</aff-alternatives>"#).ok()
    });
    if let Some(alt_re) = alt_re {
        for caps in alt_re.captures_iter(content) {
            let (Some(id), Some(inner)) = (caps.get(1), caps.get(2)) else {
                continue;
            };
            let id = id.as_str();
            if index.contains_key(id) {
                continue;
            }
            if let Some(first) = re.find(inner.as_str())
                && let Some(resolved) = resolve_aff_block(first.as_str(), Some(id.to_string()))
            {
                index.insert(id.to_string(), resolved);
            }
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
    fn test_aff_alternatives_resolved_by_wrapper_id() {
        let content = r#"
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author">
                    <name-alternatives>
                        <name xml:lang="en"><surname>Park</surname><given-names>Jee Won</given-names></name>
                        <name name-style="eastern"><surname>박</surname><given-names>지원</given-names></name>
                    </name-alternatives>
                    <xref rid="af1" ref-type="aff"><sup>1</sup></xref>
                </contrib>
            </contrib-group>
            <aff-alternatives id="af1">
                <aff xml:lang="en"><label>1</label>College of Nursing, Ajou University, Suwon, <country>Korea</country></aff>
                <aff><label>1</label>아주대학교 간호대학</aff>
            </aff-alternatives>
        </article-meta>
        "#;

        let authors = extract_authors(content).unwrap();
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].surname.as_deref(), Some("Park"));
        assert_eq!(authors[0].given_names.as_deref(), Some("Jee Won"));
        let aff = &authors[0].affiliations[0];
        assert_eq!(aff.id.as_deref(), Some("af1"));
        assert_eq!(aff.country.as_deref(), Some("Korea"));
        assert!(
            aff.institution
                .as_deref()
                .is_some_and(|i| i.contains("Ajou University"))
        );
    }

    #[test]
    fn test_skips_editors_and_reads_every_contrib_group() {
        // Book chapters list authors and editors in separate groups, and an
        // inline `<aff>` with mixed text must not drop the contributor.
        let content = r#"
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author" corresp="yes">
                    <name><surname>Labbe</surname><given-names>Danielle</given-names></name>
                    <aff id="a1">School of Urban Planning, <institution-wrap><institution-id institution-id-type="Ringgold">5622</institution-id><institution content-type="university">Universite de Montreal</institution></institution-wrap>, Montreal, Canada</aff>
                    <email>one@example.org</email>
                    <email>two@example.org</email>
                </contrib>
            </contrib-group>
            <contrib-group content-type="book-editors">
                <contrib contrib-type="editor">
                    <name><surname>Mahy</surname><given-names>Brian</given-names></name>
                </contrib>
            </contrib-group>
            <contrib-group>
                <contrib contrib-type="author">
                    <name><surname>Walker</surname><given-names>P.J.</given-names></name>
                </contrib>
                <contrib contrib-type="author">
                    <collab>Study Group<contrib-group><contrib contrib-type="author"><name><surname>Member</surname></name></contrib></contrib-group></collab>
                </contrib>
            </contrib-group>
        </article-meta>
        "#;

        let authors = extract_authors(content).unwrap();
        let names: Vec<&str> = authors.iter().map(|a| a.full_name.as_str()).collect();
        assert_eq!(names, ["Danielle Labbe", "P.J. Walker", "Study Group"]);
        assert!(authors[0].is_corresponding);
        assert_eq!(authors[0].email.as_deref(), Some("one@example.org"));
        assert_eq!(
            authors[0].affiliations[0].institution.as_deref(),
            Some("Universite de Montreal")
        );
    }

    #[test]
    fn test_unlinked_affiliations_apply_to_all_authors() {
        // Group-level `<aff>` applies to the group's contributors.
        let grouped = r#"
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author"><name><surname>Walker</surname><given-names>P</given-names></name></contrib>
                <aff id="mc1">CSIRO Australian Animal Health Laboratory, Geelong, Australia</aff>
            </contrib-group>
            <contrib-group>
                <contrib contrib-type="author"><name><surname>Sitt</surname><given-names>N</given-names></name></contrib>
                <aff id="mc2">Centex Shrimp, Bangkok, Thailand</aff>
            </contrib-group>
        </article-meta>
        "#;
        let authors = extract_authors(grouped).unwrap();
        assert_eq!(authors[0].affiliations[0].id.as_deref(), Some("mc1"));
        assert_eq!(authors[1].affiliations.len(), 1);
        assert_eq!(authors[1].affiliations[0].id.as_deref(), Some("mc2"));

        // Article-level `<aff>` with no xref anywhere applies to every author.
        let article_level = r#"
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author" corresp="yes"><name><surname>Avery</surname><given-names>E</given-names></name><xref rid="c1" ref-type="corresp"/></contrib>
            </contrib-group>
            <aff id="aff1">Geography, <institution>McGill University</institution>, Montreal, QC, CA</aff>
        </article-meta>
        "#;
        let authors = extract_authors(article_level).unwrap();
        assert_eq!(
            authors[0].affiliations[0].institution.as_deref(),
            Some("McGill University")
        );

        // ...but not when authors link their affiliations explicitly.
        let linked = r#"
        <article-meta>
            <contrib-group>
                <contrib contrib-type="author"><name><surname>A</surname></name><xref rid="aff1" ref-type="aff">1</xref></contrib>
                <contrib contrib-type="author"><name><surname>B</surname></name></contrib>
            </contrib-group>
            <aff id="aff1">First Institute</aff>
            <aff id="aff2">Second Institute</aff>
        </article-meta>
        "#;
        let authors = extract_authors(linked).unwrap();
        assert_eq!(authors[0].affiliations.len(), 1);
        assert!(authors[1].affiliations.is_empty());
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
