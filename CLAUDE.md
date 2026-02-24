# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

Rust workspace for PubMed and PMC (PubMed Central) API clients with bindings for multiple languages. Provides article search, full-text retrieval, markdown conversion, figure extraction, and citation analysis.

**PyPI package name**: `pubmed-client-py`

## Workspace Structure

```
pubmed-client/                       # Cargo workspace root
├── pubmed-client/                   # Core Rust library (crate: pubmed-client)
├── pubmed-client-napi/              # Native Node.js bindings via napi-rs (npm: pubmed-client)
├── pubmed-client-wasm/              # WASM bindings for browsers/Node.js (npm: pubmed-client-wasm)
├── pubmed-client-py/                # Python bindings via PyO3 (PyPI: pubmed-client-py)
├── pubmed-cli/                      # Command-line interface
├── pubmed-mcp/                      # MCP server for AI assistant integration
└── website/                         # Docusaurus v3 landing page (GitHub Pages)
```

## Commands

### Build & Test

```bash
# Workspace-wide
cargo build                          # Build all
cargo test                           # Test all (or: mise r test)
cargo nextest run --workspace        # Test with nextest (preferred)
cargo check                          # Check all

# Core library (from pubmed-client/)
cargo test -p pubmed-client
cargo test --test comprehensive_pmc_tests -p pubmed-client
cargo test --test comprehensive_pubmed_tests -p pubmed-client

# Real API tests (opt-in, requires network)
cd pubmed-client && PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pubmed_api_tests

# Single unit test
cargo test --lib -p pubmed-client pubmed::parser::tests::test_mesh_term_parsing

# NAPI (from pubmed-client-napi/)
pnpm run build && pnpm run test
pnpm run docs                        # Generate TypeDoc HTML → docs/

# WASM (from pubmed-client-wasm/)
pnpm run build && pnpm run test

# Python (from pubmed-client-py/)
uv run --with maturin maturin develop
uv run pytest
uv run pytest -m "not integration"   # Unit tests only

# MCP server
cargo test -p pubmed-mcp
cargo build --release -p pubmed-mcp

# CLI
cargo run -p pubmed-cli -- --help
cargo run -p pubmed-cli -- figures PMC7906746
cargo run -p pubmed-cli -- markdown PMC7906746
cargo run -p pubmed-cli -- pmid-to-pmcid 31978945
```

### Code Quality

```bash
# Rust (workspace-wide)
mise r lint                          # dprint + cargo fmt + clippy + actionlint
mise r fmt                           # dprint + cargo fmt + ruff format

# NAPI/WASM TypeScript (from respective directories)
pnpm run check                       # Biome lint + format
pnpm run typecheck

# Python (from pubmed-client-py/)
uv run ruff check .
uv run ruff format .
uv run mypy tests/ --strict
```

### Code Coverage

```bash
mise r coverage                      # HTML report
cargo llvm-cov nextest -p pubmed-client --all-features --html
```

## Architecture

### Core Library (`pubmed-client/src/`)

