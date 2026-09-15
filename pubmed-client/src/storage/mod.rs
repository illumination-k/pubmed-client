//! Download destinations: a local directory, or a bucket in object storage.
//!
//! PMC Open Access files have to land somewhere, and "somewhere" is not always a
//! filesystem — a batch job writes to S3, an MCP server may have no writable
//! directory at all. [`StorageBackend`] is the one seam every download goes
//! through, so [`PmcCloudClient`] does not care which it is.
//!
//! A caller who has a string from a user or a tool argument goes through
//! [`Destination`], which reads `s3://bucket/prefix` as object storage and
//! anything else as a local path.
//!
//! S3 support is behind the `storage-s3` feature: it pulls in the AWS SDK, which
//! nothing using only local downloads should have to build.
//!
//! [`PmcCloudClient`]: crate::pmc::cloud::PmcCloudClient

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs;

use crate::error::{ParseError, PubMedError, Result};

mod local;
pub use local::LocalStorage;

#[cfg(feature = "storage-s3")]
mod s3;
#[cfg(feature = "storage-s3")]
pub use s3::{S3Options, S3Storage};

/// Where downloaded files are written.
///
/// Paths handed to these methods are relative to the backend's own root (a
/// directory for [`LocalStorage`], a bucket and key prefix for `S3Storage`), so
/// the same download code works against either.
///
/// Object stores have no directories, so `ensure_directory` is a no-op there;
/// call it anyway, and let the backend decide whether it means anything.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Write `content` at `path`, creating any parent directories it needs.
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<()>;

    /// Copy a local file into the backend at `dest_path`.
    ///
    /// Separate from [`write_file`] so a local backend can move bytes without
    /// reading them into memory first.
    ///
    /// [`write_file`]: Self::write_file
    async fn copy_file(&self, source: &Path, dest_path: &str) -> Result<()>;

    /// Make sure `path` can be written into. A no-op for object stores.
    async fn ensure_directory(&self, path: &str) -> Result<()>;

    /// Whether anything exists at `path`.
    async fn file_exists(&self, path: &str) -> Result<bool>;

    /// Read the contents of `path`, or `None` if nothing is there.
    async fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>>;

    /// Render `relative_path` as a location a human can act on — an absolute
    /// filesystem path, or an `s3://bucket/key` URI.
    fn get_full_path(&self, relative_path: &str) -> String;
}

/// Build a storage error.
///
/// Storage failures reuse [`ParseError::IoError`], which the local download path
/// has always used for a failed write; an object store refusing a `PutObject` is
/// the same kind of problem to a caller, and callers that match on error
/// variants (the language bindings) keep working unchanged.
pub(crate) fn storage_error(message: impl Into<String>) -> PubMedError {
    ParseError::IoError {
        message: message.into(),
    }
    .into()
}

/// A download destination parsed from a caller-supplied string.
///
/// Anything starting with `s3://` is object storage; everything else is a local
/// path. That keeps one argument (`--output-dir`, an `output_dir` tool
/// parameter) able to mean either without a second "which kind is it?" flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// A directory on the local filesystem.
    Local(PathBuf),
    /// A bucket in S3-compatible object storage, with an optional key prefix.
    S3 {
        /// Bucket name.
        bucket: String,
        /// Key prefix within the bucket, if the URI named one.
        prefix: Option<String>,
    },
}

impl Destination {
    /// Parse a destination string.
    ///
    /// # Errors
    ///
    /// Returns an error for an `s3://` URI with no bucket, and for an empty
    /// string — a blank destination is a mistake, not the current directory.
    pub fn parse(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(storage_error("Destination must not be empty"));
        }

