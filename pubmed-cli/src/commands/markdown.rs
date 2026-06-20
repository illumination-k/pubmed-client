use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use pubmed_client::pmc::{MarkdownConfig, PmcMarkdownConverter};
use tempfile::TempDir;

use super::ClientContext;
use crate::commands::batch::{
    BatchItemError, BatchProcessor, FailureKind, classify_fetch_error, report_failures,
};
use crate::commands::storage::{StorageBackend, create_storage_backend};

#[derive(Args, Debug)]
pub struct Markdown {
    /// PMC IDs to convert to markdown
    #[arg(required = true)]
    pmcids: Vec<String>,

    /// Output directory for markdown files (local storage)
    #[arg(short, long, conflicts_with = "s3_path")]
    output_dir: Option<PathBuf>,

    /// S3 path for markdown files (e.g., s3://bucket/prefix)
    #[arg(long, conflicts_with = "output_dir")]
    s3_path: Option<String>,

    /// AWS region for S3 (optional, uses default AWS config if not specified)
    #[arg(long, requires = "s3_path")]
    s3_region: Option<String>,

    /// Path to save failed PMC IDs (if not specified, failures are logged only)
    #[arg(short, long)]
    failed_output: Option<PathBuf>,

    /// Download and embed figures in the markdown
    #[arg(long)]
    with_figures: bool,

    /// Use YAML frontmatter for metadata
    #[arg(long)]
    frontmatter: bool,
}

impl Markdown {
    pub async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        // Default to the current directory when no explicit destination is given.
        let output_dir = match (&self.output_dir, &self.s3_path) {
            (None, None) => Some(PathBuf::from(".")),
            (dir, _) => dir.clone(),
        };
        let storage =
            create_storage_backend(output_dir, self.s3_path.clone(), self.s3_region.clone())
                .await?;

        let client = ctx.pmc_client();

        let mut processor = BatchProcessor::new(self.pmcids.len())?;
        processor
            .run(&self.pmcids, async |_multi_progress, pmcid| {
                self.process_article(&client, pmcid, storage.as_ref(), ctx)
                    .await
            })
            .await;
        processor.finish();

        report_failures(
            processor.failures(),
            self.failed_output.clone(),
            storage.as_ref(),
        )
        .await
    }

    async fn process_article(
        &self,
        client: &pubmed_client::pmc::PmcClient,
        pmcid: &str,
        storage: &dyn StorageBackend,
        ctx: &ClientContext<'_>,
    ) -> Result<(), BatchItemError> {
        let article = client.fetch_full_text(pmcid).await.map_err(|e| {
            let timeout_seconds = client.get_pmc_config().timeout.as_secs();
            BatchItemError::new(
                pmcid,
                classify_fetch_error(&e.to_string(), timeout_seconds),
                format!("Failed to fetch article: {:#}", anyhow::anyhow!(e)),
            )
        })?;

        let mut config = MarkdownConfig {
            metadata: pubmed_client::pmc::MetadataOptions {
                use_yaml_frontmatter: self.frontmatter,
                ..Default::default()
            },
            ..Default::default()
        };

        let figure_paths = if self.with_figures {
            let paths = self.extract_figures(client, pmcid, storage, ctx).await?;
            config.figures.include_local_figures = true;
            Some(paths)
        } else {
            None
        };

        let converter = PmcMarkdownConverter::with_config(config);
        let markdown = converter.convert_with_figures(&article, figure_paths.as_ref());

        let output_file = format!("{}.md", pmcid);
        storage
            .write_file(&output_file, markdown.as_bytes())
            .await
            .map_err(|e| {
                BatchItemError::new(
                    pmcid,
                    FailureKind::StorageError {
                        operation: "write_markdown".to_string(),
                    },
                    format!("{:#}", e),
                )
            })?;

        Ok(())
    }

    /// Download figures into the storage backend and return a map from figure ID
    /// to the markdown-relative path used to reference it.
    async fn extract_figures(
        &self,
        client: &pubmed_client::pmc::PmcClient,
        pmcid: &str,
        storage: &dyn StorageBackend,
        ctx: &ClientContext<'_>,
    ) -> Result<HashMap<String, String>, BatchItemError> {
        let storage_err = |op: &str, e: anyhow::Error| {
            BatchItemError::new(
                pmcid,
                FailureKind::StorageError {
                    operation: op.to_string(),
                },
                format!("{:#}", e),
            )
        };

        let temp_dir = TempDir::new()
            .map_err(|e| BatchItemError::new(pmcid, FailureKind::Other, e.to_string()))?;

        let tar_client = pubmed_client::pmc::PmcTarClient::new(ctx.build_config());
        let extracted_figures = tar_client
            .extract_figures_with_captions(pmcid, temp_dir.path())
            .await
            .map_err(|e| {
                let timeout_seconds = client.get_tar_client_config().timeout.as_secs();
                BatchItemError::new(
                    pmcid,
                    classify_fetch_error(&e.to_string(), timeout_seconds),
                    format!("Failed to extract figures: {:#}", anyhow::anyhow!(e)),
                )
            })?;

        let figures_dir = format!("{}/figures", pmcid);
        storage
            .ensure_directory(&figures_dir)
            .await
            .map_err(|e| storage_err("ensure_directory", e))?;

        let mut figure_paths = HashMap::new();
        for fig in extracted_figures {
            let src = std::path::Path::new(&fig.extracted_file_path);
            let file_name = src
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let storage_path = format!("{}/{}", figures_dir, file_name);

            storage
                .copy_file(src, &storage_path)
                .await
                .map_err(|e| storage_err("copy_figure", e))?;

            let relative_path = format!("./{}/figures/{}", pmcid, file_name);
            figure_paths.insert(fig.figure.id.clone(), relative_path);
        }

        Ok(figure_paths)
    }
}
