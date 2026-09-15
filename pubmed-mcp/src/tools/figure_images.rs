//! Figure image tool: returns the images themselves, not paths to them.
//!
//! `download_pmc_figures` needs somewhere to write; a host that sandboxes the
//! server, runs it over HTTP, or simply wants the model to *look* at a figure
//! has nowhere to point it. This tool fetches the figures into memory and
//! answers with the bytes inline: renderable formats as MCP `image` content
//! blocks, everything else (PDF/EPS vector figures, say) as an embedded
//! resource blob, so no figure is silently dropped for its format.
//!
//! Inline bytes are expensive — base64 inflates them by a third and the whole
//! response is one JSON message — so the tool ships with a figure count cap and
//! a total byte budget, both overridable per call.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use pubmed_client::FigureSelection;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::*,
    schemars::{self, JsonSchema},
};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::common::{internal_error, invalid_params, normalize_pmc_id};
use super::output::FigureOut;

/// Figures returned when the caller does not say otherwise.
///
/// Most articles have fewer than this; the cap exists so "get me the figures of
/// PMC…" on a 40-figure review does not produce a response no client can hold.
const DEFAULT_MAX_FIGURES: usize = 8;

/// Inline bytes returned when the caller does not say otherwise (8 MiB).
///
/// Counted before base64, over the whole response.
const DEFAULT_MAX_TOTAL_BYTES: u64 = 8 * 1024 * 1024;

/// MIME types returned as an MCP `image` content block.
///
/// Everything else becomes an embedded resource blob: `image/svg+xml`,
/// `image/tiff`, `application/pdf` and `application/postscript` are all valid
/// figure formats in PMC, but clients that render an `image` block expect a
/// raster they can decode.
const RENDERABLE_IMAGE_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

/// Request parameters for the `get_pmc_figure_images` tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FigureImagesRequest {
    #[schemars(description = "PMC ID (e.g., 'PMC7906746' or '7906746')")]
    pub pmc_id: String,
    #[schemars(
        description = "Figure ids or labels to fetch (e.g. ['fig1', 'Figure 2']). Omit for all figures."
    )]
    pub figure_ids: Option<Vec<String>>,
    #[schemars(
        description = "Maximum number of figures to return, in document order (default: 8). Figures beyond the cap are not downloaded."
    )]
    pub max_figures: Option<usize>,
    #[schemars(
        description = "Maximum total image bytes to inline, before base64 encoding (default: 8388608). Figures that would exceed it are reported with their metadata but no bytes."
    )]
    pub max_total_bytes: Option<u64>,
}

/// One figure's metadata plus what happened to its bytes.
#[derive(Debug, Serialize, JsonSchema)]
pub struct FigureImageOut {
    /// Figure metadata, as reported by `get_pmc_figures`.
    #[serde(flatten)]
    pub figure: FigureOut,
    /// Name of the image file in the article's Open Access package.
    pub file_name: String,
    /// MIME type of the image.
    pub mime_type: String,
    /// Size of the image in bytes, before base64 encoding.
    pub byte_size: u64,
    /// Image width in pixels, for formats that declare it in their header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Image height in pixels, for formats that declare it in their header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Whether this figure's bytes are in the response's content blocks.
    pub included: bool,
    /// Why the bytes were left out, when `included` is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omitted_reason: Option<String>,
}

/// Structured answer of the `get_pmc_figure_images` tool.
///
/// Mirrors the content blocks: every figure appears here, and the ones with
/// `included: true` appear again, in the same order, as image or resource
/// content.
#[derive(Debug, Serialize, JsonSchema)]
pub struct FigureImagesOutput {
    /// The PMC ID that was fetched.
    pub pmc_id: String,
    /// Number of figures described (included or not).
    pub figure_count: usize,
    /// Number of figures whose bytes are in the content blocks.
    pub included_count: usize,
    /// Total inlined bytes, before base64 encoding.
    pub included_bytes: u64,
    /// Figures, in document order.
    pub figures: Vec<FigureImageOut>,
}

