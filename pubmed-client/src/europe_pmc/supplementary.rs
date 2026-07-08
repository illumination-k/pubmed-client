//! Europe PMC `supplementaryFiles` endpoint operations (native only).
//!
//! Europe PMC returns supplementary materials as a single ZIP archive. These
//! helpers fetch that archive either into memory or onto disk; ZIP extraction is
//! left to the caller to avoid pulling in an archive dependency.

use std::path::{Path, PathBuf};

use tokio::fs as tokio_fs;
use tracing::{info, instrument};

use pubmed_parser::ParseError;

use crate::error::{PubMedError, Result};

use super::client::EuropePmcClient;
use super::id::EuropePmcId;

impl EuropePmcClient {
    /// Fetch the supplementary-files ZIP archive for a record into memory.
    ///
    /// Returns the raw bytes of the ZIP (`/{source}/{id}/supplementaryFiles`).
    #[instrument(skip(self), fields(id = %id))]
    pub async fn fetch_supplementary_files(&self, id: &EuropePmcId) -> Result<Vec<u8>> {
        let endpoint = format!("{}/{}/supplementaryFiles", id.source, id.id);
        let response = self
            .executor()
            .get_endpoint(&self.base_url, &endpoint, &[])
            .await?;
        let bytes = response.bytes().await.map_err(PubMedError::from)?;
        Ok(bytes.to_vec())
    }

    /// Download the supplementary-files ZIP archive for a record to `output_path`.
    ///
    /// `output_path` is the full path of the ZIP file to write. Parent
    /// directories are created if needed. Returns the written path.
    #[instrument(skip(self, output_path), fields(id = %id))]
    pub async fn download_supplementary_files(
        &self,
        id: &EuropePmcId,
        output_path: impl AsRef<Path>,
    ) -> Result<PathBuf> {
        let output_path = output_path.as_ref().to_path_buf();
        if let Some(parent) = output_path.parent() {
            tokio_fs::create_dir_all(parent)
                .await
                .map_err(|e| ParseError::IoError {
                    message: format!("failed to create directory {}: {e}", parent.display()),
                })?;
        }

        let bytes = self.fetch_supplementary_files(id).await?;
        tokio_fs::write(&output_path, &bytes)
            .await
            .map_err(|e| ParseError::IoError {
                message: format!("failed to write {}: {e}", output_path.display()),
            })?;

        info!(id = %id, path = %output_path.display(), bytes = bytes.len(), "Downloaded supplementary files");
        Ok(output_path)
    }
}
