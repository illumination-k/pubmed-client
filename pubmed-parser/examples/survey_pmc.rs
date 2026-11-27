//! Corpus survey harness for the PMC parser.
//!
//! Parses every `*.xml` file in a directory with [`parse_pmc_xml`], reports
//! failures and timing, and writes each parsed article as pretty JSON so the
//! output can be diffed against the source XML or between parser versions.
//!
//! Usage:
//!   cargo run --release --example survey_pmc -p pubmed-parser -- <xml-dir> [<json-out-dir>]
//!
//! One way to build a corpus is to page through ESearch and batch EFetch, then
//! split the `<pmc-articleset>` into one `<article>` per file:
//!
//! ```text
//! esearch.fcgi?db=pmc&term=open+access%5Bfilter%5D+AND+2020%5Bpdat%5D&retmax=40&retmode=json
//! efetch.fcgi   (POST) db=pmc&id=<comma-separated ids>
//! ```
//!
//! [`parse_pmc_xml`]: pubmed_parser::pmc::parse_pmc_xml

// Example harness — unwrap/expect are fine here.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Instant;

use pubmed_parser::pmc::parse_pmc_xml;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(
        args.next()
            .expect("usage: survey_pmc <xml-dir> [<json-out-dir>]"),
    );
    let output = args.next().map(PathBuf::from);

    let mut paths: Vec<PathBuf> = fs::read_dir(&input)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "xml"))
        .collect();
    paths.sort();

    let docs: Vec<(String, String)> = paths
        .iter()
        .map(|p| {
            let stem = p.file_stem().unwrap().to_string_lossy().to_string();
            (stem, fs::read_to_string(p).unwrap())
        })
        .collect();

    let started = Instant::now();
    for (stem, xml) in &docs {
        let _ = black_box(parse_pmc_xml(xml, stem));
    }
    println!("parsed {} files in {:?}", docs.len(), started.elapsed());

    if let Some(output) = &output {
        fs::create_dir_all(output).unwrap();
    }
    let mut failed = 0usize;
    for (stem, xml) in &docs {
        match parse_pmc_xml(xml, stem) {
            Ok(article) => {
                if let Some(output) = &output {
                    let json = serde_json::to_string_pretty(&article).unwrap();
                    fs::write(output.join(format!("{stem}.json")), json).unwrap();
                }
            }
            Err(e) => {
                failed += 1;
                println!("FAIL {stem}: {e}");
            }
        }
    }
    println!("ok {}, failed {failed}", docs.len() - failed);
}
