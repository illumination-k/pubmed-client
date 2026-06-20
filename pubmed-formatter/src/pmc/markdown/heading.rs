use super::config::{HeadingStyle, MarkdownConfig};

pub(super) fn format_heading(config: &MarkdownConfig, text: &str, level: u8) -> String {
    let level = level.min(config.max_heading_level);

    match config.heading_style {
        HeadingStyle::ATX => {
            let hashes = "#".repeat(level as usize);
            format!("{hashes} {text}")
        }
        HeadingStyle::Setext => {
            if level == 1 {
                let underline = "=".repeat(text.len());
                format!("{text}\n{underline}")
            } else if level == 2 {
                let underline = "-".repeat(text.len());
                format!("{text}\n{underline}")
            } else {
                let hashes = "#".repeat(level as usize);
                format!("{hashes} {text}")
            }
        }
    }
}

pub(super) fn heading_anchor(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
