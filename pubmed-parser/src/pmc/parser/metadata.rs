use crate::common::{HistoryDate, PublicationDate};
use crate::pmc::domain::{
    FundingInfo, JournalMeta, KeywordGroup, RelatedArticle, SubjectGroup, SupplementaryMaterial,
};

use quick_xml::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;
use tracing::warn;

use super::reader_utils::{get_attr, make_reader, read_text_content, skip_element};
use super::xml_utils::decode_xml_entities;

enum TextAction {
    Read(Vec<u8>),
    ReadSkip(Vec<u8>),
    Continue,
    Break,
}

fn read_first_text(content: &str, tag: &[u8]) -> Option<String> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let action = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == tag => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) => {
                return read_text_content(&mut reader, &name, &mut buf)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
            }
            TextAction::ReadSkip(name) => {
                let _ = skip_element(&mut reader, QName(&name), &mut buf);
            }
            TextAction::Break => return None,
            TextAction::Continue => {}
        }
    }
}

fn read_texts_within_parent(content: &str, parent: &[u8], child: &[u8]) -> Vec<String> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();
    let mut parent_depth = 0_u32;
    let mut values = Vec::new();

    loop {
        let action = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == parent => {
                parent_depth += 1;
                TextAction::Continue
            }
            Ok(Event::Start(ref e)) if parent_depth > 0 && e.name().as_ref() == child => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == parent => {
                parent_depth = parent_depth.saturating_sub(1);
                TextAction::Continue
            }
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) => {
                if let Ok(text) = read_text_content(&mut reader, &name, &mut buf) {
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        values.push(text);
                    }
                }
            }
            TextAction::ReadSkip(name) => {
                let _ = skip_element(&mut reader, QName(&name), &mut buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    values
}

fn read_first_text_matching_attr(
    content: &str,
    tag: &[u8],
    attr: &[u8],
    value: &str,
) -> Option<String> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let action = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e))
                if e.name().as_ref() == tag && get_attr(e, attr).as_deref() == Some(value) =>
            {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == tag => {
                let name = e.name().as_ref().to_vec();
                TextAction::ReadSkip(name)
            }
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) => {
                return read_text_content(&mut reader, &name, &mut buf)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
            }
            TextAction::ReadSkip(name) => {
                let _ = skip_element(&mut reader, QName(&name), &mut buf);
            }
            TextAction::Break => return None,
            TextAction::Continue => {}
        }
    }
}

fn read_first_attr(content: &str, tag: &[u8], attrs: &[&[u8]]) -> Option<String> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let result = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == tag => {
                attrs.iter().find_map(|attr| get_attr(e, attr))
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => None,
        };
        buf.clear();

        if result.is_some() {
            return result;
        }
    }
}

fn parse_u16(text: String) -> Option<u16> {
    text.parse::<u16>().ok()
}

fn parse_u8(text: String) -> Option<u8> {
    text.parse::<u8>().ok()
}

fn read_date_parts(
    reader: &mut Reader<&[u8]>,
    parent_tag: &[u8],
    buf: &mut Vec<u8>,
) -> (Option<u16>, Option<u8>, Option<u8>) {
    let mut year = None;
    let mut month = None;
    let mut day = None;

    loop {
        let action = match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"year" | b"month" | b"day" => TextAction::Read(e.name().as_ref().to_vec()),
                other => TextAction::ReadSkip(other.to_vec()),
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == parent_tag => TextAction::Break,
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) if name == b"year" => {
                year = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(parse_u16);
            }
            TextAction::Read(name) if name == b"month" => {
                month = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(parse_u8);
            }
            TextAction::Read(name) if name == b"day" => {
                day = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(parse_u8);
            }
            TextAction::Read(name) | TextAction::ReadSkip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    (year, month, day)
}

fn clean_text(text: String) -> Option<String> {
    let text = text.trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

fn read_award_group(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
) -> (Option<String>, Option<String>) {
    let mut source = None;
    let mut award_id = None;

    loop {
        let action = match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"funding-source" => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"award-id" => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) => TextAction::ReadSkip(e.name().as_ref().to_vec()),
            Ok(Event::End(ref e)) if e.name().as_ref() == b"award-group" => TextAction::Break,
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) if name.as_slice() == b"funding-source" => {
                source = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(clean_text);
            }
            TextAction::Read(name) if name.as_slice() == b"award-id" => {
                award_id = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(clean_text);
            }
            TextAction::Read(name) | TextAction::ReadSkip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    if source.is_none() {
        warn!("funding award-group missing funding-source element");
    }
    (source, award_id)
}

