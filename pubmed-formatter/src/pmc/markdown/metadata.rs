use pubmed_parser::common::{Author, PublicationDate};
use pubmed_parser::pmc::PmcArticle;

use super::config::MarkdownConfig;
use super::entities::clean_content;
use super::frontmatter::generate_yaml_frontmatter;
use super::heading::format_heading;

fn format_first_pub_date(dates: &[PublicationDate]) -> Option<String> {
    let d = dates.first()?;
    let year = d.year?;
    match (d.month, d.day) {
        (Some(m), Some(day)) => Some(format!("{year}-{m:02}-{day:02}")),
        (Some(m), None) => Some(format!("{year}-{m:02}")),
        _ => Some(year.to_string()),
    }
}

pub(super) fn convert_metadata(config: &MarkdownConfig, article: &PmcArticle) -> String {
    if config.metadata.use_yaml_frontmatter {
        return generate_yaml_frontmatter(article);
    }

    let mut metadata = String::new();

    let title = article.title().unwrap_or("Untitled");
    metadata.push_str(&format_heading(config, &clean_content(title), 1));
    metadata.push('\n');

    if !article.authors().is_empty() {
        metadata.push_str("\n**Authors:** ");
        metadata.push_str(&format_authors(config, article.authors()));
        metadata.push('\n');
    }

    let journal_title = article
        .journal()
        .title
        .as_deref()
        .unwrap_or("Unknown Journal");
    metadata.push_str(&format!("\n**Journal:** {journal_title}"));
    if let Some(abbrev) = &article.journal().abbreviation {
        metadata.push_str(&format!(" ({abbrev})"));
    }
    metadata.push('\n');

    if let Some(pub_date) = format_first_pub_date(article.pub_dates()) {
        metadata.push_str(&format!("**Published:** {pub_date}\n"));
    }

    let mut identifiers = Vec::new();
    if let Some(doi) = article.doi() {
        if config.metadata.include_identifier_links {
            identifiers.push(format!("[DOI: {doi}](https://doi.org/{doi})"));
        } else {
            identifiers.push(format!("DOI: {doi}"));
        }
    }
    if let Some(pmid) = article.pmid() {
        let pmid_str = pmid.as_str();
        if config.metadata.include_identifier_links {
            identifiers.push(format!(
                "[PMID: {pmid_str}](https://pubmed.ncbi.nlm.nih.gov/{pmid_str})"
            ));
        } else {
            identifiers.push(format!("PMID: {pmid_str}"));
        }
    }
    let pmcid = article.pmcid().as_str();
    identifiers.push(format!("PMC: {pmcid}"));

    if !identifiers.is_empty() {
        let identifiers_str = identifiers.join(" | ");
        metadata.push_str(&format!("**Identifiers:** {identifiers_str}\n"));
    }

    if let Some(article_type) = &article.article_type {
        metadata.push_str(&format!("**Article Type:** {article_type}\n"));
    }

    if !article.keywords().is_empty() {
        let clean_keywords: Vec<String> = article
            .keywords()
            .iter()
            .map(|k| clean_content(k))
            .collect();
        let keywords_str = clean_keywords.join(", ");
        metadata.push_str(&format!("**Keywords:** {keywords_str}\n"));
    }

    let mut journal_details = Vec::new();
    if let Some(volume) = article.volume() {
        journal_details.push(format!("Volume {volume}"));
    }
    if let Some(issue) = article.issue() {
        journal_details.push(format!("Issue {issue}"));
    }
    if let Some(publisher) = &article.journal().publisher {
        journal_details.push(format!("Publisher: {publisher}"));
    }
    if !journal_details.is_empty() {
        metadata.push_str(&format!(
            "**Journal Details:** {}\n",
            journal_details.join(" | ")
        ));
    }

    metadata
}

fn format_authors(config: &MarkdownConfig, authors: &[Author]) -> String {
    authors
        .iter()
        .map(|author| {
            let mut name = clean_content(&author.full_name);

            if author.is_corresponding {
                name.push('*');
            }

            if config.metadata.include_orcid_links
                && let Some(orcid) = &author.orcid
            {
                let cleaned_orcid = clean_content(orcid);
                let clean_orcid = cleaned_orcid.trim_start_matches("https://orcid.org/");

                if clean_orcid.len() >= 19 && clean_orcid.matches('-').count() == 3 {
                    name.push_str(&format!(" ([ORCID](https://orcid.org/{clean_orcid}))"));
                }
            }

            name
        })
        .collect::<Vec<String>>()
        .join(", ")
}