/// Get PMC figure images as inline bytes
pub async fn get_pmc_figure_images(
    server: &super::PubMedServer,
    Parameters(params): Parameters<FigureImagesRequest>,
) -> Result<CallToolResult, ErrorData> {
    let pmc_id = normalize_pmc_id(&params.pmc_id);

    let max_figures = params.max_figures.unwrap_or(DEFAULT_MAX_FIGURES);
    if max_figures == 0 {
        return Err(invalid_params("max_figures must be greater than 0"));
    }
    let max_total_bytes = params.max_total_bytes.unwrap_or(DEFAULT_MAX_TOTAL_BYTES);

    let mut selection = FigureSelection::new().with_limit(max_figures);
    if let Some(ids) = params.figure_ids {
        selection = selection.with_ids(ids);
    }

    info!(
        pmc_id = %pmc_id,
        max_figures,
        max_total_bytes,
        "Fetching PMC figure images"
    );

    let blobs = server
        .client
        .pmc
        .fetch_figures_with(&pmc_id, &selection)
        .await
        .map_err(|e| internal_error(format!("Failed to fetch PMC figures: {}", e)))?;

    let mut contents = Vec::new();
    let mut figures = Vec::new();
    let mut included_bytes: u64 = 0;

    for blob in blobs {
        let byte_size = blob.data.len() as u64;
        // Compare against the remaining budget rather than the running total so
        // a single oversized figure is skipped instead of rejecting the call.
        let fits = included_bytes.saturating_add(byte_size) <= max_total_bytes;

        let omitted_reason = (!fits).then(|| {
            format!(
                "{} bytes would exceed the {} byte budget; raise max_total_bytes or request this figure on its own",
                byte_size, max_total_bytes
            )
        });

        if fits {
            contents.push(content_block(
                &blob.file_name,
                &blob.content_type,
                &blob.data,
            ));
            included_bytes += byte_size;
        }

        figures.push(FigureImageOut {
            figure: FigureOut::from(&blob.figure),
            file_name: blob.file_name,
            mime_type: blob.content_type,
            byte_size,
            width: blob.dimensions.map(|(width, _)| width),
            height: blob.dimensions.map(|(_, height)| height),
            included: fits,
            omitted_reason,
        });
    }

    let output = FigureImagesOutput {
        pmc_id,
        figure_count: figures.len(),
        included_count: figures.iter().filter(|figure| figure.included).count(),
        included_bytes,
        figures,
    };

    let structured = serde_json::to_value(&output)
        .map_err(|e| internal_error(format!("Failed to serialize figure metadata: {}", e)))?;

    // The images ride in `content` and their metadata in `structuredContent`,
    // so a client that reads only one of the two still gets something useful.
    let mut result = CallToolResult::success(contents);
    result.structured_content = Some(structured);
    Ok(result)
}

/// Wrap a figure's bytes in the content block its format warrants.
fn content_block(file_name: &str, mime_type: &str, data: &[u8]) -> ContentBlock {
    let encoded = BASE64.encode(data);

    if RENDERABLE_IMAGE_TYPES.contains(&mime_type) {
        ContentBlock::image(encoded, mime_type)
    } else {
        ContentBlock::resource(
            ResourceContents::blob(encoded, format!("pmc-figure://{}", file_name))
                .with_mime_type(mime_type),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasters_become_image_blocks() {
        let block = content_block("gr1_lrg.jpg", "image/jpeg", b"\xff\xd8\xff");
        let ContentBlock::Image(image) = block else {
            panic!("a JPEG should be an image content block, got: {block:?}");
        };
        assert_eq!(image.mime_type, "image/jpeg");
        assert_eq!(BASE64.decode(image.data).unwrap(), b"\xff\xd8\xff");
    }

    /// A vector figure is still a figure: it must come back as a blob rather
    /// than be dropped or mislabelled as a renderable image.
    #[test]
    fn vector_figures_become_resource_blobs_carrying_their_mime_type() {
        let block = content_block("fig1.pdf", "application/pdf", b"%PDF-1.4");
        let ContentBlock::Resource(resource) = block else {
            panic!("a PDF figure should be an embedded resource, got: {block:?}");
        };
        let ResourceContents::BlobResourceContents {
            uri,
            mime_type,
            blob,
            ..
        } = resource.resource
        else {
            panic!("a PDF figure should be carried as a blob, not as text");
        };
        assert_eq!(uri, "pmc-figure://fig1.pdf");
        assert_eq!(mime_type.as_deref(), Some("application/pdf"));
        assert_eq!(BASE64.decode(blob).unwrap(), b"%PDF-1.4");
    }
}