fn read_funding_group(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Vec<FundingInfo> {
    let mut statement = None;
    let mut awards = Vec::new();

    loop {
        let action = match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"funding-statement" => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"award-group" => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) => TextAction::ReadSkip(e.name().as_ref().to_vec()),
            Ok(Event::End(ref e)) if e.name().as_ref() == b"funding-group" => TextAction::Break,
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) if name.as_slice() == b"funding-statement" => {
                statement = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(clean_text);
            }
            TextAction::Read(name) if name.as_slice() == b"award-group" => {
                let (source, award_id) = read_award_group(reader, buf);
                awards.push((source, award_id));
            }
            TextAction::Read(name) | TextAction::ReadSkip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    awards
        .into_iter()
        .map(|(source, award_id)| FundingInfo {
            source,
            award_id,
            statement: statement.clone(),
        })
        .collect()
}

#[derive(Default)]
struct SectionParts {
    title: Option<String>,
    paragraphs: Vec<String>,
    text_parts: Vec<String>,
}

fn read_section_parts(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> SectionParts {
    let mut parts = SectionParts::default();

    loop {
        let action = match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => match e.name().as_ref() {
                b"title" | b"p" | b"sec" => TextAction::Read(e.name().as_ref().to_vec()),
                other => TextAction::ReadSkip(other.to_vec()),
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == b"sec" => TextAction::Break,
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) if name.as_slice() == b"title" => {
                if let Some(text) = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(clean_text)
                {
                    if parts.title.is_none() {
                        parts.title = Some(text.clone());
                    }
                    parts.text_parts.push(text);
                }
            }
            TextAction::Read(name) if name.as_slice() == b"p" => {
                if let Some(text) = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(clean_text)
                {
                    parts.paragraphs.push(text.clone());
                    parts.text_parts.push(text);
                }
            }
            TextAction::Read(name) if name.as_slice() == b"sec" => {
                let nested = read_section_parts(reader, buf);
                parts.paragraphs.extend(nested.paragraphs);
                parts.text_parts.extend(nested.text_parts);
            }
            TextAction::Read(name) | TextAction::ReadSkip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    parts
}

fn read_caption(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>) -> Option<String> {
    let mut title = None;
    let mut text_parts = Vec::new();

    loop {
        let action = match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => TextAction::Read(e.name().as_ref().to_vec()),
            Ok(Event::Text(ref e)) => {
                if let Ok(text) = e.unescape()
                    && let Some(text) = clean_text(text.into_owned())
                {
                    text_parts.push(text);
                }
                TextAction::Continue
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"caption" => TextAction::Break,
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) if name.as_slice() == b"title" => {
                let text = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(clean_text);
                if title.is_none() {
                    title = text.clone();
                }
                if let Some(text) = text {
                    text_parts.push(text);
                }
            }
            TextAction::Read(name) => {
                if let Some(text) = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(clean_text)
                {
                    text_parts.push(text);
                }
            }
            TextAction::ReadSkip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    title.or_else(|| clean_text(text_parts.join(" ")))
}

fn read_media(reader: &mut Reader<&[u8]>, buf: &mut Vec<u8>, caption: &mut Option<String>) {
    loop {
        let action = match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"caption" => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) => TextAction::ReadSkip(e.name().as_ref().to_vec()),
            Ok(Event::End(ref e)) if e.name().as_ref() == b"media" => TextAction::Break,
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) if name.as_slice() == b"caption" => {
                if caption.is_none() {
                    *caption = read_caption(reader, buf);
                } else {
                    let _ = skip_element(reader, QName(&name), buf);
                }
            }
            TextAction::Read(name) | TextAction::ReadSkip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }
}

fn read_supplementary_material(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    id: Option<String>,
    content_type: Option<String>,
    href: Option<String>,
    fallback_id: String,
) -> SupplementaryMaterial {
    let mut label = None;
    let mut caption = None;
    let mut href = href;

    loop {
        let action = match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"label" => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"caption" => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"media" => {
                if href.is_none() {
                    href = get_attr(e, b"xlink:href").or_else(|| get_attr(e, b"href"));
                }
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) => TextAction::ReadSkip(e.name().as_ref().to_vec()),
            Ok(Event::End(ref e)) if e.name().as_ref() == b"supplementary-material" => {
                TextAction::Break
            }
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) if name.as_slice() == b"label" => {
                label = read_text_content(reader, &name, buf)
                    .ok()
                    .and_then(clean_text);
            }
            TextAction::Read(name) if name.as_slice() == b"caption" => {
                caption = read_caption(reader, buf);
            }
            TextAction::Read(name) if name.as_slice() == b"media" => {
                read_media(reader, buf, &mut caption);
            }
            TextAction::Read(name) | TextAction::ReadSkip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    SupplementaryMaterial {
        id: id.unwrap_or(fallback_id),
        content_type,
        title: caption,
        description: label,
        href,
    }
}

/// Extract journal metadata.
///
/// Returns a [`JournalMeta`] without volume/issue — those belong to the article level
/// per the JATS DTD and are extracted separately via [`extract_volume`] / [`extract_issue`].
pub(crate) fn extract_journal_info(content: &str) -> JournalMeta {
    let title = read_first_text(content, b"journal-title");
    let abbreviation =
        read_first_text_matching_attr(content, b"journal-id", b"journal-id-type", "iso-abbrev");

    let mut issn_print = None;
    let mut issn_electronic = None;

    let mut reader = make_reader(content);
    let mut buf = Vec::new();
    loop {
        let action = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"issn" => {
                let pub_type = get_attr(e, b"pub-type");
                Some((e.name().as_ref().to_vec(), pub_type))
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => None,
        };
        buf.clear();

        if let Some((name, pub_type)) = action
            && let Ok(value) = read_text_content(&mut reader, &name, &mut buf)
            && let Some(value) = clean_text(value)
        {
            match pub_type.as_deref() {
                Some("epub") => issn_electronic = Some(value),
                Some("ppub") => issn_print = Some(value),
                _ => {}
            }
        }
    }

    let publisher = read_first_text(content, b"publisher-name");

    JournalMeta {
        title,
        abbreviation,
        issn_print,
        issn_electronic,
        publisher,
    }
}

/// Extract volume number from `<volume>` element.
pub(crate) fn extract_volume(content: &str) -> Option<String> {
    read_first_text(content, b"volume")
}

/// Extract issue number from `<issue>` element.
pub(crate) fn extract_issue(content: &str) -> Option<String> {
    read_first_text(content, b"issue")
}

/// Extract structured publication dates from `<pub-date>` elements.
///
/// Returns a `Vec<PublicationDate>` with `pub_type` attribute preserved.
pub(crate) fn extract_pub_dates(content: &str) -> Vec<PublicationDate> {
    let mut dates = Vec::new();
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let pub_type = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"pub-date" => {
                get_attr(e, b"pub-type").or_else(|| get_attr(e, b"date-type"))
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {
                buf.clear();
                continue;
            }
        };
        buf.clear();

        let (year, month, day) = read_date_parts(&mut reader, b"pub-date", &mut buf);
        dates.push(PublicationDate {
            pub_type,
            year,
            month,
            day,
        });
    }

    dates
}

