use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use pubmed_client::pmc::{MarkdownConfig, PmcMarkdownConverter};

use super::ClientContext;

#[derive(Args, Debug)]
pub struct Markdown {
    /// PMC IDs to convert to markdown
    #[arg(required = true)]
    pmcids: Vec<String>,

    /// Output directory for markdown files
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,

    /// Download and embed figures in the markdown
    #[arg(long)]
    with_figures: bool,

    /// Use YAML frontmatter for metadata
    #[arg(long)]
    frontmatter: bool,
}

impl Markdown {
    pub async fn execute(&self, ctx: &ClientContext<'_>) -> Result<()> {
        let client = ctx.pmc_client();

        tokio::fs::create_dir_all(&self.output_dir).await?;

        for pmcid in &self.pmcids {
            tracing::info!(pmcid = %pmcid, "Processing article");

            match self.process_article(&client, pmcid, ctx).await {
                Ok(_) => tracing::info!(pmcid = %pmcid, "Successfully converted to markdown"),
                Err(e) => tracing::error!(pmcid = %pmcid, error = %e, "Failed to process article"),
            }
        }

        Ok(())
    }

    async fn process_article(
        &self,
        client: &pubmed_client::pmc::PmcClient,
        pmcid: &str,
        ctx: &ClientContext<'_>,
    ) -> Result<()> {
        let article = client.fetch_full_text(pmcid).await?;

        let mut config = MarkdownConfig {
            metadata: pubmed_client::pmc::MetadataOptions {
                use_yaml_frontmatter: self.frontmatter,
                ..Default::default()
            },
            ..Default::default()
        };

        let figure_paths = if self.with_figures {
            let article_dir = self.output_dir.join(pmcid);
            let figures_dir = article_dir.join("figures");
            tokio::fs::create_dir_all(&figures_dir).await?;

            let tar_client = pubmed_client::pmc::PmcTarClient::new(ctx.build_config());
            let extracted_figures = tar_client
                .extract_figures_with_captions(pmcid, &figures_dir)
                .await?;

            let mut figure_paths = std::collections::HashMap::new();
            for fig in extracted_figures {
                let file_path = &fig.extracted_file_path;
                let path = std::path::Path::new(file_path);

                if let Ok(relative_from_output) = path.strip_prefix(&self.output_dir) {
                    let relative_path = format!("./{}", relative_from_output.to_string_lossy());
                    figure_paths.insert(fig.figure.id.clone(), relative_path);
                } else {
                    let file_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let relative_path = format!("./{}/figures/{}/{}", pmcid, pmcid, file_name);
                    figure_paths.insert(fig.figure.id.clone(), relative_path);
                }
            }

            config.figures.include_local_figures = true;
            Some(figure_paths)
        } else {
            None
        };

        let converter = PmcMarkdownConverter::with_config(config);
        let markdown = converter.convert_with_figures(&article, figure_paths.as_ref());

        let output_file = self.output_dir.join(format!("{}.md", pmcid));
        tokio::fs::write(&output_file, markdown).await?;

        Ok(())
    }
}
