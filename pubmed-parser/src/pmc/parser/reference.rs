//! JATS `<ref-list>` / `<ref>` parsing.
//!
//! References are read with a streaming `quick_xml::Reader` rather than serde
//! deserialization: real PMC reference lists mix free text with tagged fields
//! (`<mixed-citation>`), wrap alternatives in `<citation-alternatives>`, use
//! `<string-name>` outside any `<person-group>`, and carry attribute-bearing
//! inline markup (`<italic toggle="yes">`). A single such `<ref>` used to make
//! the whole list fail to deserialize; the streaming parser simply picks the
//! fields it knows and walks past everything else.

use crate::common::Author;
use crate::error::Result;
use crate::pmc::domain::Reference;
use crate::pmc::parser::reader_utils::{get_attr, make_reader, read_text_content, skip_element};
use quick_xml::Reader;
use quick_xml::events::Event;

/// Extract every `<ref>` in `content` (normally the `<back>` slice).
///
/// Nested and repeated `<ref-list>`s are all covered because `<ref>` elements
/// are matched wherever they appear. A `<ref>` with no citation child is
/// dropped.
pub(crate) fn extract_references_detailed(content: &str) -> Result<Vec<Reference>> {
    let mut reader = make_reader(content);
    let mut references = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"ref" => {
                let id = get_attr(e, b"id");
                if let Some(reference) = parse_ref(&mut reader, id) {
                    references.push(reference);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::debug!("Stopped reading references at XML error: {e}");
                break;
            }
            _ => {}
        }
    }

    tracing::debug!(count = references.len(), "Extracted references");
    Ok(references)
}

/// Kind of citation element, in order of preference when a `<ref>` carries
/// several (e.g. inside `<citation-alternatives>`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CitationKind {
    /// `<element-citation>`: fully tagged, no free text.
    Element,
    /// `<nlm-citation>` / `<citation>`: tagged citations from older DTDs.
    Legacy,
    /// `<mixed-citation>`: tagged fields interleaved with punctuation text.
    Mixed,
}

impl CitationKind {
    fn from_tag(tag: &[u8]) -> Option<Self> {
        match tag {
            b"element-citation" => Some(Self::Element),
            b"nlm-citation" | b"citation" => Some(Self::Legacy),
            b"mixed-citation" => Some(Self::Mixed),
            _ => None,
        }
    }
}

/// Fields collected from one citation element. Each scalar keeps the first
/// occurrence.
#[derive(Default)]
struct Citation {
    publication_type: Option<String>,
    article_title: Option<String>,
    chapter_title: Option<String>,
    source: Option<String>,
    year: Option<String>,
    volume: Option<String>,
    issue: Option<String>,
    fpage: Option<String>,
    lpage: Option<String>,
    elocation_id: Option<String>,
    publisher_name: Option<String>,
    publisher_loc: Option<String>,
    edition: Option<String>,
    isbn: Option<String>,
    conf_name: Option<String>,
    doi: Option<String>,
    pmid: Option<String>,
    authors: Vec<Author>,
    editors: Vec<Author>,
}

impl Citation {
    /// Number of tagged fields, used to break ties between citations of the same kind.
    fn richness(&self) -> usize {
        [
            &self.article_title,
            &self.chapter_title,
            &self.source,
            &self.year,
            &self.volume,
            &self.fpage,
            &self.doi,
            &self.pmid,
        ]
        .iter()
        .filter(|f| f.is_some())
        .count()
            + usize::from(!self.authors.is_empty())
    }

    fn into_reference(self, id: String) -> Reference {
        Reference {
            id,
            publication_type: self.publication_type,
            title: self.article_title.or(self.chapter_title),
            authors: self.authors,
            source: self.source,
            year: self.year,
            volume: self.volume,
            issue: self.issue,
            pages: format_pages(self.fpage, self.lpage),
            elocation_id: self.elocation_id,
            editors: self.editors,
            publisher_name: self.publisher_name,
            publisher_loc: self.publisher_loc,
            edition: self.edition,
            isbn: self.isbn,
            conf_name: self.conf_name,
            pmid: self.pmid,
            doi: self.doi,
        }
    }
}