/// Extract DOI from article metadata
pub(crate) fn extract_doi(content: &str) -> Option<String> {
    read_first_text_matching_attr(content, b"article-id", b"pub-id-type", "doi")
}

/// Extract PMID from article metadata
pub(crate) fn extract_pmid(content: &str) -> Option<String> {
    read_first_text_matching_attr(content, b"article-id", b"pub-id-type", "pmid")
}

/// Extract article type from article metadata
pub(crate) fn extract_article_type(content: &str) -> Option<String> {
    read_first_attr(content, b"article", &[b"article-type"])
        .or_else(|| read_first_text(content, b"subject"))
}

/// Extract keywords from article metadata
pub(crate) fn extract_keywords(content: &str) -> Vec<String> {
    read_texts_within_parent(content, b"kwd-group", b"kwd")
}

/// Read child `<{child}>` texts until the current `<{group}>` element closes
/// (depth-aware, so nested same-name groups roll up into the enclosing one).
/// The reader must have just consumed the opening `<{group}>` start tag.
fn read_group_child_texts(
    reader: &mut Reader<&[u8]>,
    buf: &mut Vec<u8>,
    group: &[u8],
    child: &[u8],
) -> Vec<String> {
    let mut depth = 1_u32;
    let mut values = Vec::new();

    loop {
        let action = match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == group => {
                depth += 1;
                TextAction::Continue
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == child => {
                TextAction::Read(child.to_vec())
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == group => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    TextAction::Break
                } else {
                    TextAction::Continue
                }
            }
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) => {
                if let Ok(text) = read_text_content(reader, &name, buf)
                    && !text.is_empty()
                {
                    values.push(text);
                }
            }
            TextAction::ReadSkip(name) => {
                let _ = skip_element(reader, QName(&name), buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    values
}

/// Extract structured keyword groups, preserving `kwd-group-type` and
/// `xml:lang`. From `<kwd-group>/<kwd>`.
pub(crate) fn extract_keyword_groups(content: &str) -> Vec<KeywordGroup> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();
    let mut groups = Vec::new();

    loop {
        let attrs = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"kwd-group" => {
                Some((get_attr(e, b"kwd-group-type"), get_attr(e, b"xml:lang")))
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => None,
        };
        buf.clear();

        if let Some((group_type, lang)) = attrs {
            let keywords = read_group_child_texts(&mut reader, &mut buf, b"kwd-group", b"kwd");
            if !keywords.is_empty() {
                groups.push(KeywordGroup {
                    group_type,
                    lang,
                    keywords,
                });
            }
        }
    }

    groups
}

/// Extract structured subject groups, preserving `subj-group-type`. From
/// `<article-categories>/<subj-group>/<subject>`.
pub(crate) fn extract_subject_groups(content: &str) -> Vec<SubjectGroup> {
    let Some(start) = content.find("<article-categories") else {
        return Vec::new();
    };
    let Some(end) = content[start..].find("</article-categories>") else {
        return Vec::new();
    };
    let cats = &content[start..start + end + "</article-categories>".len()];

    let mut reader = make_reader(cats);
    let mut buf = Vec::new();
    let mut groups = Vec::new();

    loop {
        let group_type = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"subj-group" => {
                Some(get_attr(e, b"subj-group-type"))
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => None,
        };
        buf.clear();

        if let Some(group_type) = group_type {
            let subjects = read_group_child_texts(&mut reader, &mut buf, b"subj-group", b"subject");
            if !subjects.is_empty() {
                groups.push(SubjectGroup {
                    group_type,
                    subjects,
                });
            }
        }
    }

    groups
}

/// Extract related-article links (corrections, retractions, companions, …).
/// From `<related-article>` attributes.
pub(crate) fn extract_related_articles(content: &str) -> Vec<RelatedArticle> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();
    let mut out = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"related-article" => {
                out.push(RelatedArticle {
                    related_article_type: get_attr(e, b"related-article-type"),
                    ext_link_type: get_attr(e, b"ext-link-type"),
                    href: get_attr(e, b"xlink:href").or_else(|| get_attr(e, b"href")),
                    id: get_attr(e, b"id"),
                });
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    out
}

/// Extract author notes (`<corresp>` / `<fn>` text) from `<author-notes>`.
pub(crate) fn extract_author_notes(content: &str) -> Vec<String> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();
    let mut notes = Vec::new();
    let mut in_notes = 0_u32;

    loop {
        let action = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"author-notes" => {
                in_notes += 1;
                TextAction::Continue
            }
            Ok(Event::Start(ref e))
                if in_notes > 0 && matches!(e.name().as_ref(), b"corresp" | b"fn") =>
            {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"author-notes" => {
                in_notes = in_notes.saturating_sub(1);
                TextAction::Continue
            }
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) => {
                if let Ok(text) = read_text_content(&mut reader, &name, &mut buf)
                    && !text.is_empty()
                {
                    notes.push(text);
                }
            }
            TextAction::ReadSkip(name) => {
                let _ = skip_element(&mut reader, QName(&name), &mut buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    notes
}

/// Extract funding information
pub(crate) fn extract_funding(content: &str) -> Vec<FundingInfo> {
    let mut funding = Vec::new();
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let is_group = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"funding-group" => true,
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => false,
        };
        buf.clear();

        if is_group {
            funding.extend(read_funding_group(&mut reader, &mut buf));
        }
    }

    funding
}

/// Extract conflict of interest statement
pub(crate) fn extract_conflict_of_interest(content: &str) -> Option<String> {
    for text in read_texts_within_parent(content, b"fn-group", b"fn") {
        let lower = text.to_lowercase();
        if lower.contains("conflict") || lower.contains("competing") {
            return Some(text);
        }
    }

    let mut reader = make_reader(content);
    let mut buf = Vec::new();
    loop {
        let is_section = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"sec" => true,
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => false,
        };
        buf.clear();

        if is_section {
            let parts = read_section_parts(&mut reader, &mut buf);
            if let Some(title) = parts.title {
                let lower = title.to_lowercase();
                if lower.contains("conflict") || lower.contains("competing") {
                    return clean_text(parts.paragraphs.join(" "))
                        .or_else(|| clean_text(parts.text_parts.join(" ")));
                }
            }
        }
    }

    None
}

