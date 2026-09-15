//! S3-compatible object storage [`StorageBackend`].
//!
//! Built on the AWS SDK, so credentials, region and endpoint come from the usual
//! `AWS_*` environment variables, shared config files, instance metadata, or
//! whatever else `aws-config` resolves. [`S3Options`] overrides the two things
//! an S3-compatible service that is not AWS invariably needs — a custom endpoint
//! and path-style addressing — so MinIO, Cloudflare R2, Ceph and friends work
//! without a separate backend.

use std::path::Path;

use async_trait::async_trait;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::config::Builder as S3ConfigBuilder;
use aws_sdk_s3::error::SdkError;
use std::env;
use tokio::fs;
use tracing::{debug, info};

use super::{StorageBackend, storage_error};
use crate::error::Result;

/// Connection overrides for an S3-compatible service.
///
/// Every field defaults to "whatever `aws-config` resolves", which is the right
/// answer for AWS itself.
#[derive(Debug, Clone, Default)]
pub struct S3Options {
    /// Region to use, overriding `AWS_REGION` and the shared config.
    pub region: Option<String>,
    /// Endpoint URL, for an S3-compatible service that is not AWS (MinIO, R2,
    /// Ceph). Also settable through `AWS_ENDPOINT_URL`/`AWS_ENDPOINT_URL_S3`.
    pub endpoint_url: Option<String>,
    /// Address buckets as `endpoint/bucket/key` instead of
    /// `bucket.endpoint/key`. Required by most non-AWS implementations, whose
    /// endpoints have no per-bucket DNS.
    pub force_path_style: bool,
}

impl S3Options {
    /// Resolve everything from the environment (the AWS default).
    pub fn new() -> Self {
        Self::default()
    }

    /// Read endpoint and addressing style from the environment.
    ///
    /// - `AWS_ENDPOINT_URL_S3`, else `AWS_ENDPOINT_URL` — the service endpoint,
    ///   for an S3-compatible store that is not AWS.
    /// - `AWS_S3_FORCE_PATH_STYLE` — `true`/`1`/`yes`/`on` (or their negatives)
    ///   to override the addressing style. A custom endpoint defaults it to
    ///   path-style, since such services rarely have per-bucket DNS.
    ///
    /// Region and credentials are deliberately not read here: `aws-config`
    /// already resolves those from the environment, shared config files and
    /// instance metadata, and second-guessing it would diverge from every other
    /// AWS tool on the machine.
    ///
    /// This is what [`Destination::into_backend`] uses, so `AWS_ENDPOINT_URL=…`
    /// is enough to point a download at MinIO, Cloudflare R2 or Ceph.
    ///
    /// [`Destination::into_backend`]: super::Destination::into_backend
    pub fn from_env() -> Self {
        let endpoint_url = env::var("AWS_ENDPOINT_URL_S3")
            .or_else(|_| env::var("AWS_ENDPOINT_URL"))
            .ok()
            .filter(|url| !url.trim().is_empty());

        let mut options = match endpoint_url {
            Some(url) => Self::new().with_endpoint_url(url.trim()),
            None => Self::new(),
        };

        if let Some(force_path_style) = env::var("AWS_S3_FORCE_PATH_STYLE")
            .ok()
            .as_deref()
            .and_then(parse_bool)
        {
            options = options.with_path_style(force_path_style);
        }

        options
    }

    /// Use `region` instead of whatever the environment resolves.
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Talk to an S3-compatible service at `endpoint_url`.
    ///
    /// Turns on [`force_path_style`] as well: a custom endpoint essentially
    /// always means a service without per-bucket DNS, and a virtual-host request
    /// against one fails in a way that is tedious to diagnose. Call
    /// [`with_path_style(false)`] afterwards for the rare endpoint that wants
    /// virtual-host addressing.
    ///
    /// [`force_path_style`]: Self::force_path_style
    /// [`with_path_style(false)`]: Self::with_path_style
    pub fn with_endpoint_url(mut self, endpoint_url: impl Into<String>) -> Self {
        self.endpoint_url = Some(endpoint_url.into());
        self.force_path_style = true;
        self
    }

