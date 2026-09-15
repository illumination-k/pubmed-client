//! CLI-shaped wrappers over `pubmed-client`'s storage backends.
//!
//! The backends themselves ([`StorageBackend`], [`LocalStorage`], [`S3Storage`])
//! live in `pubmed_client::storage`, so the CLI and the MCP server write to
//! object storage through one implementation. What stays here is the argument
//! plumbing: the CLI takes a local path and an S3 URI as two mutually exclusive
//! flags, and reports conflicts in the CLI's own wording.

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::info;

pub use pubmed_client::storage::{Destination, LocalStorage, S3Options, S3Storage, StorageBackend};

/// Build a storage backend for a command that writes a tree of files.
///
/// Exactly one of `output_dir` / `s3_path` must be set.
pub async fn create_storage_backend(
    output_dir: Option<PathBuf>,
    s3_path: Option<String>,
    s3_region: Option<String>,
) -> Result<Box<dyn StorageBackend>> {
    match (output_dir, s3_path) {
        (Some(dir), None) => {
            fs::create_dir_all(&dir).await?;
            info!("Using local storage: {}", dir.display());
            Ok(Box::new(LocalStorage::new(dir)))
        }
        (None, Some(s3)) => {
            info!("Using S3 storage: {}", s3);
            Ok(s3_backend(&s3, s3_region).await?)
        }
        (Some(_), Some(_)) => Err(anyhow!(
            "Cannot specify both --output-dir and --s3-path. Choose one storage location."
        )),
        (None, None) => Err(anyhow!(
            "Must specify either --output-dir or --s3-path for storage location."
        )),
    }
}

/// Build a storage backend plus the relative key for a command that writes a
/// single named file (e.g. `metadata.jsonl`, `citations.bib`).
///
/// - Local: `output` (default `default_filename`) is split into a parent
///   directory (the storage root, created if needed) and a filename (the key).
/// - S3: `s3_path` must be the full object path, e.g.
///   `s3://bucket/prefix/metadata.jsonl`; the last path segment becomes the key.
///
/// Exactly one of `output` / `s3_path` may be set.
pub async fn create_file_storage(
    output: Option<PathBuf>,
    s3_path: Option<String>,
    s3_region: Option<String>,
    default_filename: &str,
) -> Result<(Box<dyn StorageBackend>, String)> {
    match (output, s3_path) {
        (Some(_), Some(_)) => Err(anyhow!(
            "Cannot specify both --output and --s3-path. Choose one storage location."
        )),
        (output, None) => {
            let path = output.unwrap_or_else(|| PathBuf::from(default_filename));
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow!("Invalid output path: {}", path.display()))?
                .to_string();
            let base = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            fs::create_dir_all(&base).await?;
            info!("Using local storage: {}", base.display());
            Ok((Box::new(LocalStorage::new(base)), filename))
        }
        (None, Some(s3)) => {
            let (prefix_path, filename) = split_s3_object_path(&s3)?;
            info!("Using S3 storage: {}", s3);
            Ok((s3_backend(&prefix_path, s3_region).await?, filename))
        }
    }
}

/// Build an [`S3Storage`] from an `s3://bucket/prefix` URI.
async fn s3_backend(s3_path: &str, region: Option<String>) -> Result<Box<dyn StorageBackend>> {
    let (bucket, prefix) = parse_s3_path(s3_path)?;

    // Endpoint and addressing style come from the environment (so `--s3-path`
    // works against MinIO/R2); `--s3-region` overrides what aws-config resolves.
    let mut options = S3Options::from_env();
    if let Some(region) = region {
        options = options.with_region(region);
    }

    Ok(Box::new(S3Storage::new(bucket, prefix, options).await?))
}