/// Extract acknowledgments
///
/// Strips XML tags and decodes XML entities (e.g., `&#231;` → `ç`).
pub(crate) fn extract_acknowledgments(content: &str) -> Option<String> {
    read_first_text(content, b"ack").map(|s| decode_xml_entities(&s).into_owned())
}

/// Extract data availability statement
pub(crate) fn extract_data_availability(content: &str) -> Option<String> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let action = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"sec" => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"supplementary-material" => {
                TextAction::Read(e.name().as_ref().to_vec())
            }
            Ok(Event::Eof) => TextAction::Break,
            Err(_) => TextAction::Break,
            _ => TextAction::Continue,
        };
        buf.clear();

        match action {
            TextAction::Read(name) if name.as_slice() == b"sec" => {
                let parts = read_section_parts(&mut reader, &mut buf);
                if let Some(text) = clean_text(parts.text_parts.join(" ")) {
                    let lower = text.to_lowercase();
                    if lower.contains("data") && lower.contains("availab") {
                        return Some(text);
                    }
                }
            }
            TextAction::Read(name) if name.as_slice() == b"supplementary-material" => {
                if let Ok(text) = read_text_content(&mut reader, &name, &mut buf)
                    && let Some(text) = clean_text(text)
                {
                    let lower = text.to_lowercase();
                    if lower.contains("data") && lower.contains("availab") {
                        return Some(text);
                    }
                }
            }
            TextAction::Read(name) | TextAction::ReadSkip(name) => {
                let _ = skip_element(&mut reader, QName(&name), &mut buf);
            }
            TextAction::Break => break,
            TextAction::Continue => {}
        }
    }

    None
}

