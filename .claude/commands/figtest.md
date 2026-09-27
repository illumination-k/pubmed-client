Add a test to extract figures for the PMC ID: $ARGUMENT, run the test, and if it fails, use tracing information to modify the parser.
Continue it successfully until it works.

Steps to follow:

1. Add a new test case in the `pubmed-client/tests/integration/mocked/figures.rs` file for the specified PMC ID by using the existing macro `test_pmcid_figure_extraction!`. (e.g., `test_pmcid_figure_extraction!("PMC$ARGUMENT");`)
2. run the test using `RUST_LOG=info cargo test -p pubmed-client --test mocked_figures test_figure_extraction_pmc$ARGUMENT -- --nocapture`.
3. fix files in @pubmed-parser/src/pmc/parser; figure extraction lives in @pubmed-parser/src/pmc/parser/section/figure.rs.
4. If failed, you can check the raw xml file in `test_data/pmc_xml/PMC$ARGUMENT.xml`. You DO NOT need to download it again.
