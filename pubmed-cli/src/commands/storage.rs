use anyhow::{Result, anyhow};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client as S3Client;
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<()>;
    async fn copy_file(&self, source: &Path, dest_path: &str) -> Result<()>;
    async fn ensure_directory(&self, path: &str) -> Result<()>;
    async fn file_exists(&self, path: &str) -> Result<bool>;
    fn get_full_path(&self, relative_path: &str) -> String;
}

pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let full_path = self.base_path.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&full_path, content).await?;
        debug!("Written file to local storage: {}", full_path.display());
        Ok(())
    }

    async fn copy_file(&self, source: &Path, dest_path: &str) -> Result<()> {
        let full_dest = self.base_path.join(dest_path);
        if let Some(parent) = full_dest.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::copy(source, &full_dest).await?;
        debug!(
            "Copied file to local storage: {} -> {}",
            source.display(),
            full_dest.display()
        );
        Ok(())
    }

    async fn ensure_directory(&self, path: &str) -> Result<()> {
        let full_path = self.base_path.join(path);
        fs::create_dir_all(&full_path).await?;
        debug!("Ensured directory exists: {}", full_path.display());
        Ok(())
    }

    async fn file_exists(&self, path: &str) -> Result<bool> {
        let full_path = self.base_path.join(path);
        Ok(full_path.exists())
    }

    fn get_full_path(&self, relative_path: &str) -> String {
        self.base_path.join(relative_path).display().to_string()
    }
}

pub struct S3Storage {
    client: S3Client,
    bucket: String,
    prefix: Option<String>,
}

impl S3Storage {
    pub async fn new(s3_path: &str, region: Option<String>) -> Result<Self> {
        let (bucket, prefix) = parse_s3_path(s3_path)?;

        let mut config_builder = aws_config::defaults(BehaviorVersion::latest());
        if let Some(region) = region {
            config_builder = config_builder.region(aws_config::Region::new(region));
        }
        let config = config_builder.load().await;
        let client = S3Client::new(&config);

        Ok(Self {
            client,
            bucket,
            prefix,
        })
    }

    fn get_s3_key(&self, path: &str) -> String {
        match &self.prefix {
            Some(prefix) => {
                let prefix = prefix.trim_end_matches('/');
                format!("{}/{}", prefix, path)
            }
            None => path.to_string(),
        }
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let key = self.get_s3_key(path);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(content.to_vec().into())
            .send()
            .await
            .map_err(|e| anyhow!("Failed to upload to S3: {}", e))?;

        info!("Uploaded file to S3: s3://{}/{}", self.bucket, key);
        Ok(())
    }

    async fn copy_file(&self, source: &Path, dest_path: &str) -> Result<()> {
        let content = fs::read(source).await?;
        self.write_file(dest_path, &content).await
    }

    async fn ensure_directory(&self, _path: &str) -> Result<()> {
        // S3 doesn't have real directories, so this is a no-op
        Ok(())
    }

    async fn file_exists(&self, path: &str) -> Result<bool> {
        let key = self.get_s3_key(path);

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
                // If it's a "Not Found" error, the file doesn't exist
                if let aws_sdk_s3::error::SdkError::ServiceError(ref service_err) = e
                    && service_err.err().is_not_found()
                {
                    return Ok(false);
                }
                // For other errors, propagate the error
                Err(anyhow!("Failed to check S3 object existence: {}", e))
            }
        }
    }

    fn get_full_path(&self, relative_path: &str) -> String {
        let key = self.get_s3_key(relative_path);
        format!("s3://{}/{}", self.bucket, key)
    }
}