        match trimmed.strip_prefix("s3://") {
            Some(rest) => {
                let (bucket, prefix) = match rest.split_once('/') {
                    Some((bucket, prefix)) => (bucket, prefix.trim_matches('/')),
                    None => (rest, ""),
                };
                if bucket.is_empty() {
                    return Err(storage_error(format!(
                        "Invalid S3 URI, no bucket specified: {value}"
                    )));
                }
                Ok(Self::S3 {
                    bucket: bucket.to_string(),
                    prefix: (!prefix.is_empty()).then(|| prefix.to_string()),
                })
            }
            None => Ok(Self::Local(PathBuf::from(trimmed))),
        }
    }

    /// Whether this destination is object storage rather than the filesystem.
    pub fn is_object_storage(&self) -> bool {
        matches!(self, Self::S3 { .. })
    }

    /// Render the destination the way it was addressed.
    pub fn display(&self) -> String {
        match self {
            Self::Local(path) => path.display().to_string(),
            Self::S3 { bucket, prefix } => match prefix {
                Some(prefix) => format!("s3://{bucket}/{prefix}"),
                None => format!("s3://{bucket}"),
            },
        }
    }

    /// Build the backend this destination names.
    ///
    /// A local destination is created as a directory up front, so a caller
    /// learns immediately that the path is unusable rather than after a download
    /// has already run.
    ///
    /// An object-storage destination is configured from the environment via
    /// [`S3Options::from_env`], so `AWS_ENDPOINT_URL` is enough to target MinIO,
    /// Cloudflare R2 or Ceph rather than AWS.
    ///
    /// # Errors
    ///
    /// Returns an error if a local directory cannot be created, if the AWS
    /// configuration cannot be loaded, or — when built without the `storage-s3`
    /// feature — if the destination is an `s3://` URI.
    pub async fn into_backend(self) -> Result<Box<dyn StorageBackend>> {
        match self {
            Self::Local(path) => {
                fs::create_dir_all(&path).await.map_err(|e| {
                    storage_error(format!(
                        "Failed to create output directory {}: {e}",
                        path.display()
                    ))
                })?;
                Ok(Box::new(LocalStorage::new(path)))
            }
            #[cfg(feature = "storage-s3")]
            Self::S3 { bucket, prefix } => Ok(Box::new(
                S3Storage::new(bucket, prefix, S3Options::from_env()).await?,
            )),
            #[cfg(not(feature = "storage-s3"))]
            Self::S3 { bucket, .. } => Err(storage_error(format!(
                "Object storage destination s3://{bucket} requires the `storage-s3` feature, \
                 which this build does not enable"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_path_is_a_local_destination() {
        assert_eq!(
            Destination::parse("./figures").unwrap(),
            Destination::Local(PathBuf::from("./figures"))
        );
        assert_eq!(
            Destination::parse("/var/tmp/pmc").unwrap(),
            Destination::Local(PathBuf::from("/var/tmp/pmc"))
        );
        // Surrounding whitespace is the caller's typo, not part of the path.
        assert_eq!(
            Destination::parse("  ./figures \n").unwrap(),
            Destination::Local(PathBuf::from("./figures"))
        );
    }

    #[test]
    fn an_s3_uri_splits_into_a_bucket_and_prefix() {
        assert_eq!(
            Destination::parse("s3://my-bucket").unwrap(),
            Destination::S3 {
                bucket: "my-bucket".to_string(),
                prefix: None,
            }
        );
        // A bare bucket with a trailing slash has no prefix, not an empty one:
        // an empty prefix would put every key behind a leading `/`.
        assert_eq!(
            Destination::parse("s3://my-bucket/").unwrap(),
            Destination::S3 {
                bucket: "my-bucket".to_string(),
                prefix: None,
            }
        );
        assert_eq!(
            Destination::parse("s3://my-bucket/pmc/figures/").unwrap(),
            Destination::S3 {
                bucket: "my-bucket".to_string(),
                prefix: Some("pmc/figures".to_string()),
            }
        );
    }

    #[test]
    fn a_blank_or_bucketless_destination_is_rejected() {
        for invalid in ["", "   ", "s3://", "s3:///prefix"] {
            assert!(
                Destination::parse(invalid).is_err(),
                "{invalid:?} should not parse"
            );
        }
    }

    #[test]
    fn a_destination_renders_the_way_it_was_addressed() {
        assert_eq!(
            Destination::parse("s3://bucket/prefix").unwrap().display(),
            "s3://bucket/prefix"
        );
        assert_eq!(
            Destination::parse("s3://bucket").unwrap().display(),
            "s3://bucket"
        );
        assert_eq!(
            Destination::parse("./out").unwrap().display(),
            PathBuf::from("./out").display().to_string()
        );
    }

    #[test]
    fn only_s3_counts_as_object_storage() {
        assert!(
            Destination::parse("s3://bucket")
                .unwrap()
                .is_object_storage()
        );
        assert!(!Destination::parse("./out").unwrap().is_object_storage());
    }
}