/// Extract supplementary materials
pub(crate) fn extract_supplementary_materials(content: &str) -> Vec<SupplementaryMaterial> {
    let mut materials = Vec::new();
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let attrs = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"supplementary-material" => Some((
                get_attr(e, b"id"),
                get_attr(e, b"content-type"),
                get_attr(e, b"xlink:href").or_else(|| get_attr(e, b"href")),
            )),
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => None,
        };
        buf.clear();

        if let Some((id, content_type, href)) = attrs {
            let supp_num = materials.len() + 1;
            let fallback_id = format!("supp_{supp_num}");
            materials.push(read_supplementary_material(
                &mut reader,
                &mut buf,
                id,
                content_type,
                href,
                fallback_id,
            ));
        }
    }

    materials
}

/// Extract article title. Returns `None` when the element is absent.
pub(crate) fn extract_title(content: &str) -> Option<String> {
    read_first_text(content, b"article-title")
}

/// Extract article subtitle from `<title-group>/<subtitle>`
pub(crate) fn extract_subtitle(content: &str) -> Option<String> {
    read_texts_within_parent(content, b"title-group", b"subtitle")
        .into_iter()
        .next()
}

/// Extract copyright information
///
/// Decodes XML entities (e.g., `&#169;` → `©`).
pub(crate) fn extract_copyright(content: &str) -> Option<String> {
    read_first_text(content, b"copyright-statement")
        .or_else(|| read_first_text(content, b"copyright-year"))
        .map(|s| decode_xml_entities(&s).into_owned())
}

/// Extract license information
pub(crate) fn extract_license(content: &str) -> Option<String> {
    read_first_text(content, b"license")
}

