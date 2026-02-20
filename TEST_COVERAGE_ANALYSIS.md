# Test Coverage Analysis

*Generated: 2026-02-20*

## Overall Numbers

- **Total tests**: 578 (563 passing, 15 failing due to Git LFS fixtures not pulled, 2 skipped)
- **Core library line coverage**: **82.72%** (8,719 lines total, 1,507 uncovered)
- **Core library function coverage**: **86.10%** (1,050 functions, 146 uncovered)

## Coverage by Module (line coverage)

| Module | Lines | Coverage | Notes |
|--------|-------|----------|-------|
| `rate_limit.rs` | 73 | **100%** | Excellent |
| `time.rs` | 81 | **100%** | Excellent |
| `pubmed/query/*` | ~1,200 | **97-100%** | Excellent — best-tested module |
| `cache.rs` | 73 | **100%** | Excellent |
| `common/ids.rs` | 265 | **97.7%** | Good |
| `pubmed/parser/mod.rs` | 564 | **97.5%** | Good |
| `pubmed/models.rs` | 492 | **96.1%** | Good |
| `pubmed/parser/converters.rs` | 277 | **93.9%** | Good, but **no unit tests** — covered only via integration tests |
| `error.rs` | 225 | **92.9%** | Good |
| `pubmed/parser/xml_types.rs` | 45 | **91.1%** | Good, but **no unit tests** |
| `common/models.rs` | 121 | **90.1%** | Good |
| `retry.rs` | 196 | **88.8%** | Acceptable |
| `common/xml_utils.rs` | 238 | **87.8%** | Acceptable |
| `config.rs` | 166 | **86.1%** | Acceptable |
| `pmc/models.rs` | 402 | **85.3%** | Acceptable |
| `pmc/oa_api.rs` | 83 | **83.1%** | Acceptable |
| `pubmed/client.rs` | 1,065 | **81.8%** | Moderate — largest file, many HTTP-dependent paths |
| `pubmed/parser/deserializers.rs` | 47 | **72.3%** | Needs improvement |
| `pmc/parser/metadata.rs` | 373 | **71.1%** | Needs improvement |
| `pmc/parser/author.rs` | 264 | **70.5%** | Needs improvement |
| `pmc/parser/section.rs` | 357 | **69.8%** | Needs improvement |
| `pmc/parser/reference.rs` | 195 | **65.6%** | Needs improvement |
| `pmc/markdown.rs` | 719 | **61.9%** | Needs improvement — 1,147 lines total |
| `pmc/client.rs` | 248 | **53.2%** | Poor — many untested HTTP paths |
| `pmc/tar.rs` | 418 | **43.5%** | Poor — 693 lines, barely half tested |
| `lib.rs` | 84 | **35.7%** | Poor — `Client` wrapper has no unit tests |

## Source Files With No Unit Tests

These files have no `#[cfg(test)]` module — whatever coverage they have comes only from integration tests:

1. **`lib.rs`** (757 lines) — Unified `Client` struct with 12 public methods
2. **`pubmed/parser/converters.rs`** (352 lines) — Type conversion helpers
3. **`pubmed/parser/xml_types.rs`** (392 lines) — XML element type definitions
4. **`pubmed/responses.rs`** (189 lines) — Internal API response deserialization types

## Packages With Zero or Minimal Test Coverage

| Package | Source Lines | Tests |
|---------|-------------|-------|
| `pubmed-mcp` | 832 | Zero tests |
| `pubmed-cli` | 2,769 | Only `search.rs` and `storage.rs` have unit tests |
| `pubmed-client-napi` | — | 2 test files |
| `pubmed-client-wasm` | — | 1 test file |

## Existing Test Failures (15 tests)

All 15 failures are caused by Git LFS pointers not being resolved — XML test fixtures contain 130-byte pointer files instead of actual XML. Affected suites:

- `comprehensive_pubmed_tests` (3 tests)
- `comprehensive_pmc_tests` (1 test)
- `comprehensive_elink_tests` (3 tests)
- `comprehensive_einfo_tests` (1 test)
- `markdown_tests` (2 tests)
- `pmc_xml_tests` (1 test)
- `test_figure_extraction` (4 tests)

## Recommended Improvements (Prioritized)

### Priority 1 — High-Impact, Low-Coverage Areas

#### 1. `pmc/tar.rs` (43.5% → target 80%+)

