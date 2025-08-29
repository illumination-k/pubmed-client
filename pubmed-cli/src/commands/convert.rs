use anyhow::Result;
use clap::Args;
use serde_json;

use super::create_pubmed_client;

#[derive(Args, Debug)]
pub struct Convert {
    /// PMID(s) to convert to PMCID
    #[arg(required = true)]
    pub pmids: Vec<String>,

    /// Output format (json, csv, or txt)
    #[arg(long, default_value = "json")]
    pub format: String,

    /// Batch size for processing PMIDs (to avoid API rate limits)
    #[arg(long, default_value = "100")]
    pub batch_size: usize,
}

impl Convert {
    pub async fn execute_with_config(
        &self,
        api_key: Option<&str>,
        email: Option<&str>,
        tool: &str,
    ) -> Result<()> {
        // Parse PMIDs from strings to u32
        let parsed_pmids: Result<Vec<u32>, _> =
            self.pmids.iter().map(|pmid| pmid.parse::<u32>()).collect();

        let parsed_pmids = match parsed_pmids {
            Ok(pmids) => pmids,
            Err(e) => {
                tracing::error!("Invalid PMID format. PMIDs must be numeric. Error: {}", e);
                std::process::exit(1);
            }
        };

        // Create client with configuration
        let client = create_pubmed_client(api_key, email, tool)?;

        // Process PMIDs in batches to avoid 419 errors
        let pmc_links = self
            .process_pmids_in_batches(&client, &parsed_pmids)
            .await?;

        match self.format.as_str() {
            "json" => {
                self.output_json(&pmc_links)?;
            }
            "csv" => {
                self.output_csv(&pmc_links);
            }
            "txt" => {
                self.output_txt(&pmc_links);
            }
            _ => {
                tracing::error!(
                    "Unsupported format '{}'. Use 'json', 'csv', or 'txt'.",
                    self.format
                );
                std::process::exit(1);
            }
        }

        Ok(())
    }

    async fn process_pmids_in_batches(
        &self,
        client: &pubmed_client_rs::PubMedClient,
        parsed_pmids: &[u32],
    ) -> Result<pubmed_client_rs::pubmed::models::PmcLinks> {
        let mut all_source_pmids = Vec::new();
        let mut all_pmc_ids = Vec::new();

        tracing::info!(
            total_pmids = parsed_pmids.len(),
            batch_size = self.batch_size,
            "Processing PMIDs in batches"
        );

        for (batch_idx, batch) in parsed_pmids.chunks(self.batch_size).enumerate() {
            let batch_result = self
                .process_single_batch(client, batch, batch_idx + 1)
                .await?;
            all_source_pmids.extend(batch_result.source_pmids);
            all_pmc_ids.extend(batch_result.pmc_ids);

            // Add delay between batches to be respectful to the API
            self.add_inter_batch_delay(batch_idx, parsed_pmids.len())
                .await;
        }

        Ok(pubmed_client_rs::pubmed::models::PmcLinks {
            source_pmids: all_source_pmids,
            pmc_ids: all_pmc_ids,
        })
    }

    async fn process_single_batch(
        &self,
        client: &pubmed_client_rs::PubMedClient,
        batch: &[u32],
        batch_number: usize,
    ) -> Result<pubmed_client_rs::pubmed::models::PmcLinks> {
        tracing::info!(
            batch_index = batch_number,
            batch_size = batch.len(),
            "Processing batch"
        );

        match client.get_pmc_links(batch).await {
            Ok(pmc_links) => {
                let pmcids_found = pmc_links.pmc_ids.len();
                tracing::info!(
                    batch_index = batch_number,
                    pmcids_found = pmcids_found,
                    "Batch processed successfully"
                );
                Ok(pmc_links)
            }
            Err(e) => {
                tracing::error!(
                    batch_index = batch_number,
                    error = %e,
                    "Failed to process batch"
                );
                Err(e.into())
            }
        }
    }

    async fn add_inter_batch_delay(&self, batch_idx: usize, total_pmids: usize) {
        let total_batches = total_pmids.div_ceil(self.batch_size);
        if batch_idx + 1 < total_batches {
            tracing::debug!("Adding delay between batches");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    fn output_json(&self, pmc_links: &pubmed_client_rs::pubmed::models::PmcLinks) -> Result<()> {
        // Create a more user-friendly JSON output
        let mut result = serde_json::Map::new();
        let mut conversions = Vec::new();

        // Add all source PMIDs with their conversion status
        for pmid in &pmc_links.source_pmids {
            let mut conversion = serde_json::Map::new();
            conversion.insert(
                "pmid".to_string(),
                serde_json::Value::Number((*pmid).into()),
            );
            conversion.insert("pmcid".to_string(), serde_json::Value::Null);
            conversions.push(serde_json::Value::Object(conversion));
        }

        // If we have PMCIDs, update the conversions
        // Note: This is a simplified approach - the actual PMID->PMCID mapping
        // requires parsing the ELink response more carefully
        if !pmc_links.pmc_ids.is_empty() {
            result.insert(
                "note".to_string(),
                serde_json::Value::String(
                    "PMCIDs found but mapping to specific PMIDs requires detailed ELink parsing"
                        .to_string(),
                ),
            );
            result.insert(
                "available_pmcids".to_string(),
                serde_json::Value::Array(
                    pmc_links
                        .pmc_ids
                        .iter()
                        .map(|id| serde_json::Value::String(id.clone()))
                        .collect(),
                ),
            );
        }

        result.insert(
            "conversions".to_string(),
            serde_json::Value::Array(conversions),
        );

        let json_output = serde_json::to_string_pretty(&result)?;
        println!("{}", json_output);
        Ok(())
    }

    fn output_csv(&self, pmc_links: &pubmed_client_rs::pubmed::models::PmcLinks) {
        println!("PMID,PMCID_Available,PMCIDs_Found");
        for pmid in &pmc_links.source_pmids {
            let has_pmc = if pmc_links.pmc_ids.is_empty() {
                "false"
            } else {
                "true"
            };
            let pmcids_str = if pmc_links.pmc_ids.is_empty() {
                "".to_string()
            } else {
                pmc_links.pmc_ids.join(";")
            };
            println!("{},{},{}", pmid, has_pmc, pmcids_str);
        }
    }

    fn output_txt(&self, pmc_links: &pubmed_client_rs::pubmed::models::PmcLinks) {
        // Text format: only output PMCIDs, one per line
        for pmcid in &pmc_links.pmc_ids {
            println!("{}", pmcid);
        }
    }
}