/// Parse one `<ref>`; the reader has just consumed its start tag.
fn parse_ref(reader: &mut Reader<&[u8]>, id: Option<String>) -> Option<Reference> {
    let mut best: Option<(CitationKind, Citation)> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let tag = e.name().as_ref().to_vec();
                let Some(kind) = CitationKind::from_tag(&tag) else {
                    // `<label>`, `<citation-alternatives>`, `<note>`, ... —
                    // descend so wrapped citations are still found.
                    continue;
                };
                let publication_type =
                    get_attr(e, b"publication-type").or_else(|| get_attr(e, b"citation-type"));
                let citation = parse_citation(reader, &tag, publication_type);
                let better = match &best {
                    None => true,
                    Some((best_kind, best_citation)) => {
                        kind < *best_kind
                            || (kind == *best_kind
                                && citation.richness() > best_citation.richness())
                    }
                };
                if better {
                    best = Some((kind, citation));
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"ref" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    let (_, citation) = best?;
    Some(citation.into_reference(id.unwrap_or_else(|| String::from("unknown"))))
}

/// Parse a citation element named `tag`; the reader has just consumed its start tag.
fn parse_citation(
    reader: &mut Reader<&[u8]>,
    tag: &[u8],
    publication_type: Option<String>,
) -> Citation {
    let mut c = Citation {
        publication_type,
        ..Citation::default()
    };

    loop {
        let e = match reader.read_event() {
            Ok(Event::Start(e)) => e,
            Ok(Event::End(ref e)) if e.name().as_ref() == tag => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => continue,
        };
        let name = e.name();
        let child = name.as_ref();

        let slot = match child {
            b"article-title" => &mut c.article_title,
            b"chapter-title" => &mut c.chapter_title,
            b"source" => &mut c.source,
            b"year" => &mut c.year,
            b"volume" => &mut c.volume,
            b"issue" => &mut c.issue,
            b"fpage" => &mut c.fpage,
            b"lpage" => &mut c.lpage,
            b"elocation-id" => &mut c.elocation_id,
            b"publisher-name" => &mut c.publisher_name,
            b"publisher-loc" => &mut c.publisher_loc,
            b"edition" => &mut c.edition,
            b"isbn" => &mut c.isbn,
            b"conf-name" => &mut c.conf_name,
            b"pub-id" => match get_attr(&e, b"pub-id-type").as_deref() {
                Some("doi") => &mut c.doi,
                Some("pmid") => &mut c.pmid,
                _ => {
                    let _ = skip_element(reader, name);
                    continue;
                }
            },
            b"person-group" => {
                let group_type = get_attr(&e, b"person-group-type");
                let people = parse_person_group(reader);
                match group_type.as_deref() {
                    None | Some("author") => c.authors.extend(people),
                    Some("editor") => c.editors.extend(people),
                    // compiler, translator, inventor, ...: not modeled
                    Some(_) => {}
                }
                continue;
            }
            // Names outside a `<person-group>` (common in `<mixed-citation>`) are authors.
            b"name" | b"string-name" => {
                if let Some(author) = parse_name(reader, child) {
                    c.authors.push(author);
                }
                continue;
            }
            b"collab" => {
                if let Some(text) = read_field(reader, child) {
                    c.authors.push(Author::collaboration(text));
                }
                continue;
            }
            // Access dates and free-text notes carry `<year>`s and ids that are
            // not the publication's own.
            b"date-in-citation" | b"comment" => {
                let _ = skip_element(reader, name);
                continue;
            }
            // Anything else (inline markup, `<etal>`, `<uri>`, ...): descend.
            _ => continue,
        };

        let value = read_field(reader, child);
        if slot.is_none() {
            *slot = value;
        }
    }

    c
}

/// Parse a `<person-group>`; the reader has just consumed its start tag.
fn parse_person_group(reader: &mut Reader<&[u8]>) -> Vec<Author> {
    let mut people = Vec::new();

    loop {
        let e = match reader.read_event() {
            Ok(Event::Start(e)) => e,
            Ok(Event::End(ref e)) if e.name().as_ref() == b"person-group" => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => continue,
        };
        let name = e.name();
        match name.as_ref() {
            tag @ (b"name" | b"string-name") => {
                if let Some(author) = parse_name(reader, tag) {
                    people.push(author);
                }
            }
            b"collab" => {
                if let Some(text) = read_field(reader, b"collab") {
                    people.push(Author::collaboration(text));
                }
            }
            b"etal" => {
                let _ = skip_element(reader, name);
            }
            _ => {}
        }
    }

    people
}

/// Parse a `<name>` or `<string-name>`; the reader has just consumed its start tag.
///
/// A `<string-name>` without `<surname>`/`<given-names>` children is kept as an
/// unstructured full name.
fn parse_name(reader: &mut Reader<&[u8]>, tag: &[u8]) -> Option<Author> {
    let mut surname = None;
    let mut given_names = None;
    let mut suffix = None;
    let mut loose_text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let child = e.name().as_ref().to_vec();
                let value = read_field(reader, &child);
                match child.as_slice() {
                    b"surname" => surname = surname.or(value),
                    b"given-names" => given_names = given_names.or(value),
                    b"suffix" => suffix = suffix.or(value),
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                if let Ok(text) = t.decode() {
                    loose_text.push_str(&text);
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == tag => break,
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    if surname.is_none() && given_names.is_none() {
        let full_name = collapse_whitespace(&loose_text);
        return (!full_name.is_empty()).then(|| Author::from_full_name(full_name));
    }

    let mut author = Author::new(surname, given_names);
    author.suffix = suffix;
    Some(author)
}

/// Read the text of the element `tag` (reader just past its start tag),
/// collapsing whitespace runs. `None` for empty text.
fn read_field(reader: &mut Reader<&[u8]>, tag: &[u8]) -> Option<String> {
    let text = read_text_content(reader, tag).ok()?;
    let text = collapse_whitespace(&text);
    (!text.is_empty()).then_some(text)
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Format page range from first and last page
fn format_pages(fpage: Option<String>, lpage: Option<String>) -> Option<String> {
    match (fpage, lpage) {
        (Some(f), Some(l)) => Some(format!("{}-{}", f, l)),
        (Some(f), None) => Some(f),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_citation_publisher_editors() {
        let content = r#"
        <ref-list>
            <ref id="b1">
                <element-citation publication-type="book">
                    <person-group person-group-type="author">
                        <name><surname>Smith</surname><given-names>A</given-names></name>
                    </person-group>
                    <person-group person-group-type="editor">
                        <name><surname>Jones</surname><given-names>B</given-names></name>
                    </person-group>
                    <source>Molecular Biology</source>
                    <edition>2nd ed</edition>
                    <publisher-name>Academic Press</publisher-name>
                    <publisher-loc>London</publisher-loc>
                    <isbn>978-0-12-345678-9</isbn>
                    <year>2020</year>
                </element-citation>
            </ref>
        </ref-list>
        "#;
        let refs = extract_references_detailed(content).unwrap();
        assert_eq!(refs.len(), 1);
        let r = &refs[0];
        assert_eq!(r.publication_type.as_deref(), Some("book"));
        assert_eq!(r.source.as_deref(), Some("Molecular Biology"));
        assert_eq!(r.edition.as_deref(), Some("2nd ed"));
        assert_eq!(r.publisher_name.as_deref(), Some("Academic Press"));
        assert_eq!(r.publisher_loc.as_deref(), Some("London"));
        assert_eq!(r.isbn.as_deref(), Some("978-0-12-345678-9"));
        assert_eq!(r.authors.len(), 1);
        assert_eq!(r.authors[0].surname.as_deref(), Some("Smith"));
        assert_eq!(r.editors.len(), 1);
        assert_eq!(r.editors[0].surname.as_deref(), Some("Jones"));
    }

    #[test]
    fn test_extract_references_detailed() {
        let content = r#"
        <ref-list>
            <ref id="ref1">
                <element-citation publication-type="journal">
                    <person-group person-group-type="author">
                        <name>
                            <surname>Smith</surname>
                            <given-names>J</given-names>
                        </name>
                    </person-group>
                    <article-title>Test Article</article-title>
                    <source>Test Journal</source>
                    <year>2023</year>
                    <volume>10</volume>
                    <issue>2</issue>
                    <fpage>123</fpage>
                    <lpage>130</lpage>
                    <pub-id pub-id-type="doi">10.1234/test</pub-id>
                </element-citation>
            </ref>
        </ref-list>
        "#;

        let references = extract_references_detailed(content).unwrap();
        assert_eq!(references.len(), 1);

        let ref1 = &references[0];
        assert_eq!(ref1.id, "ref1");
        assert_eq!(ref1.title, Some("Test Article".to_string()));
        assert_eq!(ref1.source, Some("Test Journal".to_string()));
        assert_eq!(ref1.year, Some("2023".to_string()));
        assert_eq!(ref1.volume, Some("10".to_string()));
        assert_eq!(ref1.issue, Some("2".to_string()));
        assert_eq!(ref1.pages, Some("123-130".to_string()));
        assert_eq!(ref1.doi, Some("10.1234/test".to_string()));
        assert_eq!(ref1.authors.len(), 1);
    }

    #[test]
    fn test_extract_references_no_ref_list() {
        let content = "<article>No references here</article>";
        let references = extract_references_detailed(content).unwrap();
        assert_eq!(references.len(), 0);
    }

    #[test]
    fn test_extract_references_invalid_xml() {
        // The function is designed to be robust and handle malformed XML gracefully
        // by returning an empty vector instead of erroring. This test verifies that behavior.
        let content = "<ref-list><ref>Invalid XML</ref-list>";
        let result = extract_references_detailed(content);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_extract_references_with_comments_and_etal() {
        // Test that the serde structs handle all elements that appear in real PMC XML
        let content = r#"<ref-list id="bibl10"><title>References</title>
<ref id="bib3"><label>3</label><element-citation publication-type="journal" id="sbref30"><person-group person-group-type="author"><name name-style="western"><surname>Alvarez</surname><given-names>C</given-names></name><etal/></person-group><article-title>Test Article</article-title><source>MedRxiv</source><year>2021</year><comment>published online 20.</comment><pub-id pub-id-type="doi">10.1234/test</pub-id><comment>(preprint)</comment><pub-id pub-id-type="pmcid">PMC123</pub-id><pub-id pub-id-type="pmid">123</pub-id></element-citation></ref>
</ref-list>"#;

        let references = extract_references_detailed(content).unwrap();
        assert_eq!(
            references.len(),
            1,
            "Should parse ref with comments and etal"
        );

        let ref3 = &references[0];
        assert_eq!(ref3.id, "bib3");
        assert_eq!(ref3.title, Some("Test Article".to_string()));
        assert_eq!(ref3.source, Some("MedRxiv".to_string()));
        assert_eq!(ref3.authors.len(), 1);
        assert_eq!(ref3.authors[0].surname, Some("Alvarez".to_string()));
    }

    #[test]
    fn test_citation_alternatives_prefers_element_citation() {
        // Springer-style: <citation-alternatives> with both forms, and inline
        // markup carrying attributes (`<italic toggle="yes">`) in the title.
        let content = r#"<ref-list><ref id="CR1"><citation-alternatives><element-citation id="ec-CR1" publication-type="journal"><person-group person-group-type="author"><name name-style="western"><surname>Abe</surname><given-names>F</given-names></name><name name-style="western"><surname>Usui</surname><given-names>K</given-names></name></person-group><article-title>Fluconazole modulates membrane rigidity in <italic toggle="yes">Saccharomyces cerevisiae</italic></article-title><source>Biochemistry</source><year>2009</year><volume>48</volume><issue>36</issue><fpage>8494</fpage><lpage>8504</lpage><pub-id pub-id-type="doi">10.1021/bi900578y</pub-id><pub-id pub-id-type="pmid">19670905</pub-id></element-citation><mixed-citation id="mc-CR1" publication-type="journal">Abe F, Usui K (2009) Fluconazole modulates membrane rigidity. Biochemistry 48(36):8494-8504</mixed-citation></citation-alternatives></ref></ref-list>"#;

        let refs = extract_references_detailed(content).unwrap();
        assert_eq!(refs.len(), 1);
        let r = &refs[0];
        assert_eq!(
            r.title.as_deref(),
            Some("Fluconazole modulates membrane rigidity in Saccharomyces cerevisiae")
        );
        assert_eq!(r.authors.len(), 2);
        assert_eq!(r.pages.as_deref(), Some("8494-8504"));
        assert_eq!(r.doi.as_deref(), Some("10.1021/bi900578y"));
        assert_eq!(r.pmid.as_deref(), Some("19670905"));
    }

    #[test]
    fn test_mixed_citation_with_string_names_and_collab() {
        let content = r#"<ref-list><title>References</title>
<ref id="ref1"><label>1.</label><mixed-citation publication-type="journal" id="r1">
<string-name name-style="western">
<surname>van Oldenborgh</surname>
<given-names>GJ</given-names>
</string-name>, <string-name name-style="western"><surname>Krikken</surname><given-names>F</given-names></string-name>
<etal>et al.</etal> (<year>2021</year>) <article-title>Attribution of the Australian bushfire risk</article-title>. <source>Nat Hazards Earth Syst Sci</source>
<volume>21</volume>, <fpage>941</fpage>&#8211;<lpage>960</lpage>.</mixed-citation></ref>
<ref id="ref2"><mixed-citation publication-type="gov"><collab>National Center for Health Statistics</collab>; <collab>CDC</collab>. <source>Survey</source>. <comment>Accessed <year>2023</year></comment></mixed-citation></ref>
</ref-list>"#;

        let refs = extract_references_detailed(content).unwrap();
        assert_eq!(refs.len(), 2);

        let r1 = &refs[0];
        let names: Vec<&str> = r1.authors.iter().map(|a| a.full_name.as_str()).collect();
        assert_eq!(names, ["GJ van Oldenborgh", "F Krikken"]);
        assert_eq!(r1.year.as_deref(), Some("2021"));
        assert_eq!(r1.pages.as_deref(), Some("941-960"));

        let r2 = &refs[1];
        assert_eq!(r2.authors.len(), 2);
        assert!(r2.authors.iter().all(|a| a.is_collaboration()));
        // `<year>` inside `<comment>` is an access date, not the publication year.
        assert_eq!(r2.year, None);
    }

    #[test]
    fn test_one_bad_ref_does_not_drop_the_list_and_nested_lists_are_read() {
        let content = r#"<back><sec><title>Further Reading</title><ref-list>
<ref id="a"><element-citation publication-type="journal"><article-title>First</article-title><comment>x</comment><comment>y</comment></element-citation></ref>
<ref id="b"><mixed-citation>Unstructured citation text only.</mixed-citation></ref>
<ref id="c"><element-citation publication-type="book"><chapter-title>A chapter</chapter-title><source>A book</source></element-citation></ref>
</ref-list></sec></back>"#;

        let refs = extract_references_detailed(content).unwrap();
        let ids: Vec<&str> = refs.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["a", "b", "c"]);
        assert_eq!(refs[0].title.as_deref(), Some("First"));
        assert_eq!(refs[2].title.as_deref(), Some("A chapter"));
    }
}