    /// Choose path-style (`true`) or virtual-host (`false`) bucket addressing.
    pub fn with_path_style(mut self, force_path_style: bool) -> Self {
        self.force_path_style = force_path_style;
        self
    }
}

/// Parse the boolish spellings configuration files and container runtimes use.
///
/// Returns `None` for anything unrecognized, so a typo leaves the default in
/// place rather than silently meaning `false`.
fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// Writes into a bucket in S3-compatible object storage.
///
/// Paths are joined onto `prefix` to form the object key, so a download writes
/// the same relative names it would on a filesystem.
#[derive(Debug, Clone)]
pub struct S3Storage {
    client: S3Client,
    bucket: String,
    prefix: Option<String>,
}

impl S3Storage {
    /// Connect to `bucket`, placing every key under `prefix`.
    ///
    /// # Errors
    ///
    /// Returns an error if the bucket name is empty. Credential and endpoint
    /// problems surface on the first request rather than here — `aws-config`
    /// resolves lazily, and failing at construction would make a misconfigured
    /// region look like a missing bucket.
    pub async fn new(
        bucket: impl Into<String>,
        prefix: Option<String>,
        options: S3Options,
    ) -> Result<Self> {
        let bucket = bucket.into();
        if bucket.is_empty() {
            return Err(storage_error("S3 bucket name must not be empty"));
        }

        let mut loader = aws_config::defaults(BehaviorVersion::latest());
        if let Some(region) = options.region {
            loader = loader.region(Region::new(region));
        }
        if let Some(endpoint_url) = &options.endpoint_url {
            loader = loader.endpoint_url(endpoint_url);
        }
        let sdk_config = loader.load().await;

        let client = S3Client::from_conf(
            S3ConfigBuilder::from(&sdk_config)
                .force_path_style(options.force_path_style)
                .build(),
        );

        Ok(Self {
            client,
            bucket,
            prefix: prefix.map(|prefix| prefix.trim_matches('/').to_string()),
        })
    }

    /// Build the object key for a path relative to this backend's prefix.
    fn object_key(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        match self.prefix.as_deref().filter(|prefix| !prefix.is_empty()) {
            Some(prefix) => format!("{prefix}/{path}"),
            None => path.to_string(),
        }
    }

    /// The bucket this backend writes to.
    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    /// The key prefix every write is placed under, if any.
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let key = self.object_key(path);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(content.to_vec().into())
            .send()
            .await
            .map_err(|e| {
                storage_error(format!(
                    "Failed to upload to s3://{}/{key}: {e}",
                    self.bucket
                ))
            })?;