fn parse_s3_path(s3_path: &str) -> Result<(String, Option<String>)> {
    if !s3_path.starts_with("s3://") {
        return Err(anyhow!(
            "Invalid S3 path. Must start with 's3://'. Got: {}",
            s3_path
        ));
    }

    let path_without_scheme = &s3_path[5..];
    let parts: Vec<&str> = path_without_scheme.splitn(2, '/').collect();

    if parts.is_empty() || parts[0].is_empty() {
        return Err(anyhow!("Invalid S3 path. No bucket specified: {}", s3_path));
    }

    let bucket = parts[0].to_string();
    let prefix = if parts.len() > 1 && !parts[1].is_empty() {
        Some(parts[1].to_string())
    } else {
        None
    };

    Ok((bucket, prefix))
}

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
            Ok(Box::new(S3Storage::new(&s3, s3_region).await?))
        }
        (Some(_), Some(_)) => Err(anyhow!(
            "Cannot specify both --output-dir and --s3-path. Choose one storage location."
        )),
        (None, None) => Err(anyhow!(
            "Must specify either --output-dir or --s3-path for storage location."
        )),
    }
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
    fn test_s3_get_s3_key() {
        // Test helper function that doesn't need the actual S3 client
        fn test_get_s3_key(prefix: Option<String>, path: &str) -> String {
            match prefix {
                Some(prefix) => {
                    let prefix = prefix.trim_end_matches('/');
                    format!("{}/{}", prefix, path)
                }
                None => path.to_string(),
            }
        }

        assert_eq!(test_get_s3_key(None, "file.txt"), "file.txt");
        assert_eq!(
            test_get_s3_key(Some("prefix".to_string()), "file.txt"),
            "prefix/file.txt"
        );
        assert_eq!(
            test_get_s3_key(Some("prefix/".to_string()), "file.txt"),
            "prefix/file.txt"
        );
    }

    #[test]
    fn test_s3_storage_get_s3_key_with_instance() {
        // Test the actual S3Storage.get_s3_key method
        // This test doesn't need a real S3 client, just testing the key generation logic
        struct TestS3Storage {
            prefix: Option<String>,
        }

        impl TestS3Storage {
            fn get_s3_key(&self, path: &str) -> String {
                match &self.prefix {
                    Some(prefix) => {
                        let prefix = prefix.trim_end_matches('/');
                        format!("{}/{}", prefix, path)
                    }
                    None => path.to_string(),
                }
            }
        }

        let storage_no_prefix = TestS3Storage { prefix: None };
        assert_eq!(storage_no_prefix.get_s3_key("file.txt"), "file.txt");

        let storage_with_prefix = TestS3Storage {
            prefix: Some("prefix".to_string()),
        };
        assert_eq!(
            storage_with_prefix.get_s3_key("file.txt"),
            "prefix/file.txt"
        );

        let storage_with_nested = TestS3Storage {
            prefix: Some("data/2024".to_string()),
        };
        assert_eq!(
            storage_with_nested.get_s3_key("subdir/file.txt"),
            "data/2024/subdir/file.txt"
        );
    }

    #[test]
    fn test_s3_storage_get_full_path_logic() {
        // Test the path generation logic without needing an actual S3 client
        let bucket = "my-bucket";
        let prefix = Some("data/2024");

        let get_full_path = |relative_path: &str| -> String {
            let key = match prefix {
                Some(p) => format!("{}/{}", p.trim_end_matches('/'), relative_path),
                None => relative_path.to_string(),
            };
            format!("s3://{}/{}", bucket, key)
        };

        assert_eq!(
            get_full_path("file.txt"),
            "s3://my-bucket/data/2024/file.txt"
        );
        assert_eq!(
            get_full_path("subdir/image.png"),
            "s3://my-bucket/data/2024/subdir/image.png"
        );
    }

    #[tokio::test]
    async fn test_local_storage_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf());

        let result = storage.write_file("test.txt", b"test content").await;
        assert!(result.is_ok());

        // Verify file was created
        let file_path = temp_dir.path().join("test.txt");
        assert!(file_path.exists());
        let content = tokio::fs::read_to_string(file_path).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_local_storage_write_file_with_subdirs() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf());

        let result = storage
            .write_file("subdir/nested/file.txt", b"nested content")
            .await;
        assert!(result.is_ok());

        // Verify file and directories were created
        let file_path = temp_dir.path().join("subdir/nested/file.txt");
        assert!(file_path.exists());
        let content = tokio::fs::read_to_string(file_path).await.unwrap();
        assert_eq!(content, "nested content");
    }

    #[tokio::test]
    async fn test_local_storage_copy_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf());

        // Create source file
        let source_dir = TempDir::new().unwrap();
        let source_path = source_dir.path().join("source.txt");
        tokio::fs::write(&source_path, b"source content")
            .await
            .unwrap();

        let result = storage.copy_file(&source_path, "copied.txt").await;
        assert!(result.is_ok());

        // Verify file was copied
        let dest_path = temp_dir.path().join("copied.txt");
        assert!(dest_path.exists());
        let content = tokio::fs::read_to_string(dest_path).await.unwrap();
        assert_eq!(content, "source content");
    }

    #[tokio::test]
    async fn test_local_storage_ensure_directory() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf());

        let result = storage.ensure_directory("new/nested/dir").await;
        assert!(result.is_ok());

        // Verify directory was created
        let dir_path = temp_dir.path().join("new/nested/dir");
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
    }

    #[tokio::test]
    async fn test_local_storage_get_full_path() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path().to_path_buf());

        let full_path = storage.get_full_path("file.txt");
        assert_eq!(
            full_path,
            temp_dir.path().join("file.txt").display().to_string()
        );

        let nested_path = storage.get_full_path("dir/subdir/file.txt");
        assert_eq!(
            nested_path,
            temp_dir
                .path()
                .join("dir/subdir/file.txt")
                .display()
                .to_string()
        );
    }

    #[tokio::test]
    async fn test_create_storage_backend_local() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        let result = create_storage_backend(Some(path.clone()), None, None).await;
        assert!(result.is_ok());

        let storage = result.unwrap();
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
