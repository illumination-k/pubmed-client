//! PMC Open Access download tools.
//!
//! `get_pmc_figures` only reports what the JATS XML says about an article's
//! figures; these two tools fetch the actual bytes from the PMC OA Cloud and
//! write them to a caller-chosen destination, closing the gap between the CLI
//! and the MCP server (issue #288).
//!
//! The destination is a local directory or an `s3://bucket/prefix` URI — one
//! argument, either kind, resolved by [`Destination`]. Object storage is what
//! makes these usable from a server that has no writable filesystem, or whose
//! output another job needs to read.
//!
//! Both require an explicit `output_dir`: an MCP server runs on the caller's
//! machine, so guessing where to scatter downloaded files would be the wrong
//! kind of helpful.

use pubmed_client::{Destination, FigureSelection, StorageBackend};
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
        description = "Figure ids or labels to download (e.g. ['fig1', 'Figure 2']). Omit for all figures."
    )]
    pub figure_ids: Option<Vec<String>>,
    #[schemars(
        description = "Where to write: a local directory, or an object-storage prefix as 's3://bucket/prefix' (S3, MinIO, R2 — credentials and region come from the usual AWS_* environment variables). Local directories are created if missing."
    )]
    pub output_dir: String,
}

/// Request parameters for the `download_pmc_files` tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DownloadFilesRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,
    #[schemars(
        description = "Where to write: a local directory, or an object-storage prefix as 's3://bucket/prefix' (S3, MinIO, R2 — credentials and region come from the usual AWS_* environment variables). Local directories are created if missing."
    )]
    pub output_dir: String,
}

/// A figure that was downloaded to disk: its XML metadata plus where it landed.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct DownloadedFigureOut {
    /// Figure metadata, as reported by `get_pmc_figures`.
    #[serde(flatten)]
    pub figure: FigureOut,
    /// Where the image was written: an absolute path, or an `s3://bucket/key` URI.
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
    /// The destination the figures were written to, as given.
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
    /// The destination the files were written to, as given.
    pub output_dir: String,
    /// Number of files downloaded.
    pub file_count: usize,
    /// Where each file was written: absolute paths, or `s3://bucket/key` URIs.
    pub files: Vec<String>,
}

/// Resolve an `output_dir` argument into a storage backend.
///
/// A blank destination is the caller's mistake, not the working directory, so it
/// is rejected as an invalid parameter rather than silently scattering files next
/// to the server process. A malformed `s3://` URI is likewise the caller's to
/// fix; anything that goes wrong reaching the destination (an uncreatable
/// directory, an unresolvable AWS config) is the server's problem to report.
pub(crate) async fn resolve_destination(
    output_dir: &str,
) -> Result<(Destination, Box<dyn StorageBackend>), ErrorData> {
    let destination = Destination::parse(output_dir)
        .map_err(|e| invalid_params(format!("Invalid output_dir: {}", e)))?;

    let storage = destination
        .clone()
        .into_backend()
        .await
        .map_err(|e| internal_error(format!("Failed to open output_dir: {}", e)))?;

    Ok((destination, storage))
}

/// Download a PMC article's figures to a local directory or object storage
///
/// Only the figures are written. The client resolves `<fig>` elements in memory,
/// so unlike the CLI's temp-directory dance the destination never sees the
/// article's PDF or supplementary files.
pub async fn download_pmc_figures(
    server: &super::PubMedServer,
    Parameters(params): Parameters<DownloadFiguresRequest>,
) -> Result<Json<DownloadFiguresOutput>, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);
    let (destination, storage) = resolve_destination(&params.output_dir).await?;

    let mut selection = FigureSelection::new();
    if let Some(ids) = params.figure_ids {
        selection = selection.with_ids(ids);
    }

    info!(
        pmc_id = %pmc_id,
        destination = %destination.display(),
        object_storage = destination.is_object_storage(),
        "Downloading PMC figures"
    );

    let downloaded = server
        .client
        .pmc
        .download_figures_to(&pmc_id, storage.as_ref(), &selection)
        .await
        .map_err(|e| internal_error(format!("Failed to download PMC figures: {}", e)))?;

    let figures: Vec<DownloadedFigureOut> = downloaded
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
        output_dir: destination.display(),
        figure_count: figures.len(),
        figures,
    }))
}

/// Download a PMC article's full Open Access package to a local directory or object storage
pub async fn download_pmc_files(
    server: &super::PubMedServer,
    Parameters(params): Parameters<DownloadFilesRequest>,
) -> Result<Json<DownloadFilesOutput>, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);
    let (destination, storage) = resolve_destination(&params.output_dir).await?;

    info!(
        pmc_id = %pmc_id,
        destination = %destination.display(),
        object_storage = destination.is_object_storage(),
        "Downloading PMC OA files"
    );

    let files = server
        .client
        .pmc
        .download_files_to(&pmc_id, storage.as_ref())
        .await
        .map_err(|e| internal_error(format!("Failed to download PMC files: {}", e)))?;

    Ok(Json(DownloadFilesOutput {
        pmc_id,
        output_dir: destination.display(),
        file_count: files.len(),
        files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_blank_output_dir_is_rejected_rather_than_defaulted() {
        for blank in ["", "   "] {
            let err = resolve_destination(blank)
                .await
                .err()
                .map(|err| err.code)
                .expect("a blank output_dir must not resolve to the working directory");
            assert_eq!(err, ErrorCode(-32602));
        }
    }

    /// A caller's malformed URI is their mistake to fix, so it must come back as
    /// invalid_params — an internal error would be rendered opaquely by clients.
    #[tokio::test]
    async fn a_bucketless_s3_uri_is_a_parameter_error() {
        let err = resolve_destination("s3://")
            .await
            .err()
            .map(|err| err.code)
            .expect("an S3 URI with no bucket must be rejected");
        assert_eq!(err, ErrorCode(-32602));
    }

    #[tokio::test]
    async fn a_local_destination_is_created_and_reported_as_a_filesystem_path() {
        let temp = tempfile::tempdir().unwrap();
        let nested = temp.path().join("nested/figures");

        let (destination, storage) = resolve_destination(&nested.to_string_lossy())
            .await
            .expect("a creatable local path should resolve");

        assert!(!destination.is_object_storage());
        assert!(
            nested.is_dir(),
            "the destination should be created up front"
        );
        assert!(storage.get_full_path("fig.png").ends_with("fig.png"));
    }

    /// Whitespace around a destination is a typo, and an `s3://` URI must be
    /// recognized through it rather than treated as a directory named "s3:".
    #[tokio::test]
    async fn an_s3_destination_is_recognized_and_reported_as_a_uri() {
        let (destination, _storage) = resolve_destination("  s3://bucket/pmc/figures  ")
            .await
            .expect("an S3 URI should resolve without contacting the bucket");

        assert!(destination.is_object_storage());
        assert_eq!(destination.display(), "s3://bucket/pmc/figures");
    }
}