/// Extract abstract text from article metadata
///
/// Handles both simple abstracts (`<abstract><p>...</p></abstract>`)
/// and structured abstracts with sections (`<abstract><sec><title>Background</title><p>...</p></sec>...</abstract>`).
pub(crate) fn extract_abstract(content: &str) -> Option<String> {
    let mut reader = make_reader(content);
    let mut buf = Vec::new();

    loop {
        let found = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"abstract" => true,
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => false,
        };
        buf.clear();

        if !found {
            continue;
        }

        let mut paragraphs = Vec::new();
        let mut fallback_parts = Vec::new();
        loop {
            let action = match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"p" => {
                    TextAction::Read(e.name().as_ref().to_vec())
                }
                Ok(Event::Text(ref e)) => {
                    if let Ok(text) = e.unescape()
                        && let Some(text) = clean_text(text.into_owned())
                    {
                        fallback_parts.push(text);
                    }
                    TextAction::Continue
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"abstract" => TextAction::Break,
                Ok(Event::Eof) => TextAction::Break,
                Err(_) => TextAction::Break,
                _ => TextAction::Continue,
            };
            buf.clear();

            match action {
                TextAction::Read(name) => {
                    if let Some(text) = read_text_content(&mut reader, &name, &mut buf)
                        .ok()
                        .and_then(clean_text)
                    {
                        paragraphs.push(text);
                    }
                }
                TextAction::ReadSkip(name) => {
                    let _ = skip_element(&mut reader, QName(&name), &mut buf);
                }
                TextAction::Break => {
                    let text = if paragraphs.is_empty() {
                        fallback_parts.join(" ")
                    } else {
                        paragraphs.join(" ")
                    };
                    return clean_text(text);
                }
                TextAction::Continue => {}
            }
        }
    }
}

