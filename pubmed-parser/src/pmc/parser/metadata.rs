use super::xml_utils;
use crate::pmc::models::{FundingInfo, JournalInfo, SupplementaryMaterial};

/// Extract comprehensive journal information
pub fn extract_journal_info(content: &str) -> JournalInfo {
    let mut journal = JournalInfo::new(
        xml_utils::extract_text_between(content, "<journal-title>", "</journal-title>")
            .unwrap_or_else(|| "Unknown Journal".to_string()),
    );

    // Extract journal abbreviation
    journal.abbreviation = xml_utils::extract_text_between(
        content,
        "<journal-id journal-id-type=\"iso-abbrev\">",
        "</journal-id>",
    );

    // Extract ISSNs
    let mut pos = 0;
    while let Some(issn_start) = content[pos..].find("<issn") {
        let issn_start = pos + issn_start;
        if let Some(issn_end) = content[issn_start..].find("</issn>") {
            let issn_end = issn_start + issn_end;
            let issn_section = &content[issn_start..issn_end];

            if let Some(content_start) = issn_section.find(">") {
                let issn_value = &issn_section[content_start + 1..];

                if issn_section.contains("pub-type=\"epub\"") {
                    journal.issn_electronic = Some(issn_value.to_string());
                } else if issn_section.contains("pub-type=\"ppub\"") {
                    journal.issn_print = Some(issn_value.to_string());
                }
            }
            pos = issn_end;
        } else {
            break;
        }
    }

    // Extract publisher
    journal.publisher =
        xml_utils::extract_text_between(content, "<publisher-name>", "</publisher-name>");

    // Extract volume and issue
    journal.volume = xml_utils::extract_text_between(content, "<volume>", "</volume>");
    journal.issue = xml_utils::extract_text_between(content, "<issue>", "</issue>");

    journal
}

/// Extract publication date in YYYY-MM-DD format
pub fn extract_pub_date(content: &str) -> String {
    if let Some(year) = xml_utils::extract_text_between(content, "<year>", "</year>") {
        if let Some(month) = xml_utils::extract_text_between(content, "<month>", "</month>") {
            if let Some(day) = xml_utils::extract_text_between(content, "<day>", "</day>") {
                return format!(
                    "{}-{:02}-{:02}",
                    year,
                    month.parse::<u32>().unwrap_or(1),
                    day.parse::<u32>().unwrap_or(1)
                );
            }
            return format!("{}-{:02}", year, month.parse::<u32>().unwrap_or(1));
        }
        return year;
    }
    "Unknown Date".to_string()
}

/// Extract DOI from article metadata
pub fn extract_doi(content: &str) -> Option<String> {
    let mut pos = 0;
    while let Some(id_start) = content[pos..].find(r#"<article-id pub-id-type="doi""#) {
        let id_start = pos + id_start;
        if let Some(content_start) = content[id_start..].find(">") {
            let content_start = id_start + content_start + 1;
            if let Some(content_end) = content[content_start..].find("</article-id>") {
                let content_end = content_start + content_end;
                return Some(content[content_start..content_end].trim().to_string());
            }
        }
        pos = id_start + 1;
    }
    None
}

/// Extract PMID from article metadata
pub fn extract_pmid(content: &str) -> Option<String> {
    let mut pos = 0;
    while let Some(id_start) = content[pos..].find(r#"<article-id pub-id-type="pmid""#) {
        let id_start = pos + id_start;
        if let Some(content_start) = content[id_start..].find(">") {
            let content_start = id_start + content_start + 1;
            if let Some(content_end) = content[content_start..].find("</article-id>") {
                let content_end = content_start + content_end;
                return Some(content[content_start..content_end].trim().to_string());
            }
        }
        pos = id_start + 1;
    }
    None
}