Lowest-coverage source file with existing tests. The tar extraction logic has many untested paths.

- Unit tests for `extract_tar_contents` with synthetic tar archives (using `tar` crate)
- Tests for error cases: corrupt archives, missing files, invalid image formats
- Tests for `match_figures_to_files` with various file naming patterns
- Mock-based tests for `download_tar` to cover HTTP error paths

#### 2. `pmc/client.rs` (53.2% → target 80%+)

Many HTTP-dependent code paths untested. Add wiremock-based tests for:

- `fetch_full_text` success and error paths
- `check_pmc_availability` with various ELink response shapes
- `extract_figures` and `extract_figures_with_captions`
- `download_and_extract_tar` with mocked HTTP responses
- Error handling: network timeouts, XML parse failures, non-200 status codes

#### 3. `pmc/markdown.rs` (61.9% → target 85%+)

1,147 lines but only 62% covered. Likely uncovered areas:

- Edge cases in section rendering (deeply nested, empty sections)
- Table-of-contents generation
- Various `ReferenceStyle` and `HeadingStyle` combinations
- Special character escaping
- Figures and tables rendering within sections
- `MarkdownConfig` builder options

#### 4. `lib.rs` unified `Client` (35.7% → target 75%+)

The `Client` struct wraps PubMed and PMC clients but has no unit tests. The orchestration logic in `search_with_full_text` should have mock-based tests to verify coordination and error propagation.

### Priority 2 — PMC Parser Modules

#### 5. `pmc/parser/reference.rs` (65.6%)

- References with missing fields (no DOI, no volume, etc.)
- Mixed reference types (journal article, book, conference paper)
- Malformed reference XML

#### 6. `pmc/parser/section.rs` (69.8%)

- Nested subsections (3+ levels deep)
- Sections with inline formulas, code blocks, or special elements
- Empty sections and sections with only figures/tables

#### 7. `pmc/parser/author.rs` (70.5%)

- Authors with multiple affiliations
- Consortium/group authors
- Authors with ORCID identifiers
- Non-Latin name handling

#### 8. `pmc/parser/metadata.rs` (71.1%)

- Articles with missing metadata fields
- Various `article-type` values
- License/copyright extraction edge cases
- Funding information parsing

### Priority 3 — Zero-Test Packages

#### 9. `pubmed-mcp` (0 tests, 832 lines)

Complete blind spot. Add:

- Unit tests for each tool's parameter validation and response formatting
- Tests for search tool filter construction (`search.rs`, 268 lines)
- Tests for citation matching input parsing (`citmatch.rs`, 113 lines)
- Integration test with mocked `pubmed-client` for end-to-end tool execution

#### 10. `pubmed-cli` (most commands untested, 2,769 lines)

Only `search.rs` and `storage.rs` have tests. Commands needing coverage:

- `figures.rs` (523 lines) — figure extraction and display logic
- `metadata.rs` (366 lines) — metadata formatting and output
- `convert.rs` (216 lines) — format conversion logic
- `markdown.rs` (134 lines) — markdown output formatting
- `citmatch.rs` (125 lines) — citation matching CLI
- `gquery.rs` (127 lines) — global query CLI

### Priority 4 — Missing Unit Tests for Integration-Covered Files

#### 11. `pubmed/parser/converters.rs` (93.9% via integration tests only)

Adding direct unit tests would make failures easier to diagnose and provide faster feedback.

#### 12. `pubmed/parser/deserializers.rs` (72.3%)

Custom serde deserializers need targeted unit tests for edge cases: empty strings, unexpected types, null values.

#### 13. `pubmed/responses.rs` (no tests, 189 lines)

Internal API response types should have deserialization round-trip tests using sample JSON/XML snippets.

### Priority 5 — Test Infrastructure

#### 14. Fix Git LFS test data handling

15 tests fail because LFS-managed XML fixtures are not resolved. Either:

- Document the `git lfs pull` requirement in CI and `CLAUDE.md`
- Add runtime checks that skip tests when fixtures are unavailable
- Consider embedding small XML test snippets directly in the test code for critical paths

#### 15. Binding package test expansion

- `pubmed-client-napi`: Add tests for PMC client bindings, markdown conversion, error handling
- `pubmed-client-wasm`: Add tests for search, fetch, and configuration
- `pubmed-client-py`: Add mocked equivalents for integration tests that currently require network
