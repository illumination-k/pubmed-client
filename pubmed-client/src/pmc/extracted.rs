use pubmed_parser::pmc::Figure;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents an extracted figure with both XML metadata and file path.
///
/// This is a client-layer type that combines domain-level figure metadata
/// (from the parsed XML) with extraction concerns (file path, size, dimensions).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtractedFigure {
    /// Figure metadata from XML
    pub figure: Figure,
    /// Actual file path where the figure was extracted
    pub extracted_file_path: String,
    /// File size in bytes
    pub file_size: Option<u64>,
    /// Image dimensions (width, height) if available
    pub dimensions: Option<(u32, u32)>,
}

/// A figure's image bytes together with its XML metadata, fetched without
/// touching the filesystem.
///
/// This is what [`PmcCloudClient::fetch_figures`] returns: the same metadata as
/// [`ExtractedFigure`], but carrying the image itself instead of a path to it.
/// Useful for callers that forward the bytes somewhere else (an MCP response, a
/// web handler) and have nowhere sensible to write a file.
///
/// `data` is deliberately not serializable — a `Vec<u8>` would serde-encode as a
/// JSON array of integers, which is never what a caller wants. Encode it
/// yourself (base64, multipart, …) at the boundary that needs it.
///
/// [`PmcCloudClient::fetch_figures`]: crate::pmc::cloud::PmcCloudClient::fetch_figures
#[derive(Debug, Clone)]
pub struct FigureBlob {
    /// Figure metadata from XML
    pub figure: Figure,
    /// Name of the image file in the OA Cloud package (e.g. `gr1_lrg.jpg`)
    pub file_name: String,
    /// MIME type guessed from the file extension (e.g. `image/jpeg`)
    pub content_type: String,
    /// The image bytes
    pub data: Vec<u8>,
    /// Image dimensions (width, height) if the format exposes them in its header
    pub dimensions: Option<(u32, u32)>,
}

impl FigureBlob {
    /// Guess a MIME type from a figure file name's extension.
    ///
    /// The OA Cloud serves figures in a small, stable set of formats; anything
    /// unrecognized falls back to `application/octet-stream` so callers can
    /// still hand the bytes on rather than dropping them.
    pub(crate) fn guess_content_type(file_name: &str) -> String {
        let ext = Path::new(file_name)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "tif" | "tiff" => "image/tiff",
            "bmp" => "image/bmp",
            "pdf" => "application/pdf",
            "eps" => "application/postscript",
            _ => "application/octet-stream",
        }
        .to_string()
    }
}

/// Which of an article's figures to fetch, and how many.
///
/// The default selects every figure with no cap, which is what
/// [`PmcCloudClient::fetch_figures`] uses.
///
/// [`PmcCloudClient::fetch_figures`]: crate::pmc::cloud::PmcCloudClient::fetch_figures
#[derive(Debug, Clone, Default)]
pub struct FigureSelection {
    /// Figure ids or labels to fetch; empty selects every figure.
    ///
    /// Each entry is matched against a figure's `id` (e.g. `"fig1"`) and its
    /// `label` (e.g. `"Figure 1"`), both lowercased with spaces and dots
    /// removed — a caller quoting a caption should not have to know the XML id.
    /// Entries that match nothing are simply absent from the result.
    pub ids: Vec<String>,
    /// Maximum number of figures to fetch, in document order; `None` fetches
    /// every selected figure.
    pub limit: Option<usize>,
}

impl FigureSelection {
    /// Select every figure, with no cap.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict the selection to the given figure ids or labels.
    pub fn with_ids<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.ids = ids.into_iter().map(Into::into).collect();
        self
    }

    /// Fetch at most `limit` figures, in document order.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_is_guessed_from_the_extension_case_insensitively() {
        assert_eq!(FigureBlob::guess_content_type("gr1_lrg.jpg"), "image/jpeg");
        assert_eq!(FigureBlob::guess_content_type("fig1.JPEG"), "image/jpeg");
        assert_eq!(FigureBlob::guess_content_type("fig1.png"), "image/png");
        assert_eq!(
            FigureBlob::guess_content_type("scheme.svg"),
            "image/svg+xml"
        );
        assert_eq!(
            FigureBlob::guess_content_type("figure.eps"),
            "application/postscript"
        );
    }

    /// An unknown or missing extension must still produce a usable MIME type:
    /// dropping the bytes because we can't name them would be worse.
    #[test]
    fn unknown_extensions_fall_back_to_octet_stream() {
        assert_eq!(
            FigureBlob::guess_content_type("figure.xyz"),
            "application/octet-stream"
        );
        assert_eq!(
            FigureBlob::guess_content_type("figure"),
            "application/octet-stream"
        );
    }
}
