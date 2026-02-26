use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent")
        .to_path_buf()
}

fn load_pubmed_xml(pmid: &str) -> String {
    let path = workspace_root()
        .join("test_data/pubmed_xml")
        .join(format!("{pmid}.xml"));
    fs::read_to_string(path).unwrap()
}

fn load_all_pubmed_xmls() -> Vec<(String, String)> {
    let xml_dir = workspace_root().join("test_data/pubmed_xml");
    let mut files: Vec<(String, String)> = fs::read_dir(&xml_dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()?.to_str()? == "xml" {
                let pmid = path.file_stem()?.to_str()?.to_string();
                let content = fs::read_to_string(&path).ok()?;
                Some((pmid, content))
            } else {
                None
            }
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn build_batch_xml(files: &[(String, String)]) -> String {
    let mut combined = String::from("<?xml version=\"1.0\" ?>\n<PubmedArticleSet>\n");
    for (_, content) in files {
        if let Some(start) = content.find("<PubmedArticle>") {
            if let Some(end) = content.rfind("</PubmedArticle>") {
                combined.push_str(&content[start..end + "</PubmedArticle>".len()]);
                combined.push('\n');
            }
        }
    }
    combined.push_str("</PubmedArticleSet>");
    combined
}

fn bench_single_parse(c: &mut Criterion) {
    let test_cases = vec![
        ("27350240", "small_5KB"),
        ("34567890", "medium_11KB"),
        ("31978945", "large_30KB"),
        ("32887691", "xlarge_35KB"),
    ];

    let mut group = c.benchmark_group("pubmed_single_parse");
    for (pmid, label) in &test_cases {
        let xml = load_pubmed_xml(pmid);
        let size = xml.len() as u64;
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::new("parse_article", label), &xml, |b, xml| {
            b.iter(|| {
                pubmed_parser::pubmed::parser::parse_article_from_xml(
                    black_box(xml),
                    black_box(pmid),
                )
            })
        });
    }
    group.finish();
}

fn bench_batch_parse(c: &mut Criterion) {
    let files = load_all_pubmed_xmls();
    let batch_xml = build_batch_xml(&files);
    let size = batch_xml.len() as u64;

    let mut group = c.benchmark_group("pubmed_batch_parse");
    group.throughput(Throughput::Bytes(size));
    group.bench_function(
        BenchmarkId::new("parse_articles_from_xml", format!("{}_files", files.len())),
        |b| {
            b.iter(|| pubmed_parser::pubmed::parser::parse_articles_from_xml(black_box(&batch_xml)))
        },
    );
    group.finish();
}

fn bench_sequential_parse(c: &mut Criterion) {
    let files = load_all_pubmed_xmls();
    let total_size: u64 = files.iter().map(|(_, xml)| xml.len() as u64).sum();

    let mut group = c.benchmark_group("pubmed_sequential_parse");
    group.throughput(Throughput::Bytes(total_size));
    group.bench_function(
        BenchmarkId::new("parse_all_sequential", format!("{}_files", files.len())),
        |b| {
            b.iter(|| {
                for (pmid, xml) in &files {
                    let _ = pubmed_parser::pubmed::parser::parse_article_from_xml(
                        black_box(xml),
                        black_box(pmid),
                    );
                }
            })
        },
    );
    group.finish();
}

criterion_group!(
    benches,
    bench_single_parse,
    bench_batch_parse,
    bench_sequential_parse
);
criterion_main!(benches);