```
lib.rs                 # Entry point, re-exports, unified Client struct
cache.rs               # Response caching (moka)
config.rs              # ClientConfig (API keys, rate limiting, caching, timeouts)
error.rs               # PubMedError enum and Result type alias
rate_limit.rs          # Token bucket rate limiter for NCBI API compliance
retry.rs               # Retry with exponential backoff
time.rs                # Cross-platform time utilities (native + WASM)

common/                # Shared types between PubMed and PMC
  ids.rs               # PmcId, PubMedId type-safe identifiers
  models.rs            # Shared Author, Affiliation types
  xml_utils.rs         # XML parsing helpers

pubmed/                # PubMed E-utilities API
  client.rs            # PubMedClient - search, fetch, ELink, EInfo, ECitMatch, EGQuery
  models.rs            # PubMedArticle, SearchResult, Citations, RelatedArticles, etc.
  responses.rs         # Internal API response deserialization types
  parser/              # XML parsing for PubMed article metadata
    batch.rs           # Batch article parsing
    converters.rs      # Type conversion helpers
    deserializers.rs   # Custom serde deserializers
    extractors.rs      # Field extraction from XML elements
    preprocessing.rs   # XML preprocessing
    xml_types.rs       # XML element type definitions
  query/               # SearchQuery builder
    builder.rs         # Main SearchQuery builder
    filters.rs         # Field-specific filters (title, author, journal, etc.)
    dates.rs           # Date range filtering
    date.rs            # PubDate type
    boolean.rs         # AND, OR, NOT logic
    advanced.rs        # MeSH terms, article types
    search.rs          # Search execution
    validation.rs      # Query validation

pmc/                   # PMC (PubMed Central) API
  client.rs            # PmcClient - full-text fetch, availability check, figure extraction
  models.rs            # PmcFullText, ArticleSection, Figure, Table, Reference, etc.
  markdown.rs          # PmcMarkdownConverter with configurable output
  oa_api.rs            # PMC Open Access API for tar.gz downloads
  tar.rs               # PmcTarClient for tar archive extraction
  parser/              # XML parsing for PMC full-text
    author.rs          # Author extraction
    metadata.rs        # Metadata extraction
    reference.rs       # Reference extraction
    section.rs         # Section parsing
    xml_utils.rs       # XML utilities
```

### Key Types

- `Client` - Unified client with `pubmed` and `pmc` fields; also has convenience methods (`search_with_full_text`, `fetch_articles`, `get_related_articles`, `get_pmc_links`, `get_citations`, `match_citations`, `global_query`, `get_database_list`, `get_database_info`)
- `PubMedClient` - Search, fetch metadata, ELink, EInfo, ECitMatch, EGQuery
- `PmcClient` - Fetch full-text, check availability, extract figures, download tar archives
- `SearchQuery` - Builder pattern for complex queries with filters, date ranges, boolean logic
- `PubMedArticle` - Article metadata (title, authors, abstract, MeSH, keywords, etc.)
- `PmcFullText` - Structured full-text (sections, references, figures, tables)
- `ClientConfig` - API key, email, tool name, rate limit, cache, timeout, retry config

### NAPI Bindings (`pubmed-client-napi/`)

Native Node.js bindings via napi-rs. Published as `pubmed-client` on npm. Pre-built binaries for Windows/macOS/Linux (x64/ARM64). Key types: `PubMedClient`, `SearchQuery`, `Config`.

- TypeDoc generates HTML docs from `index.d.ts` via `pnpm run docs` (output: `docs/`, gitignored)
- Config: `typedoc.json` + `tsconfig.typedoc.json` (separate tsconfig scoped to `index.d.ts`)
- CI: `node-docs` job in `docs.yml` uploads artifact → merged into `website/build/node/` by `build-site`

### WASM Bindings (`pubmed-client-wasm/`)

WebAssembly bindings via wasm-pack. Published as `pubmed-client-wasm` on npm. Key types: `WasmPubMedClient`, `WasmClientConfig`.

### Python Bindings (`pubmed-client-py/`)

Python bindings via PyO3/maturin. Published as `pubmed-client-py` on PyPI. Synchronous API with internal Tokio runtime. Key types: `Client`, `PubMedClient`, `PmcClient`, `SearchQuery`, `ClientConfig`.

### Website (`website/`)

Docusaurus v3 landing page deployed to GitHub Pages at `https://illumination-k.github.io/pubmed-client/`.

