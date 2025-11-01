use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use pubmed_client::pmc::{MarkdownConfig, PmcMarkdownConverter};

use super::create_pmc_client;

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
}

impl Markdown {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        let client = create_pmc_client(api_key, email, tool)?;

        // Create output directory if it doesn't exist
        tokio::fs::create_dir_all(&self.output_dir).await?;

        for pmcid in &self.pmcids {
            tracing::info!(pmcid = %pmcid, "Processing article");

            match self
                .process_article(&client, pmcid, api_key, email, tool)
                .await
            {
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
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        // Fetch the article
        let article = client.fetch_full_text(pmcid).await?;

        // Prepare markdown config
        let mut config = MarkdownConfig::default();

        // Handle figures if requested
        let figure_paths = if self.with_figures {
            // Create article-specific directory
            let article_dir = self.output_dir.join(pmcid);
            let figures_dir = article_dir.join("figures");
            tokio::fs::create_dir_all(&figures_dir).await?;

            // Extract figures using tar client with same config
            let mut tar_config = pubmed_client::ClientConfig::new().with_tool(tool);
            if let Some(key) = api_key {
                tar_config = tar_config.with_api_key(key);
            }
            if let Some(email_addr) = email {
                tar_config = tar_config.with_email(email_addr);
            }
            let tar_client = pubmed_client::pmc::PmcTarClient::new(tar_config);
            let extracted_figures = tar_client
                .extract_figures_with_captions(pmcid, &figures_dir)
                .await?;

            // Save figures and collect paths
            let mut figure_paths = std::collections::HashMap::new();
            for fig in extracted_figures {
                let file_path = &fig.extracted_file_path;

                // The extracted path is absolute, we need to make it relative to the output directory
                // Extract creates: output_dir/PMCID/figures/PMCID/filename.jpg
                // We want in markdown: ./PMCID/figures/PMCID/filename.jpg
                let path = std::path::Path::new(file_path);

                // Find the relative path from output_dir
                if let Ok(relative_from_output) = path.strip_prefix(&self.output_dir) {
                    let relative_path = format!("./{}", relative_from_output.to_string_lossy());
                    figure_paths.insert(fig.figure.id.clone(), relative_path);
                } else {
                    // Fallback: try to construct the path manually
                    let file_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let relative_path = format!("./{}/figures/{}/{}", pmcid, pmcid, file_name);
                    figure_paths.insert(fig.figure.id.clone(), relative_path);
                }
            }

            config.include_local_figures = true;
            Some(figure_paths)
        } else {
            None
        };

        // Convert to markdown
        let converter = PmcMarkdownConverter::with_config(config);
        let markdown = converter.convert_with_figures(&article, figure_paths.as_ref());

        // Save markdown file
        let output_file = self.output_dir.join(format!("{}.md", pmcid));
        tokio::fs::write(&output_file, markdown).await?;

        Ok(())
    }
}
