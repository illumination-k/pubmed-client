//! Shared helpers for MCP tool implementations

use pubmed_client::{Figure, Section, Table};
use rmcp::model::*;
use std::borrow::Cow;
use std::fmt::Display;

pub fn internal_error(msg: impl Display) -> ErrorData {
    ErrorData {
        code: ErrorCode(-32603),
        message: Cow::from(msg.to_string()),
        data: None,
    }
}

pub fn invalid_params(msg: impl Display) -> ErrorData {
    ErrorData {
        code: ErrorCode(-32602),
        message: Cow::from(msg.to_string()),
        data: None,
    }
}

pub fn text_result(s: impl Into<String>) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![Content::text(s.into())]))
}

pub fn normalize_pmc_id(pmc_id: &str) -> String {
    if pmc_id.starts_with("PMC") {
        pmc_id.to_string()
    } else {
        format!("PMC{}", pmc_id)
    }
}

pub fn collect_figures(sections: &[Section]) -> Vec<&Figure> {
    let mut figures = Vec::new();
    for section in sections {
        figures.extend(section.figures.iter());
        figures.extend(collect_figures(&section.subsections));
    }
    figures
}

pub fn collect_tables(sections: &[Section]) -> Vec<&Table> {
    let mut tables = Vec::new();
    for section in sections {
        tables.extend(section.tables.iter());
        tables.extend(collect_tables(&section.subsections));
    }
    tables
}