/// Extract publication history dates from `<history>` element
///
/// Parses `<date date-type="received">`, `<date date-type="accepted">`, etc.
pub(crate) fn extract_history_dates(content: &str) -> Vec<HistoryDate> {
    let mut dates = Vec::new();
    let mut reader = make_reader(content);
    let mut buf = Vec::new();
    let mut in_history = false;

    loop {
        let date_type = match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"history" => {
                in_history = true;
                None
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"history" => {
                in_history = false;
                None
            }
            Ok(Event::Start(ref e)) if in_history && e.name().as_ref() == b"date" => {
                get_attr(e, b"date-type")
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => None,
        };
        buf.clear();

        if let Some(date_type) = date_type {
            let (year, month, day) = read_date_parts(&mut reader, b"date", &mut buf);
            dates.push(HistoryDate {
                date_type,
                year,
                month,
                day,
            });
        }
    }

    dates
}

/// Extract article categories from `<article-categories>/<subj-group>/<subject>`
pub(crate) fn extract_categories(content: &str) -> Vec<String> {
    read_texts_within_parent(content, b"article-categories", b"subject")
}

/// Extract license URL from `<license xlink:href="...">` attribute
/// or from `<ali:license_ref>` element content
pub(crate) fn extract_license_url(content: &str) -> Option<String> {
    read_first_attr(content, b"license", &[b"xlink:href", b"href"])
        .or_else(|| read_first_text(content, b"ali:license_ref"))
}

/// Extract first page number from `<fpage>` element
///
/// Handles `<fpage>` with or without attributes (e.g., `<fpage seq="b">54</fpage>`).
pub(crate) fn extract_fpage(content: &str) -> Option<String> {
    read_first_text(content, b"fpage")
}

/// Extract last page number from `<lpage>` element
///
/// Handles `<lpage>` with or without attributes.
pub(crate) fn extract_lpage(content: &str) -> Option<String> {
    read_first_text(content, b"lpage")
}

/// Extract electronic location identifier from `<elocation-id>` element
pub(crate) fn extract_elocation_id(content: &str) -> Option<String> {
    read_first_text(content, b"elocation-id")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keyword_groups_with_type_and_lang() {
        let content = r#"
        <article-meta>
            <kwd-group kwd-group-type="author" xml:lang="en">
                <kwd>genomics</kwd>
                <kwd>RNA-seq</kwd>
            </kwd-group>
            <kwd-group>
                <kwd>plain</kwd>
            </kwd-group>
        </article-meta>"#;
        let groups = extract_keyword_groups(content);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].group_type.as_deref(), Some("author"));
        assert_eq!(groups[0].lang.as_deref(), Some("en"));
        assert_eq!(groups[0].keywords, vec!["genomics", "RNA-seq"]);
        assert_eq!(groups[1].group_type, None);
        assert_eq!(groups[1].keywords, vec!["plain"]);
        // Flattened view still works and is consistent.
        assert_eq!(
            extract_keywords(content),
            vec!["genomics", "RNA-seq", "plain"]
        );
    }

    #[test]
    fn test_extract_subject_groups_with_type() {
        let content = r#"
        <article-categories>
            <subj-group subj-group-type="heading">
                <subject>Research Article</subject>
            </subj-group>
            <subj-group subj-group-type="discipline">
                <subject>Microbiology</subject>
                <subject>Virology</subject>
            </subj-group>
        </article-categories>"#;
        let groups = extract_subject_groups(content);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].group_type.as_deref(), Some("heading"));
        assert_eq!(groups[0].subjects, vec!["Research Article"]);
        assert_eq!(groups[1].group_type.as_deref(), Some("discipline"));
        assert_eq!(groups[1].subjects, vec!["Microbiology", "Virology"]);
    }

    #[test]
    fn test_extract_related_articles() {
        let content = r#"
        <article-meta>
            <related-article related-article-type="corrected-article"
                ext-link-type="doi" xlink:href="10.1/orig" id="d1"/>
        </article-meta>"#;
        let related = extract_related_articles(content);
        assert_eq!(related.len(), 1);
        assert_eq!(
            related[0].related_article_type.as_deref(),
            Some("corrected-article")
        );
        assert_eq!(related[0].ext_link_type.as_deref(), Some("doi"));
        assert_eq!(related[0].href.as_deref(), Some("10.1/orig"));
        assert_eq!(related[0].id.as_deref(), Some("d1"));
        assert!(related[0].is_correction());
    }

    #[test]
    fn test_extract_author_notes() {
        let content = r#"
        <article-meta>
            <author-notes>
                <corresp id="c1">Correspondence to: jane@example.org</corresp>
                <fn fn-type="equal"><p>These authors contributed equally.</p></fn>
            </author-notes>
        </article-meta>"#;
        let notes = extract_author_notes(content);
        assert_eq!(notes.len(), 2);
        assert!(notes[0].contains("jane@example.org"));
        assert!(notes[1].contains("contributed equally"));
    }

    #[test]
    fn test_extract_title() {
        let content = r#"<article-title>Test Article Title</article-title>"#;
        let title = extract_title(content);
        assert_eq!(title.as_deref(), Some("Test Article Title"));
    }

    #[test]
    fn test_extract_title_missing() {
        let content = r#"<body>No title here</body>"#;
        assert_eq!(extract_title(content), None);
    }

    #[test]
    fn test_extract_doi() {
        let content = r#"<article-id pub-id-type="doi">10.1234/test.doi</article-id>"#;
        let doi = extract_doi(content);
        assert_eq!(doi, Some("10.1234/test.doi".to_string()));
    }

    #[test]
    fn test_extract_pmid() {
        let content = r#"<article-id pub-id-type="pmid">12345678</article-id>"#;
        let pmid = extract_pmid(content);
        assert_eq!(pmid, Some("12345678".to_string()));
    }

    #[test]
    fn test_extract_keywords() {
        let content = r#"
        <kwd-group>
            <kwd>keyword1</kwd>
            <kwd>keyword2</kwd>
            <kwd>keyword3</kwd>
        </kwd-group>
        "#;

        let keywords = extract_keywords(content);
        assert_eq!(keywords, vec!["keyword1", "keyword2", "keyword3"]);
    }

    #[test]
    fn test_extract_keywords_with_nested_tags() {
        let content = r#"
        <kwd-group>
            <kwd><italic toggle="yes">Prevotella copri</italic></kwd>
            <kwd>normal keyword</kwd>
            <kwd><bold>important</bold> keyword</kwd>
        </kwd-group>
        "#;

        let keywords = extract_keywords(content);
        assert_eq!(
            keywords,
            vec!["Prevotella copri", "normal keyword", "important keyword"]
        );
    }

    #[test]
    fn test_extract_article_type() {
        let content = r#"<article article-type="research-article">Content</article>"#;
        let article_type = extract_article_type(content);
        assert_eq!(article_type, Some("research-article".to_string()));
    }

    #[test]
    fn test_extract_acknowledgments() {
        let content = r#"<ack><p>We thank the contributors for their valuable input.</p></ack>"#;
        let ack = extract_acknowledgments(content);
        assert_eq!(
            ack,
            Some("We thank the contributors for their valuable input.".to_string())
        );
    }

    #[test]
    fn test_extract_abstract_simple() {
        let content = r#"<abstract><p>This is a simple abstract.</p></abstract>"#;
        let result = extract_abstract(content);
        assert_eq!(result, Some("This is a simple abstract.".to_string()));
    }

    #[test]
    fn test_extract_abstract_structured() {
        let content = r#"
        <abstract id="Abs1">
            <sec>
                <title>Background</title>
                <p>Background text.</p>
            </sec>
            <sec>
                <title>Methods</title>
                <p>Methods text.</p>
            </sec>
        </abstract>
        "#;
        let result = extract_abstract(content);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("Background text."));
        assert!(text.contains("Methods text."));
    }

    #[test]
    fn test_extract_abstract_with_attributes() {
        let content = r#"<abstract><p id="Par1">Text with id attribute.</p></abstract>"#;
        let result = extract_abstract(content);
        assert_eq!(result, Some("Text with id attribute.".to_string()));
    }

    #[test]
    fn test_extract_abstract_missing() {
        let content = r#"<title>No abstract here</title>"#;
        let result = extract_abstract(content);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_history_dates() {
        let content = r#"
        <history>
            <date date-type="received">
                <day>21</day>
                <month>2</month>
                <year>2019</year>
            </date>
            <date date-type="accepted">
                <day>23</day>
                <month>4</month>
                <year>2019</year>
            </date>
        </history>
        "#;
        let dates = extract_history_dates(content);
        assert_eq!(dates.len(), 2);

        assert_eq!(dates[0].date_type, "received");
        assert_eq!(dates[0].year, Some(2019));
        assert_eq!(dates[0].month, Some(2));
        assert_eq!(dates[0].day, Some(21));

        assert_eq!(dates[1].date_type, "accepted");
        assert_eq!(dates[1].year, Some(2019));
        assert_eq!(dates[1].month, Some(4));
        assert_eq!(dates[1].day, Some(23));
    }

    #[test]
    fn test_extract_history_dates_compact() {
        let content = r#"
        <history>
<date date-type="received"><day>09</day><month>5</month><year>2023</year></date>
<date date-type="accepted"><day>29</day><month>6</month><year>2023</year></date>
</history>
        "#;
        let dates = extract_history_dates(content);
        assert_eq!(dates.len(), 2);
        assert_eq!(dates[0].date_type, "received");
        assert_eq!(dates[0].year, Some(2023));
        assert_eq!(dates[0].month, Some(5));
        assert_eq!(dates[0].day, Some(9));
    }

    #[test]
    fn test_extract_history_dates_missing() {
        let content = r#"<article-meta><title>No history</title></article-meta>"#;
        let dates = extract_history_dates(content);
        assert!(dates.is_empty());
    }

    #[test]
    fn test_extract_categories() {
        let content = r#"
        <article-categories>
            <subj-group subj-group-type="heading">
                <subject>Original Article</subject>
            </subj-group>
        </article-categories>
        "#;
        let categories = extract_categories(content);
        assert_eq!(categories, vec!["Original Article"]);
    }

    #[test]
    fn test_extract_categories_multiple() {
        let content = r#"
        <article-categories>
            <subj-group subj-group-type="heading">
                <subject>Research Article</subject>
            </subj-group>
            <subj-group subj-group-type="discipline">
                <subject>Biology</subject>
                <subject>Medicine</subject>
            </subj-group>
        </article-categories>
        "#;
        let categories = extract_categories(content);
        assert_eq!(categories.len(), 3);
        assert!(categories.contains(&"Research Article".to_string()));
        assert!(categories.contains(&"Biology".to_string()));
        assert!(categories.contains(&"Medicine".to_string()));
    }

    #[test]
    fn test_extract_categories_missing() {
        let content = r#"<title>No categories</title>"#;
        let categories = extract_categories(content);
        assert!(categories.is_empty());
    }

    #[test]
    fn test_extract_license_url() {
        let content = r#"<license license-type="open-access" xlink:href="http://creativecommons.org/licenses/by-nc-nd/3.0/"><license-p>Text</license-p></license>"#;
        let url = extract_license_url(content);
        assert_eq!(
            url,
            Some("http://creativecommons.org/licenses/by-nc-nd/3.0/".to_string())
        );
    }

    #[test]
    fn test_extract_license_url_missing() {
        let content = r#"<license><license-p>No URL</license-p></license>"#;
        let url = extract_license_url(content);
        assert!(url.is_none());
    }

    #[test]
    fn test_extract_fpage_lpage() {
        let content = r#"<fpage>1865</fpage><lpage>1868</lpage>"#;
        assert_eq!(extract_fpage(content), Some("1865".to_string()));
        assert_eq!(extract_lpage(content), Some("1868".to_string()));
    }

    #[test]
    fn test_extract_elocation_id() {
        let content = r#"<elocation-id>e12345</elocation-id>"#;
        assert_eq!(extract_elocation_id(content), Some("e12345".to_string()));
    }

    #[test]
    fn test_extract_elocation_id_missing() {
        let content = r#"<fpage>100</fpage>"#;
        assert!(extract_elocation_id(content).is_none());
    }
}
