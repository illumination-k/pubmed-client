use pubmed_parser::pmc::Figure;
use serde::{Deserialize, Serialize};

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