/// Extract article type from article metadata
pub fn extract_article_type(content: &str) -> Option<String> {
    // Look for article-type attribute in article tag
    if let Some(article_start) = content.find("<article") {
        if let Some(article_end) = content[article_start..].find(">") {
            let article_tag = &content[article_start..article_start + article_end];
            if let Some(type_start) = article_tag.find("article-type=\"") {
                let type_start = type_start + 14; // Length of "article-type=\""
                if let Some(type_end) = article_tag[type_start..].find('"') {
                    return Some(article_tag[type_start..type_start + type_end].to_string());
                }
            }
        }
    }

    // Fallback: look in article-categories
    xml_utils::extract_text_between(content, "<subject>", "</subject>")
}

/// Extract keywords from article metadata
pub fn extract_keywords(content: &str) -> Vec<String> {
    let mut keywords = Vec::new();

    if let Some(kwd_start) = content.find("<kwd-group") {
        if let Some(kwd_end) = content[kwd_start..].find("</kwd-group>") {
            let kwd_section = &content[kwd_start..kwd_start + kwd_end];

            let mut pos = 0;
            while let Some(kwd_start) = kwd_section[pos..].find("<kwd>") {
                let kwd_start = pos + kwd_start + 5; // Length of "<kwd>"
                if let Some(kwd_end) = kwd_section[kwd_start..].find("</kwd>") {
                    let raw_keyword = kwd_section[kwd_start..kwd_start + kwd_end].trim();
                    // Strip any nested XML tags from the keyword
                    let keyword = xml_utils::strip_xml_tags(raw_keyword);
                    if !keyword.is_empty() {
                        keywords.push(keyword);
                    }
                    pos = kwd_start + kwd_end;
                } else {
                    break;
                }
            }
        }
    }

    keywords
}

/// Extract funding information
pub fn extract_funding(content: &str) -> Vec<FundingInfo> {
    let mut funding = Vec::new();

    if let Some(funding_start) = content.find("<funding-group>") {
        if let Some(funding_end) = content[funding_start..].find("</funding-group>") {
            let funding_section = &content[funding_start..funding_start + funding_end];

            let mut pos = 0;
            while let Some(award_start) = funding_section[pos..].find("<award-group") {
                let award_start = pos + award_start;
                if let Some(award_end) = funding_section[award_start..].find("</award-group>") {
                    let award_end = award_start + award_end;
                    let award_section = &funding_section[award_start..award_end];

                    let source = xml_utils::extract_text_between(
                        award_section,
                        "<funding-source>",
                        "</funding-source>",
                    )
                    .unwrap_or_else(|| "Unknown Source".to_string());

                    let mut funding_info = FundingInfo::new(source);
                    funding_info.award_id =
                        xml_utils::extract_text_between(award_section, "<award-id>", "</award-id>");

                    funding.push(funding_info);
                    pos = award_end;
                } else {
                    break;
                }
            }
        }
    }

    funding
}

