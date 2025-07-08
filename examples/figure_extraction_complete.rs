use clap::Parser;
use pubmed_client_rs::PmcClient;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Parser, Debug)]
#[clap(
    name = "pmc-figure-extractor",
    about = "Extract figures with captions from PMC articles",
    long_about = "Downloads PMC articles as TAR files, extracts figures, matches them with captions from XML, and outputs both images and JSON metadata"
)]
struct Args {
    /// PMC ID(s) to process (e.g., PMC7906746 or 7906746)
    #[clap(required = true, min_values = 1)]
    pmcids: Vec<String>,

    /// Output directory for extracted figures
    #[clap(short, long, default_value = "./extracted_figures")]
    output_dir: PathBuf,
}

/// Complete example that extracts figures with captions and outputs both images and JSON metadata
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command line arguments
    let args = Args::parse();

    println!("🔬 PMC Figure Extraction Tool");
    println!("=============================");

    // Initialize the PMC client
    let client = PmcClient::new();

    // Process each PMCID
    for pmcid in &args.pmcids {
        println!("\n📄 Processing article: {}", pmcid);

        if let Err(e) = process_article(&client, pmcid, &args.output_dir).await {
            eprintln!("❌ Error processing {}: {}", pmcid, e);
            continue;
        }

        println!("✅ Successfully processed {}", pmcid);
    }

    Ok(())
}

async fn process_article(
    client: &PmcClient,
    pmcid: &str,
    output_base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create output directory for this article
    let article_dir = output_base.join(pmcid);

    println!("📂 Creating output directory: {}", article_dir.display());
    fs::create_dir_all(&article_dir).await?;

    // Extract figures with captions
    println!("🧬 Extracting figures and matching with captions...");
    let figures = match client
        .extract_figures_with_captions(pmcid, &article_dir)
        .await
    {
        Ok(figures) => figures,
        Err(e) => {
            eprintln!("⚠️  Could not extract figures for {}: {}", pmcid, e);
            return Ok(()); // Continue with other articles
        }
    };

    if figures.is_empty() {
        println!("📭 No figures found in {}", pmcid);
        return Ok(());
    }

    println!("🖼️  Found {} figures", figures.len());

    // Process each figure
    let mut figure_metadata = Vec::new();

    for (index, extracted_figure) in figures.iter().enumerate() {
        let figure_num = index + 1;
        println!(
            "\n  📸 Processing Figure {} (ID: {})",
            figure_num, extracted_figure.figure.id
        );

        // Create metadata for this figure
        let metadata = FigureMetadata {
            pmcid: pmcid.to_string(),
            figure_id: extracted_figure.figure.id.clone(),
            label: extracted_figure.figure.label.clone(),
            caption: extracted_figure.figure.caption.clone(),
            alt_text: extracted_figure.figure.alt_text.clone(),
            fig_type: extracted_figure.figure.fig_type.clone(),
            original_file_path: extracted_figure.extracted_file_path.clone(),
            file_size_bytes: extracted_figure.file_size,
            dimensions: extracted_figure.dimensions,
            extracted_at: chrono::Utc::now().to_rfc3339(),
        };

        // Copy figure to a standardized filename
        let original_path = Path::new(&extracted_figure.extracted_file_path);
        if let (Some(extension), Some(_filename)) =
            (original_path.extension(), original_path.file_stem())
        {
            let new_filename = format!(
                "{}_{}.{}",
                pmcid,
                extracted_figure.figure.id,
                extension.to_string_lossy()
            );
            let new_path = article_dir.join(&new_filename);

            if let Err(e) = fs::copy(&extracted_figure.extracted_file_path, &new_path).await {
                eprintln!("    ⚠️  Could not copy figure: {}", e);
            } else {
                println!("    💾 Saved as: {}", new_filename);
                println!(
                    "    📝 Caption: {}",
                    if extracted_figure.figure.caption.len() > 80 {
                        format!("{}...", &extracted_figure.figure.caption[..80])
                    } else {
                        extracted_figure.figure.caption.clone()
                    }
                );

                if let Some(dimensions) = extracted_figure.dimensions {
                    println!("    📐 Dimensions: {}x{}", dimensions.0, dimensions.1);
                }

                if let Some(size) = extracted_figure.file_size {
                    println!("    📊 Size: {} KB", size / 1024);
                }
            }
        }

        figure_metadata.push(metadata);
    }

    // Save metadata as JSON
    let json_filename = format!("{}_figures_metadata.json", pmcid);
    let json_path = article_dir.join(&json_filename);

    let json_content = serde_json::to_string_pretty(&figure_metadata)?;
    fs::write(&json_path, json_content).await?;

    println!("\n📋 Saved metadata to: {}", json_filename);

    // Create a summary report
    create_summary_report(&article_dir, pmcid, &figure_metadata).await?;

    Ok(())
}

async fn create_summary_report(
    output_dir: &Path,
    pmcid: &str,
    figures: &[FigureMetadata],
) -> Result<(), Box<dyn std::error::Error>> {
    let report_path = output_dir.join(format!("{}_summary.txt", pmcid));

    let mut report = String::new();
    report.push_str(&format!("Figure Extraction Summary for {}\n", pmcid));
    report.push_str(&"=".repeat(50));
    report.push_str("\n\n");

    report.push_str(&format!("📊 Total figures extracted: {}\n", figures.len()));
    report.push_str(&format!(
        "📅 Extraction date: {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    for (i, figure) in figures.iter().enumerate() {
        report.push_str(&format!("{}. Figure: {}\n", i + 1, figure.figure_id));
        if let Some(label) = &figure.label {
            report.push_str(&format!("   Label: {}\n", label));
        }
        report.push_str(&format!(
            "   Caption: {}\n",
            if figure.caption.len() > 100 {
                format!("{}...", &figure.caption[..100])
            } else {
                figure.caption.clone()
            }
        ));

        if let Some(dimensions) = figure.dimensions {
            report.push_str(&format!(
                "   Dimensions: {}x{}\n",
                dimensions.0, dimensions.1
            ));
        }

        if let Some(size) = figure.file_size_bytes {
            report.push_str(&format!("   Size: {} KB\n", size / 1024));
        }

        report.push('\n');
    }

    fs::write(&report_path, report).await?;
    println!("📄 Created summary report: {}_summary.txt", pmcid);

    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FigureMetadata {
    pmcid: String,
    figure_id: String,
    label: Option<String>,
    caption: String,
    alt_text: Option<String>,
    fig_type: Option<String>,
    original_file_path: String,
    file_size_bytes: Option<u64>,
    dimensions: Option<(u32, u32)>,
    extracted_at: String,
}
