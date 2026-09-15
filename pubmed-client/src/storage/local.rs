//! Filesystem-backed [`StorageBackend`].

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use std::io::ErrorKind;
use tokio::fs;
use tracing::debug;

use super::{StorageBackend, storage_error};
use crate::error::Result;

/// Writes into a directory on the local filesystem.
///
/// Every path is resolved under `base_path`, and parent directories are created
/// on demand, so a download that organizes files into subdirectories needs no
/// setup beyond naming the root.
#[derive(Debug, Clone)]
pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    /// Write into `base_path`.
    ///
    /// The directory is not created here — [`StorageBackend::write_file`] and
    /// [`StorageBackend::ensure_directory`] create what they need, and
    /// [`Destination::into_backend`] creates the root up front.
    ///
    /// [`Destination::into_backend`]: super::Destination::into_backend
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let full_path = self.base_path.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                storage_error(format!(
                    "Failed to create directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        fs::write(&full_path, content)
            .await
            .map_err(|e| storage_error(format!("Failed to write {}: {e}", full_path.display())))?;
        debug!("Written file to local storage: {}", full_path.display());
        Ok(())
    }

    async fn copy_file(&self, source: &Path, dest_path: &str) -> Result<()> {
        let full_dest = self.base_path.join(dest_path);
        if let Some(parent) = full_dest.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                storage_error(format!(
                    "Failed to create directory {}: {e}",
                    parent.display()
                ))
            })?;
        }
        fs::copy(source, &full_dest).await.map_err(|e| {
            storage_error(format!(
                "Failed to copy {} to {}: {e}",
                source.display(),
                full_dest.display()
            ))
        })?;
        debug!(
            "Copied file to local storage: {} -> {}",
            source.display(),
            full_dest.display()
        );
        Ok(())
    }

    async fn ensure_directory(&self, path: &str) -> Result<()> {
        let full_path = self.base_path.join(path);
        fs::create_dir_all(&full_path).await.map_err(|e| {
            storage_error(format!(
                "Failed to create directory {}: {e}",
                full_path.display()
            ))
        })?;
        debug!("Ensured directory exists: {}", full_path.display());
        Ok(())
    }

    async fn file_exists(&self, path: &str) -> Result<bool> {
        Ok(self.base_path.join(path).exists())
    }

    async fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let full_path = self.base_path.join(path);
        match fs::read(&full_path).await {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(storage_error(format!(
                "Failed to read {}: {e}",
                full_path.display()
            ))),
        }
    }

    fn get_full_path(&self, relative_path: &str) -> String {
        self.base_path.join(relative_path).display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn write_file_creates_missing_parent_directories() {
        let temp = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp.path());

        storage
            .write_file("nested/deeper/file.txt", b"content")
            .await
            .unwrap();

        let written = temp.path().join("nested/deeper/file.txt");
        assert_eq!(fs::read(&written).await.unwrap(), b"content");
    }

    #[tokio::test]
    async fn a_missing_file_reads_as_none_rather_than_an_error() {
        let temp = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp.path());

        assert!(storage.read_file("absent.txt").await.unwrap().is_none());
        assert!(!storage.file_exists("absent.txt").await.unwrap());

        storage.write_file("present.txt", b"hello").await.unwrap();
        assert_eq!(
            storage.read_file("present.txt").await.unwrap(),
            Some(b"hello".to_vec())
        );
        assert!(storage.file_exists("present.txt").await.unwrap());
    }

    #[tokio::test]
    async fn copy_file_brings_a_local_file_in() {
        let temp = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let source = source_dir.path().join("source.txt");
        fs::write(&source, b"source content").await.unwrap();

        let storage = LocalStorage::new(temp.path());
        storage.copy_file(&source, "sub/copied.txt").await.unwrap();

        assert_eq!(
            fs::read(temp.path().join("sub/copied.txt")).await.unwrap(),
            b"source content"
        );
    }

    #[tokio::test]
    async fn ensure_directory_creates_the_whole_chain() {
        let temp = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp.path());

        storage.ensure_directory("new/nested/dir").await.unwrap();

        assert!(temp.path().join("new/nested/dir").is_dir());
    }

    #[tokio::test]
    async fn full_paths_are_resolved_under_the_root() {
        let temp = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp.path());

        assert_eq!(
            storage.get_full_path("dir/file.txt"),
            temp.path().join("dir/file.txt").display().to_string()
        );
    }

    /// A write into an unwritable location must surface as an error, not a
    /// silent no-op: a download that reports success having stored nothing is
    /// the worst outcome here.
    #[tokio::test]
    async fn a_failed_write_is_reported() {
        let temp = TempDir::new().unwrap();
        let blocker = temp.path().join("blocker");
        fs::write(&blocker, b"not a directory").await.unwrap();

        let storage = LocalStorage::new(temp.path());
        let err = storage
            .write_file("blocker/file.txt", b"content")
            .await
            .expect_err("writing under a regular file must fail");

        assert!(err.to_string().contains("blocker"), "{err}");
    }
}