/// Extract conflict of interest statement
pub fn extract_conflict_of_interest(content: &str) -> Option<String> {
    // Look for conflict of interest in fn-group
    if let Some(fn_start) = content.find("<fn-group") {
        if let Some(fn_end) = content[fn_start..].find("</fn-group>") {
            let fn_section = &content[fn_start..fn_start + fn_end];

            // Look for conflict or competing interest
            let mut pos = 0;
            while let Some(fn_start) = fn_section[pos..].find("<fn") {
                let fn_start = pos + fn_start;
                if let Some(fn_end) = fn_section[fn_start..].find("</fn>") {
                    let fn_end = fn_start + fn_end;
                    let fn_content = &fn_section[fn_start..fn_end];

                    if fn_content.contains("conflict") || fn_content.contains("competing") {
                        if let Some(p_start) = fn_content.find("<p>") {
                            if let Some(p_end) = fn_content[p_start..].find("</p>") {
                                let coi = &fn_content[p_start + 3..p_start + p_end];
                                return Some(xml_utils::strip_xml_tags(coi));
                            }
                        }
                    }
                    pos = fn_end;
                } else {
                    break;
                }
            }
        }
    }

    // Look for conflict statement in dedicated section
    if let Some(coi_start) = content.find("<sec") {
        if let Some(coi_end) = content[coi_start..].find("</sec>") {
            let coi_section = &content[coi_start..coi_start + coi_end];
            if coi_section.contains("conflict") || coi_section.contains("competing") {
                if let Some(title_start) = coi_section.find("<title>") {
                    if let Some(title_end) = coi_section[title_start..].find("</title>") {
                        let title = &coi_section[title_start + 7..title_start + title_end];
                        if title.to_lowercase().contains("conflict")
                            || title.to_lowercase().contains("competing")
                        {
                            if let Some(p_start) = coi_section.find("<p>") {
                                if let Some(p_end) = coi_section[p_start..].find("</p>") {
                                    let coi = &coi_section[p_start + 3..p_start + p_end];
                                    return Some(xml_utils::strip_xml_tags(coi));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Extract acknowledgments
pub fn extract_acknowledgments(content: &str) -> Option<String> {
    xml_utils::extract_text_between(content, "<ack>", "</ack>")
        .map(|ack| xml_utils::strip_xml_tags(&ack))
}

/// Extract data availability statement
pub fn extract_data_availability(content: &str) -> Option<String> {
    // Look for data availability in dedicated section
    if let Some(data_start) = content.find("<sec") {
        if let Some(data_end) = content[data_start..].find("</sec>") {
            let data_section = &content[data_start..data_start + data_end];
            if data_section.contains("data") && data_section.contains("availab") {
                return Some(xml_utils::strip_xml_tags(data_section));
            }
        }
    }

    // Look for data availability statement in supplementary material
    if let Some(supp_start) = content.find("<supplementary-material") {
        if let Some(supp_end) = content[supp_start..].find("</supplementary-material>") {
            let supp_section = &content[supp_start..supp_start + supp_end];
            if supp_section.contains("data") && supp_section.contains("availab") {
                return Some(xml_utils::strip_xml_tags(supp_section));
            }
        }
    }

    None
}

/// Extract supplementary materials
pub fn extract_supplementary_materials(content: &str) -> Vec<SupplementaryMaterial> {
    let mut materials = Vec::new();

    let mut pos = 0;
    while let Some(supp_start) = content[pos..].find("<supplementary-material") {
        let supp_start = pos + supp_start;
        if let Some(supp_end) = content[supp_start..].find("</supplementary-material>") {
            let supp_end = supp_start + supp_end;
            let supp_content = &content[supp_start..supp_end];

            let id = xml_utils::extract_attribute_value(supp_content, "id").unwrap_or_else(|| {
                let supp_num = materials.len() + 1;
                format!("supp_{supp_num}")
            });

            let label = xml_utils::extract_text_between(supp_content, "<label>", "</label>");
            let caption = xml_utils::extract_text_between(supp_content, "<caption>", "</caption>")
                .and_then(|caption_content| {
                    // First try to extract just the title from caption
                    xml_utils::extract_text_between(&caption_content, "<title>", "</title>")
                        .or_else(|| {
                            // If no title, extract all content and strip tags
                            Some(xml_utils::strip_xml_tags(&caption_content))
                        })
                })
                .unwrap_or_else(|| "No caption available".to_string());

            let content_type = xml_utils::extract_attribute_value(supp_content, "content-type");
            let mime_type = xml_utils::extract_attribute_value(supp_content, "mimetype");
            let position = xml_utils::extract_attribute_value(supp_content, "position");
            let href = xml_utils::extract_attribute_value(supp_content, "href")
                .or_else(|| xml_utils::extract_attribute_value(supp_content, "xlink:href"))
                .or_else(|| {
                    // Look for href in nested media tags
                    if let Some(media_start) = supp_content.find("<media") {
                        if let Some(media_end) = supp_content[media_start..].find(">") {
                            let media_tag = &supp_content[media_start..media_start + media_end + 1];
                            xml_utils::extract_attribute_value(media_tag, "xlink:href")
                                .or_else(|| xml_utils::extract_attribute_value(media_tag, "href"))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

            let mut material = SupplementaryMaterial::new(id);
            material.title = Some(caption);
            material.description = label;
            material.content_type = content_type;
            material.file_type = mime_type;
            material.position = position;
            material.file_url = href;

            materials.push(material);
            pos = supp_end;
        } else {
            break;
        }
    }

    materials
}

/// Extract article title
pub fn extract_title(content: &str) -> String {
    xml_utils::extract_text_between(content, "<article-title>", "</article-title>")
        .unwrap_or_else(|| "Unknown Title".to_string())
}

/// Extract article language
pub fn extract_language(content: &str) -> Option<String> {
    // Look for language in article tag
    if let Some(article_start) = content.find("<article") {
        if let Some(article_end) = content[article_start..].find(">") {
            let article_tag = &content[article_start..article_start + article_end];
            if let Some(lang) = xml_utils::extract_attribute_value(article_tag, "xml:lang") {
                return Some(lang);
            }
        }
    }
    None
}

/// Extract article identifiers (DOI, PMID, PMC ID, etc.)
pub fn extract_article_ids(content: &str) -> Vec<(String, String)> {
    let mut ids = Vec::new();

    let id_tags = xml_utils::find_all_tags(content, "article-id");
    for id_tag in id_tags {
        if let Some(id_type) = xml_utils::extract_attribute_value(&id_tag, "pub-id-type") {
            if let Some(id_value) = xml_utils::extract_element_content(&id_tag, "article-id") {
                ids.push((id_type, id_value.trim().to_string()));
            }
        }
    }

    ids
}

/// Extract copyright information
pub fn extract_copyright(content: &str) -> Option<String> {
    xml_utils::extract_text_between(content, "<copyright-statement>", "</copyright-statement>")
        .or_else(|| {
            xml_utils::extract_text_between(content, "<copyright-year>", "</copyright-year>")
        })
}

/// Extract license information
pub fn extract_license(content: &str) -> Option<String> {
    xml_utils::extract_element_content(content, "license")
        .map(|license_content| xml_utils::strip_xml_tags(&license_content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title() {
        let content = r#"<article-title>Test Article Title</article-title>"#;
        let title = extract_title(content);
        assert_eq!(title, "Test Article Title");
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
    fn test_extract_pub_date() {
        let content_full = r#"<year>2023</year><month>12</month><day>25</day>"#;
        assert_eq!(extract_pub_date(content_full), "2023-12-25");

        let content_year_month = r#"<year>2023</year><month>12</month>"#;
        assert_eq!(extract_pub_date(content_year_month), "2023-12");

        let content_year_only = r#"<year>2023</year>"#;
        assert_eq!(extract_pub_date(content_year_only), "2023");

        let content_no_date = r#"<title>No date here</title>"#;
        assert_eq!(extract_pub_date(content_no_date), "Unknown Date");
    }

    #[test]
    fn test_extract_article_type() {
        let content = r#"<article article-type="research-article">Content</article>"#;
        let article_type = extract_article_type(content);
        assert_eq!(article_type, Some("research-article".to_string()));
    }

    #[test]
    fn test_extract_language() {
        let content = r#"<article xml:lang="en">Content</article>"#;
        let language = extract_language(content);
        assert_eq!(language, Some("en".to_string()));
    }

    #[test]
    fn test_extract_article_ids() {
        let content = r#"
        <article-id pub-id-type="doi">10.1234/test</article-id>
        <article-id pub-id-type="pmid">12345</article-id>
        <article-id pub-id-type="pmc">PMC123456</article-id>
        "#;

        let ids = extract_article_ids(content);
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&("doi".to_string(), "10.1234/test".to_string())));
        assert!(ids.contains(&("pmid".to_string(), "12345".to_string())));
        assert!(ids.contains(&("pmc".to_string(), "PMC123456".to_string())));
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
}
