//! Profiling harness for parser benchmarking with samply.
//!
//! Usage:
//!   cargo build --profile profiling --example profile_parsing -p pubmed-parser
//!   samply record target/profiling/examples/profile_parsing [pmc|pubmed|all] [iterations]
//!
//! The default mode is "all" which profiles both PMC and PubMed parsing.

use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent")
        .to_path_buf()
}

fn load_xml_files(dir: &Path) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()?.to_str()? == "xml" {
                let name = path.file_stem()?.to_str()?.to_string();
                let content = fs::read_to_string(&path).ok()?;
                Some((name, content))
            } else {
                None
            }
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn profile_pmc(iterations: usize) {
    let root = workspace_root();
    let files = load_xml_files(&root.join("test_data/pmc_xml"));
    eprintln!("PMC: {} files, {} iterations", files.len(), iterations);

    let start = Instant::now();
    for _ in 0..iterations {
        for (pmcid, xml) in &files {
            let _ = black_box(pubmed_parser::pmc::parser::parse_pmc_xml(
                black_box(xml),
                black_box(pmcid),
            ));
        }
    }
    let elapsed = start.elapsed();
    eprintln!(
        "PMC: {elapsed:.2?} total ({:.2?} per iteration)",
        elapsed / iterations as u32
    );
}

fn profile_pubmed(iterations: usize) {
    let root = workspace_root();
    let files = load_xml_files(&root.join("test_data/pubmed_xml"));
    eprintln!("PubMed: {} files, {} iterations", files.len(), iterations);

    let start = Instant::now();
    for _ in 0..iterations {
        for (pmid, xml) in &files {
            let _ = black_box(pubmed_parser::pubmed::parser::parse_article_from_xml(
                black_box(xml),
                black_box(pmid),
            ));
        }
    }
    let elapsed = start.elapsed();
    eprintln!(
        "PubMed single: {elapsed:.2?} total ({:.2?} per iteration)",
        elapsed / iterations as u32
    );

    let batch_xml = build_batch_xml(&files);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = black_box(pubmed_parser::pubmed::parser::parse_articles_from_xml(
            black_box(&batch_xml),
        ));
    }
    let elapsed = start.elapsed();
    eprintln!(
        "PubMed batch: {elapsed:.2?} total ({:.2?} per iteration)",
        elapsed / iterations as u32
    );
}

fn build_batch_xml(files: &[(String, String)]) -> String {
    let mut combined = String::from("<?xml version=\"1.0\" ?>\n<PubmedArticleSet>\n");
    for (_, content) in files {
        if let Some(start) = content.find("<PubmedArticle>")
            && let Some(end) = content.rfind("</PubmedArticle>")
        {
            combined.push_str(&content[start..end + "</PubmedArticle>".len()]);
            combined.push('\n');
        }
    }
    combined.push_str("</PubmedArticleSet>");
    combined
}

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());
    let iterations: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    match mode.as_str() {
        "pmc" => profile_pmc(iterations),
        "pubmed" => profile_pubmed(iterations),
        "all" => {
            profile_pmc(iterations);
            profile_pubmed(iterations);
        }
        other => {
            eprintln!("Unknown mode: {other}");
            eprintln!("Usage: profile_parsing [pmc|pubmed|all] [iterations]");
            std::process::exit(1);
        }
    }
}
