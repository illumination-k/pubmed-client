Add a test to extract figures for the PMC ID: $ARGUMENT, run the test, and if it fails, use tracing information to modify the parser.
Continue it successfully until it works.

Steps to follow:

1. Add a new test case in the `pubmed-client/tests/integration/test_figure_extraction.rs` file for the specified PMC ID by using the existing macro `test_pmcid_figure_extraction!`. (e.g., `test_pmcid_figure_extraction!("PMC$ARGUMENT");`)
2. run the test using `RUST_LOG=info cargo test --test test_figure_extraction est_figure_extraction_pmc$ARGUMENT -- --nocapture`.
3. fix files in @pubmed-client/src/pmc/parser; Maybe you need to fix @pubmed-client/src/pmc/parser/section.rs;
4. If failed, you can check the raw xml file in `pubmed-client/tests/integration/test_data/pmc_xml/PMC$ARGUMENT.xml`. You DO NOT need to download it again.