        info!("Uploaded file to S3: s3://{}/{}", self.bucket, key);
        Ok(())
    }

    async fn copy_file(&self, source: &Path, dest_path: &str) -> Result<()> {
        let content = fs::read(source)
            .await
            .map_err(|e| storage_error(format!("Failed to read {}: {e}", source.display())))?;
        self.write_file(dest_path, &content).await
    }

    async fn ensure_directory(&self, path: &str) -> Result<()> {
        // Object stores have no directories; a key containing `/` is enough.
        debug!("Skipping directory creation for S3 prefix: {}", path);
        Ok(())
    }

    async fn file_exists(&self, path: &str) -> Result<bool> {
        let key = self.object_key(path);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if let SdkError::ServiceError(ref service_err) = e
                    && service_err.err().is_not_found()
                {
                    return Ok(false);
                }
                Err(storage_error(format!(
                    "Failed to check s3://{}/{key}: {e}",
                    self.bucket
                )))
            }
        }
    }

    async fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let key = self.object_key(path);

        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(output) => {
                let bytes = output.body.collect().await.map_err(|e| {
                    storage_error(format!(
                        "Failed to read body of s3://{}/{key}: {e}",
                        self.bucket
                    ))
                })?;
                Ok(Some(bytes.into_bytes().to_vec()))
            }
            Err(e) => {
                if let SdkError::ServiceError(ref service_err) = e
                    && service_err.err().is_no_such_key()
                {
                    return Ok(None);
                }
                Err(storage_error(format!(
                    "Failed to read s3://{}/{key}: {e}",
                    self.bucket
                )))
            }
        }
    }

    fn get_full_path(&self, relative_path: &str) -> String {
        format!("s3://{}/{}", self.bucket, self.object_key(relative_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Key building is the part that silently corrupts a layout when wrong, and
    /// it needs no client — exercise it directly rather than behind a mock.
    fn storage(bucket: &str, prefix: Option<&str>) -> S3Storage {
        // A default-configured client never sends a request in these tests.
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("us-east-1"))
            .build();
        S3Storage {
            client: S3Client::from_conf(config),
            bucket: bucket.to_string(),
            prefix: prefix.map(|prefix| prefix.trim_matches('/').to_string()),
        }
    }

    #[test]
    fn keys_are_placed_under_the_prefix() {
        assert_eq!(storage("b", None).object_key("file.txt"), "file.txt");
        assert_eq!(
            storage("b", Some("pmc")).object_key("file.txt"),
            "pmc/file.txt"
        );
        assert_eq!(
            storage("b", Some("data/2024")).object_key("sub/file.txt"),
            "data/2024/sub/file.txt"
        );
    }

    /// Stray slashes must not produce `prefix//key` or a leading `/`: S3 treats
    /// those as distinct keys, so the files would land somewhere nobody looks.
    #[test]
    fn surrounding_slashes_never_double_up_in_a_key() {
        assert_eq!(
            storage("b", Some("/pmc/")).object_key("/file.txt"),
            "pmc/file.txt"
        );
        assert_eq!(storage("b", Some("")).object_key("file.txt"), "file.txt");
        assert_eq!(storage("b", None).object_key("/file.txt"), "file.txt");
    }

    #[test]
    fn full_paths_render_as_s3_uris() {
        assert_eq!(
            storage("my-bucket", Some("data/2024")).get_full_path("fig.png"),
            "s3://my-bucket/data/2024/fig.png"
        );
        assert_eq!(
            storage("my-bucket", None).get_full_path("fig.png"),
            "s3://my-bucket/fig.png"
        );
    }

    #[tokio::test]
    async fn an_empty_bucket_name_is_rejected() {
        assert!(
            S3Storage::new("", None, S3Options::default())
                .await
                .is_err()
        );
    }

    /// A custom endpoint means a non-AWS service, which needs path-style
    /// addressing; forgetting it yields DNS failures that look like outages.
    #[test]
    fn a_custom_endpoint_turns_on_path_style_addressing() {
        let options = S3Options::new().with_endpoint_url("http://localhost:9000");
        assert!(options.force_path_style);
        assert_eq!(
            options.endpoint_url.as_deref(),
            Some("http://localhost:9000")
        );

        // ...and it stays overridable for an endpoint that wants virtual hosts.
        assert!(
            !S3Options::new()
                .with_endpoint_url("http://localhost:9000")
                .with_path_style(false)
                .force_path_style
        );
    }

    #[test]
    fn boolish_spellings_are_all_accepted_and_typos_are_not() {
        for truthy in ["1", "true", "TRUE", "yes", " on "] {
            assert_eq!(parse_bool(truthy), Some(true), "{truthy:?}");
        }
        for falsy in ["0", "false", "No", "off"] {
            assert_eq!(parse_bool(falsy), Some(false), "{falsy:?}");
        }
        // A typo must leave the caller's default alone, not mean `false`.
        for unknown in ["", "maybe", "2"] {
            assert_eq!(parse_bool(unknown), None, "{unknown:?}");
        }
    }

    #[tokio::test]
    async fn directories_are_a_no_op_but_still_answer_ok() {
        assert!(
            storage("b", Some("pmc"))
                .ensure_directory("anything")
                .await
                .is_ok()
        );
    }
}
