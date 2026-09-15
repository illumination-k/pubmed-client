//! PMC Open Access download tools.
//!
//! `get_pmc_figures` only reports what the JATS XML says about an article's
//! figures; these two tools fetch the actual bytes from the PMC OA Cloud and
//! write them to a caller-chosen directory, closing the gap between the CLI and
//! the MCP server (issue #288).
//!
//! Both require an explicit `output_dir`: an MCP server runs on the caller's
//! machine, so guessing where to scatter downloaded files would be the wrong
//! kind of helpful.

use rmcp::{
    handler::server::wrapper::{Json, Parameters},
    model::*,
    schemars,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::{internal_error, invalid_params, normalize_pmc_id};
use super::output::FigureOut;

/// Request parameters for the `download_pmc_figures` tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DownloadFiguresRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,
    #[schemars(
        description = "Directory to download the article's figures into. Created if missing."
    )]
    pub output_dir: String,
}

/// Request parameters for the `download_pmc_files` tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DownloadFilesRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,
    #[schemars(
        description = "Directory to download the article's Open Access files into. Created if missing."
    )]
    pub output_dir: String,
}

/// A figure that was downloaded to disk: its XML metadata plus where it landed.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DownloadedFigureOut {
    /// Figure metadata, as reported by `get_pmc_figures`.
    #[serde(flatten)]
    pub figure: FigureOut,
    /// Absolute path of the downloaded image.
    pub file_path: String,
    /// File size in bytes, if it could be read back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    /// Image width in pixels, for formats that declare it in their header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Image height in pixels, for formats that declare it in their header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// Structured answer of the `download_pmc_figures` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DownloadFiguresOutput {
    /// The PMC ID that was downloaded.
    pub pmc_id: String,
    /// The directory the figures were written to.
    pub output_dir: String,
    /// Number of figures downloaded.
    pub figure_count: usize,
    /// Downloaded figures, in document order.
    pub figures: Vec<DownloadedFigureOut>,
}

/// Structured answer of the `download_pmc_files` tool.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DownloadFilesOutput {
    /// The PMC ID that was downloaded.
    pub pmc_id: String,
    /// The directory the files were written to.
    pub output_dir: String,
    /// Number of files downloaded.
    pub file_count: usize,
    /// Absolute paths of the downloaded files.
    pub files: Vec<String>,
}

/// Reject an output directory the caller left blank.
///
/// `PmcClient` would happily create a directory named `""` relative to the
/// server's working directory; making the caller name a destination is the
/// whole point of the argument.
pub(crate) fn validate_output_dir(output_dir: &str) -> Result<&str, ErrorData> {
    let trimmed = output_dir.trim();
    if trimmed.is_empty() {
        return Err(invalid_params("output_dir must not be empty"));
    }
    Ok(trimmed)
}

/// Download a PMC article's figures to a local directory
///
/// The underlying client fetches the whole OA package to resolve `<fig>`
/// elements against real files, so `output_dir` also ends up holding the
/// article's XML, PDF and supplementary materials. That is the honest cost of
/// figure matching; callers who want only the images should use
/// `get_pmc_figure_images`, which resolves them without writing anything.
pub async fn download_pmc_figures(
    server: &super::PubMedServer,
    Parameters(params): Parameters<DownloadFiguresRequest>,
) -> Result<Json<DownloadFiguresOutput>, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);
    let output_dir = validate_output_dir(&params.output_dir)?;

    info!(pmc_id = %pmc_id, output_dir = %output_dir, "Downloading PMC figures");

    let extracted = server
        .client
        .pmc
        .extract_figures_with_captions(&pmc_id, output_dir)
        .await
        .map_err(|e| internal_error(format!("Failed to download PMC figures: {}", e)))?;

    let figures: Vec<DownloadedFigureOut> = extracted
        .into_iter()
        .map(|extracted| DownloadedFigureOut {
            figure: FigureOut::from(&extracted.figure),
            file_path: extracted.extracted_file_path,
            file_size: extracted.file_size,
            width: extracted.dimensions.map(|(width, _)| width),
            height: extracted.dimensions.map(|(_, height)| height),
        })
        .collect();

    Ok(Json(DownloadFiguresOutput {
        pmc_id,
        output_dir: output_dir.to_string(),
        figure_count: figures.len(),
        figures,
    }))
}

/// Download a PMC article's full Open Access package to a local directory
pub async fn download_pmc_files(
    server: &super::PubMedServer,
    Parameters(params): Parameters<DownloadFilesRequest>,
) -> Result<Json<DownloadFilesOutput>, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);
    let output_dir = validate_output_dir(&params.output_dir)?;

    info!(pmc_id = %pmc_id, output_dir = %output_dir, "Downloading PMC OA files");

    let files = server
        .client
        .pmc
        .download_files(&pmc_id, output_dir)
        .await
        .map_err(|e| internal_error(format!("Failed to download PMC files: {}", e)))?;

    Ok(Json(DownloadFilesOutput {
        pmc_id,
        output_dir: output_dir.to_string(),
        file_count: files.len(),
        files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_output_dir_is_rejected_rather_than_defaulted() {
        for blank in ["", "   "] {
            let err = validate_output_dir(blank)
                .expect_err("a blank output_dir must not resolve to the working directory");
            assert_eq!(err.code, ErrorCode(-32602));
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(validate_output_dir("  ./figures \n").unwrap(), "./figures");
    }
}
