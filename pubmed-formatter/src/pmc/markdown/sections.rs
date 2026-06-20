use std::collections::HashMap;

use pubmed_parser::pmc::{Figure, FundingInfo, PmcArticle, Section, Table};

use super::config::MarkdownConfig;
use super::entities::clean_content;
use super::heading::format_heading;

pub(super) fn convert_sections(
    config: &MarkdownConfig,
    sections: &[Section],
    level: u8,
    figure_paths: Option<&HashMap<String, String>>,
) -> String {
    let mut content = String::new();

    for section in sections {
        if let Some(title) = &section.title {
            content.push_str(&format_heading(config, title, level));
            content.push_str("\n\n");
        }

        if !section.content.is_empty() {
            content.push_str(&clean_content(&section.content));
            content.push_str("\n\n");
        }

        if config.figures.include_figure_captions {
            for figure in &section.figures {
                let figure_path = figure_paths.and_then(|paths| paths.get(&figure.id));
                content.push_str(&convert_figure(config, figure, figure_path));
                content.push_str("\n\n");
            }

            for table in &section.tables {
                content.push_str(&convert_table(table));
                content.push_str("\n\n");
            }
        }

        if !section.subsections.is_empty() {
            let next_level = (level + 1).min(config.max_heading_level);
            content.push_str(&convert_sections(
                config,
                &section.subsections,
                next_level,
                figure_paths,
            ));
        }
    }

    content
}

pub(super) fn convert_additional_sections(config: &MarkdownConfig, article: &PmcArticle) -> String {
    let mut content = String::new();

    if !article.funding().is_empty() {
        content.push_str(&format_heading(config, "Funding", 2));
        content.push_str("\n\n");
        for funding in article.funding() {
            content.push_str(&format_funding(funding));
            content.push('\n');
        }
        content.push('\n');
    }

    if let Some(coi) = article.conflict_of_interest() {
        content.push_str(&format_heading(config, "Conflict of Interest", 2));
        content.push_str("\n\n");
        content.push_str(&clean_content(coi));
        content.push_str("\n\n");
    }

    if let Some(ack) = article.acknowledgments() {
        content.push_str(&format_heading(config, "Acknowledgments", 2));
        content.push_str("\n\n");
        content.push_str(&clean_content(ack));
        content.push_str("\n\n");
    }

    if let Some(data_avail) = &article.data_availability {
        content.push_str(&format_heading(config, "Data Availability", 2));
        content.push_str("\n\n");
        content.push_str(&clean_content(data_avail));
        content.push_str("\n\n");
    }

    content
}

fn convert_figure(
    config: &MarkdownConfig,
    figure: &Figure,
    figure_path: Option<&String>,
) -> String {
    let mut content = String::new();

    if config.figures.include_local_figures
        && let Some(path) = figure_path
    {
        let alt_text = figure
            .alt_text
            .as_deref()
            .or(figure.label.as_deref())
            .unwrap_or(&figure.id);
        content.push_str(&format!("![{alt_text}]({path})\n\n"));
    }

    if let Some(label) = &figure.label {
        content.push_str(&format!("**{label}**"));
    } else {
        let figure_id = &figure.id;
        content.push_str(&format!("**Figure {figure_id}**"));
    }

    if let Some(caption) = &figure.caption {
        let caption = clean_content(caption);
        content.push_str(&format!(": {caption}"));
    }

    if let Some(alt_text) = &figure.alt_text {
        let alt_content = clean_content(alt_text);
        content.push_str(&format!("\n\n*Alt text: {alt_content}*"));
    }

    content
}

fn convert_table(table: &Table) -> String {
    let mut content = String::new();

    if let Some(label) = &table.label {
        content.push_str(&format!("**{label}**"));
    } else {
        let table_id = &table.id;
        content.push_str(&format!("**Table {table_id}**"));
    }

    if let Some(caption) = &table.caption {
        let caption = clean_content(caption);
        content.push_str(&format!(": {caption}"));
    }

    if !table.footnotes.is_empty() {
        content.push_str("\n\n*Footnotes:*\n");
        for (i, footnote) in table.footnotes.iter().enumerate() {
            let index = i + 1;
            let footnote_content = clean_content(footnote);
            content.push_str(&format!("{index}. {footnote_content}\n"));
        }
    }

    content
}

fn format_funding(funding: &FundingInfo) -> String {
    let source = funding.source.as_deref().unwrap_or("Unknown");
    let mut text = format!("- **{source}**");

    if let Some(award_id) = &funding.award_id {
        text.push_str(&format!(" (Award ID: {award_id})"));
    }

    if let Some(statement) = &funding.statement {
        let content = clean_content(statement);
        text.push_str(&format!(": {content}"));
    }

    text
}
