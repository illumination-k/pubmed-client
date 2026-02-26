use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has no parent")
        .to_path_buf()
}

fn load_pmc_xml(pmcid: &str) -> String {
    let path = workspace_root()
        .join("test_data/pmc_xml")
        .join(format!("{pmcid}.xml"));
    fs::read_to_string(path).unwrap()
}

fn load_all_pmc_xmls() -> Vec<(String, String)> {
    let xml_dir = workspace_root().join("test_data/pmc_xml");
    let mut files: Vec<(String, String)> = fs::read_dir(&xml_dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()?.to_str()? == "xml" {
                let pmcid = path.file_stem()?.to_str()?.to_string();
                let content = fs::read_to_string(&path).ok()?;
                Some((pmcid, content))
            } else {
                None
            }
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn bench_single_parse(c: &mut Criterion) {
    let test_cases = vec![
        ("PMC10000000", "tiny_4KB"),
        ("PMC5000000", "small_8KB"),
        ("PMC7906746", "medium_41KB"),
        ("PMC9000000", "large_166KB"),
        ("PMC10821037", "xlarge_264KB"),
    ];

    let mut group = c.benchmark_group("pmc_single_parse");
    for (pmcid, label) in &test_cases {
        let xml = load_pmc_xml(pmcid);
        let size = xml.len() as u64;
        group.throughput(Throughput::Bytes(size));
        group.bench_with_input(BenchmarkId::new("parse_pmc_xml", label), &xml, |b, xml| {
            b.iter(|| pubmed_parser::pmc::parser::parse_pmc_xml(black_box(xml), black_box(pmcid)))
        });
    }
    group.finish();
}

fn bench_sequential_parse(c: &mut Criterion) {
    let files = load_all_pmc_xmls();
    let total_size: u64 = files.iter().map(|(_, xml)| xml.len() as u64).sum();

    let mut group = c.benchmark_group("pmc_sequential_parse");
    group.throughput(Throughput::Bytes(total_size));
    group.sample_size(50);
    group.bench_function(
        BenchmarkId::new("parse_all_sequential", format!("{}_files", files.len())),
        |b| {
            b.iter(|| {
                for (pmcid, xml) in &files {
                    let _ =
                        pubmed_parser::pmc::parser::parse_pmc_xml(black_box(xml), black_box(pmcid));
                }
            })
        },
    );
    group.finish();
}

criterion_group!(benches, bench_single_parse, bench_sequential_parse);
criterion_main!(benches);
