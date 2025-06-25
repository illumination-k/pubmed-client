use crate::error::{PubMedError, Result};
use crate::pubmed::models::PubMedArticle;
use quick_xml::Reader;
use quick_xml::events::Event;
use std::io::BufReader;
use tracing::{debug, instrument};

pub struct PubMedXmlParser;

impl PubMedXmlParser {
    /// Parse article from EFetch XML response
    #[instrument(skip(xml), fields(pmid = %pmid, xml_size = xml.len()))]
    pub fn parse_article_from_xml(xml: &str, pmid: &str) -> Result<PubMedArticle> {
        let mut reader = Reader::from_reader(BufReader::new(xml.as_bytes()));
        reader.config_mut().trim_text(true);

        let mut title = String::new();
        let mut authors = Vec::new();
        let mut journal = String::new();
        let mut pub_date = String::new();
        let doi = None;
        let mut abstract_text = None;
        let mut article_types = Vec::new();

        let mut buf = Vec::new();
        let mut in_article_title = false;
        let mut in_abstract = false;
        let mut in_abstract_text = false;
        let mut in_journal_title = false;
        let mut in_pub_date = false;
        let mut in_author_list = false;
        let mut in_author = false;
        let mut in_last_name = false;
        let mut in_fore_name = false;
        let mut in_publication_type = false;
        let mut current_author_last = String::new();
        let mut current_author_fore = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        b"ArticleTitle" => in_article_title = true,
                        b"Abstract" => in_abstract = true,
                        b"AbstractText" => in_abstract_text = true,
                        b"Title" if !in_article_title => in_journal_title = true,
                        b"PubDate" => in_pub_date = true,
                        b"AuthorList" => in_author_list = true,
                        b"Author" if in_author_list => {
                            in_author = true;
                            current_author_last.clear();
                            current_author_fore.clear();
                        }
                        b"LastName" if in_author => in_last_name = true,
                        b"ForeName" if in_author => in_fore_name = true,
                        b"PublicationType" => in_publication_type = true,
                        b"ELocationID" => {
                            // Check if this is a DOI
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"EIdType" && attr.value.as_ref() == b"doi"
                                {
                                    // We'll capture the DOI text in the next text event
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => match e.name().as_ref() {
                    b"ArticleTitle" => in_article_title = false,
                    b"Abstract" => in_abstract = false,
                    b"AbstractText" => in_abstract_text = false,
                    b"Title" => in_journal_title = false,
                    b"PubDate" => in_pub_date = false,
                    b"AuthorList" => in_author_list = false,
                    b"Author" => {
                        if in_author {
                            let full_name = if !current_author_fore.is_empty() {
                                format!("{} {}", current_author_fore, current_author_last)
                            } else {
                                current_author_last.clone()
                            };
                            if !full_name.trim().is_empty() {
                                authors.push(full_name);
                            }
                            in_author = false;
                        }
                    }
                    b"LastName" => in_last_name = false,
                    b"ForeName" => in_fore_name = false,
                    b"PublicationType" => in_publication_type = false,
                    _ => {}
                },
                Ok(Event::Text(e)) => {
                    let text = e
                        .unescape()
                        .map_err(|_| PubMedError::XmlParseError {
                            message: "Failed to decode XML text".to_string(),
                        })?
                        .into_owned();

                    if in_article_title {
                        title = text;
                    } else if in_abstract_text && in_abstract {
                        abstract_text = Some(text);
                    } else if in_journal_title && !in_article_title {
                        journal = text;
                    } else if in_pub_date {
                        if pub_date.is_empty() {
                            pub_date = text;
                        } else {
                            pub_date.push(' ');
                            pub_date.push_str(&text);
                        }
                    } else if in_last_name && in_author {
                        current_author_last = text;
                    } else if in_fore_name && in_author {
                        current_author_fore = text;
                    } else if in_publication_type {
                        article_types.push(text);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(PubMedError::XmlParseError {
                        message: format!("XML parsing error: {}", e),
                    });
                }
                _ => {}
            }
            buf.clear();
        }

        // If no article found, return error
        if title.is_empty() {
            debug!("No article title found in XML, article not found");
            return Err(PubMedError::ArticleNotFound {
                pmid: pmid.to_string(),
            });
        }

        debug!(
            authors_parsed = authors.len(),
            has_abstract = abstract_text.is_some(),
            journal = %journal,
            "Completed XML parsing"
        );

        Ok(PubMedArticle {
            pmid: pmid.to_string(),
            title,
            authors,
            journal,
            pub_date,
            doi,
            abstract_text,
            article_types,
        })
    }
}