- `baseUrl: '/pubmed-client/'`, `docs: false`, `blog: false` (landing page only)
- Linter/formatter: Biome v2 (`pnpm run check`), TypeScript: `pnpm run typecheck`
- All doc links use full absolute URLs (`https://illumination-k.github.io/pubmed-client/...`) — use `<a href>` not `<Link to>` for external HTML (React Router can't route to non-Docusaurus paths); same rule applies in `docusaurus.config.ts` navbar/footer
- CI: `.github/workflows/docs.yml` — `docs` job (cargo doc) + `node-docs` job (TypeDoc, parallel) → `build-site` job (Docusaurus build + merge both into `build/`) → `deploy-docs` job (GitHub Pages, main only)
- GitHub Pages URL structure: `/` (landing) · `/rust/pubmed_client/` (rustdoc) · `/node/` (TypeDoc) · `/python/` (placeholder, future Sphinx)

```bash
# from website/
pnpm run start        # local dev server
pnpm run build        # production build
pnpm run check        # Biome lint + format
pnpm run typecheck    # tsc
```

### MCP Server (`pubmed-mcp/`)

MCP server for AI assistants (Claude Desktop, etc.) built with rmcp. Communicates via stdio.

```
src/
  main.rs              # Server entry point
  tools/
    mod.rs             # PubMedServer definition
    search.rs          # search_pubmed tool (with study type/text availability filters)
    markdown.rs        # get_pmc_markdown tool
    citmatch.rs        # Citation matching tool
    gquery.rs          # Global query tool
```

### Integration Tests (`pubmed-client/tests/integration/`)

Tests use real XML fixtures in `tests/integration/test_data/` (pmc_xml/ and pubmed_xml/). Key test suites: `comprehensive_pmc_tests`, `comprehensive_pubmed_tests`, `comprehensive_elink_tests`, `comprehensive_einfo_tests`, `markdown_tests`, `test_figure_extraction`, `test_tar_extraction`, `test_pmc_cache`, `test_webenv`, `test_batch_fetch_mocked`.

## Guidelines

### Git Operations

Always use `git mv` for renames (preserves history). Check `git status` before and after operations.

### Logging

Use `tracing` macros (`info!`, `debug!`, `warn!`, `error!`), never `println!`/`eprintln!` in library code. `println!` is only acceptable in doc examples and the CLI.

### PubMed Search Field Tags

Always reference the official NCBI documentation before adding or modifying field tags:

- https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags
- https://www.ncbi.nlm.nih.gov/books/NBK25499/

Validated tags: `[ti]`, `[tiab]`, `[au]`, `[1au]`, `[lastau]`, `[ad]`, `[ta]`, `[la]`, `[pt]`, `[mh]`, `[majr]`, `[sh]`, `[gr]`, `[auid]`, `[pdat]`, `[edat]`, `[mdat]`, `[sb]`.

Invalid tags (do NOT use): `[Organism]`, `[lang]`, long-form tags like `[Title]`, `[Author]`.

### Python Bindings (maturin/PyO3)

**The #1 issue**: UV + maturin package manager conflict. Never mix `maturin develop` with `uv run python` (UV reinstalls from cache, overwriting builds).

Correct workflow (from `pubmed-client-py/`):

```bash
uv run --with maturin --with patchelf maturin build --release
uv pip install ../target/wheels/pubmed_client_py-*.whl --force-reinstall
.venv/bin/python -m pytest tests/
```

See `.claude/skills/maturin-debugger/SKILL.md` for detailed troubleshooting.

### Dependencies

Core: `tokio`, `reqwest`, `serde`, `quick-xml`, `thiserror`, `tracing`, `moka` (caching), `urlencoding`, `rand`, `regex`, `tar`, `flate2`, `image`, `futures-util`.

Dev: `rstest`, `tracing-test`, `wiremock`, `tempfile`.

### Design Patterns

- Async/await with tokio runtime
- Builder pattern for `SearchQuery` and `ClientConfig`
- Module functions for parsers (not structs with static methods)
- Separation of PubMed (metadata) and PMC (full-text) concerns
- Internal response types separate from public API types
- `tracing` for structured logging
- Token bucket rate limiting for NCBI compliance
- Response caching with moka (configurable TTL and capacity)
- Cross-platform time abstraction for native and WASM targets
