use pubmed_parser::pmc::Reference;

use super::config::{MarkdownConfig, ReferenceStyle};
use super::heading::format_heading;

pub(super) fn convert_references(config: &MarkdownConfig, references: &[Reference]) -> String {
    let mut content = String::new();
    content.push_str(&format_heading(config, "References", 2));
    content.push_str("\n\n");

    match config.reference_style {
        ReferenceStyle::Numbered => {
            for (i, reference) in references.iter().enumerate() {
                content.push_str(&format!(
                    "{}. {}\n",
                    i + 1,
                    format_reference(config, reference)
                ));
            }
        }
        ReferenceStyle::AuthorYear | ReferenceStyle::FullCitation => {
            for reference in references {
                let formatted_ref = format_reference(config, reference);
                content.push_str(&format!("- {formatted_ref}\n"));
            }
        }
    }

    content.push('\n');
    content
}

fn format_reference(config: &MarkdownConfig, reference: &Reference) -> String {
    match config.reference_style {
        ReferenceStyle::Numbered | ReferenceStyle::FullCitation => {
            let citation = reference.format_citation();

            if config.metadata.include_identifier_links {
                let mut formatted = citation;

                if let Some(doi) = &reference.doi {
                    formatted.push_str(&format!(" [DOI](https://doi.org/{doi})"));
                }

                if let Some(pmid) = &reference.pmid {
                    formatted.push_str(&format!(" [PMID](https://pubmed.ncbi.nlm.nih.gov/{pmid})"));
                }

                formatted
            } else {
                citation
            }
        }
        ReferenceStyle::AuthorYear => {
            if let (Some(first_author), Some(year)) =
                (reference.authors.first(), reference.year.as_ref())
            {
                format!("{} ({})", first_author.full_name, year)
            } else {
                reference.format_citation()
            }
        }
    }
}
