use pubmed_parser::pmc::PmcArticle;

use super::config::MarkdownConfig;
use super::heading::{format_heading, heading_anchor};

pub(super) fn convert_toc(config: &MarkdownConfig, article: &PmcArticle) -> String {
    let mut toc = String::new();
    toc.push_str(&format_heading(config, "Table of Contents", 2));
    toc.push('\n');

    for (i, section) in article.sections().iter().enumerate() {
        let default_title = "Untitled".to_string();
        let title = section.title.as_ref().unwrap_or(&default_title);
        let anchor = heading_anchor(title);
        let index = i + 1;
        toc.push_str(&format!("{index}. [{title}](#{anchor})\n"));

        for (j, subsection) in section.subsections.iter().enumerate() {
            let default_sub_title = "Untitled".to_string();
            let sub_title = subsection.title.as_ref().unwrap_or(&default_sub_title);
            let sub_anchor = heading_anchor(sub_title);
            let main_index = i + 1;
            let sub_index = j + 1;
            toc.push_str(&format!(
                "   {main_index}.{sub_index}. [{sub_title}](#{sub_anchor})\n"
            ));
        }
    }

    toc
}