/// Split an `s3://bucket/prefix` URI into its bucket and optional prefix.
///
/// A thin wrapper over [`Destination::parse`] that rejects a local path: the
/// CLI's `--s3-path` flag has already committed to object storage, so a value
/// without the scheme is a mistake worth naming rather than silently treating as
/// a directory.
fn parse_s3_path(s3_path: &str) -> Result<(String, Option<String>)> {
    match Destination::parse(s3_path)? {
        Destination::S3 { bucket, prefix } => Ok((bucket, prefix)),
        Destination::Local(_) => Err(anyhow!(
            "Invalid S3 path. Must start with 's3://'. Got: {}",
            s3_path
        )),
    }
}

/// Split a full S3 object path into a prefix path (`s3://bucket/prefix`) and a
/// filename (the final segment).
fn split_s3_object_path(s3_path: &str) -> Result<(String, String)> {
    if !s3_path.starts_with("s3://") {
        return Err(anyhow!(
            "Invalid S3 path. Must start with 's3://'. Got: {}",
            s3_path
        ));
    }

    let trimmed = s3_path.trim_end_matches('/');
    let (prefix_path, filename) = trimmed
        .rsplit_once('/')
        .filter(|(prefix, name)| *prefix != "s3:/" && !name.is_empty())
        .ok_or_else(|| {
            anyhow!(
                "Invalid S3 object path. Must include a bucket and object key, e.g. \
                 's3://bucket/path/file.ext'. Got: {}",
                s3_path
            )
        })?;

    Ok((prefix_path.to_string(), filename.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_s3_path() {
        assert_eq!(
            parse_s3_path("s3://bucket").unwrap(),
            ("bucket".to_string(), None)
        );
        assert_eq!(
            parse_s3_path("s3://bucket/").unwrap(),
            ("bucket".to_string(), None)
        );
        assert_eq!(
            parse_s3_path("s3://bucket/prefix").unwrap(),
            ("bucket".to_string(), Some("prefix".to_string()))
        );
        assert_eq!(
            parse_s3_path("s3://bucket/prefix/subdir").unwrap(),
            ("bucket".to_string(), Some("prefix/subdir".to_string()))
        );
        assert!(parse_s3_path("bucket/prefix").is_err());
        assert!(parse_s3_path("s3://").is_err());
    }

    #[test]
    fn test_split_s3_object_path() {
        assert_eq!(
            split_s3_object_path("s3://bucket/file.jsonl").unwrap(),
            ("s3://bucket".to_string(), "file.jsonl".to_string())
        );
        assert_eq!(
            split_s3_object_path("s3://bucket/a/b/file.jsonl").unwrap(),
            ("s3://bucket/a/b".to_string(), "file.jsonl".to_string())
        );
        // Missing object key
        assert!(split_s3_object_path("s3://bucket").is_err());
        assert!(split_s3_object_path("s3://bucket/").is_err());
        // Missing scheme
        assert!(split_s3_object_path("bucket/file.jsonl").is_err());
    }

    #[tokio::test]
    async fn test_create_file_storage_local_default() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("metadata.jsonl");

        let (storage, key) = create_file_storage(Some(path.clone()), None, None, "metadata.jsonl")
            .await
            .unwrap();
        assert_eq!(key, "metadata.jsonl");

        storage.write_file(&key, b"line\n").await.unwrap();
        assert!(path.exists());
        assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), "line\n");
    }

    #[tokio::test]
    async fn test_create_file_storage_conflict() {
        let result = create_file_storage(
            Some(PathBuf::from("out.jsonl")),
            Some("s3://bucket/out.jsonl".to_string()),
            None,
            "metadata.jsonl",
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_storage_backend_local() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let storage = create_storage_backend(Some(path.clone()), None, None)
            .await
            .unwrap();

        let full_path = storage.get_full_path("test.txt");
        assert!(full_path.contains(&path.display().to_string()));
    }

    #[tokio::test]
    async fn test_create_storage_backend_errors() {
        // Test both options specified
        let temp_dir = TempDir::new().unwrap();
        let result = create_storage_backend(
            Some(temp_dir.path().to_path_buf()),
            Some("s3://bucket".to_string()),
            None,
        )
        .await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Cannot specify both"));

        // Test neither option specified
        let result = create_storage_backend(None, None, None).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Must specify either"));
    }
}
