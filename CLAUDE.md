# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

Rust workspace for PubMed and PMC (PubMed Central) API clients with bindings for multiple languages. Provides article search, full-text retrieval, markdown conversion, figure extraction, and citation analysis.

**PyPI package name**: `pubmed-client-py`

## Workspace Structure

```
pubmed-client-rs/                    # Cargo workspace root
├── pubmed-parser/                   # XML parsing & data models (crate: pubmed-parser)
├── pubmed-formatter/                # Citation export & markdown conversion (crate: pubmed-formatter)
├── pubmed-client/                   # HTTP client & API integration (crate: pubmed-client)
├── pubmed-client-napi/              # Native Node.js bindings via napi-rs (npm: pubmed-client)
├── pubmed-client-wasm/              # WASM bindings for browsers/Node.js (npm: pubmed-client-wasm)
├── pubmed-client-py/                # Python bindings via PyO3 (PyPI: pubmed-client-py)
├── pubmed-cli/                      # Command-line interface
├── pubmed-mcp/                      # MCP server for AI assistant integration
└── website/                         # Docusaurus v3 landing page (GitHub Pages)
```

**Crate dependency graph**: `pubmed-parser` ← `pubmed-formatter` ← `pubmed-client` ← bindings/cli/mcp

## Commands

### Build & Test

```bash
# Workspace-wide
cargo build                          # Build all
cargo test                           # Test all (or: mise r test)
cargo nextest run --workspace        # Test with nextest (preferred)
cargo check                          # Check all

# Parser crate
cargo test -p pubmed-parser

# Formatter crate
cargo test -p pubmed-formatter

# Client crate
cargo test -p pubmed-client
cargo test --test comprehensive_pmc_tests -p pubmed-client
cargo test --test comprehensive_pubmed_tests -p pubmed-client

# Real API tests (opt-in, requires network)
cd pubmed-client && PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pubmed_api_tests

# Single unit test
cargo test --lib -p pubmed-parser pubmed::parser::tests::test_mesh_term_parsing

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

mise tasks require `MISE_ENV` to load per-area configs. See `DEVELOPMENT.md` for full details.

```bash
# Rust (workspace-wide, requires MISE_ENV=rust)
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

The codebase is split into three core Rust crates with a clear layering:

1. **`pubmed-parser`** — Pure parsing library (no network). XML parsing and data models.
2. **`pubmed-formatter`** — Citation export and markdown conversion. Depends on `pubmed-parser`.
3. **`pubmed-client`** — HTTP client, caching, rate limiting. Depends on both above. Re-exports their types.

### Parser (`pubmed-parser/src/`)

```
lib.rs                 # Re-exports: common, error, pmc, pubmed modules
error.rs               # ParseError enum and Result type alias

common/                # Shared types between PubMed and PMC
  ids.rs               # PmcId, PubMedId type-safe identifiers
  models.rs            # Shared Author, Affiliation types
  xml_utils.rs         # XML parsing helpers

pubmed/                # PubMed XML parsing
  models.rs            # PubMedArticle, SearchResult, Citations, MeshHeading, etc.
  parser/              # XML parsing for PubMed article metadata
    mod.rs             # parse_article_from_xml, main entry
    batch.rs           # parse_articles_from_xml (batch)
    converters.rs      # Type conversion helpers
    deserializers.rs   # Custom serde deserializers
    extractors.rs      # Field extraction from XML elements
    preprocessing.rs   # XML preprocessing
    xml_types.rs       # XML element type definitions

pmc/                   # PMC XML parsing
  models.rs            # PmcFullText, ArticleSection, Figure, Table, Reference, etc.
  oa_api.rs            # PMC Open Access API types
  parser/              # XML parsing for PMC full-text
    mod.rs             # parse_pmc_xml, main entry
    author.rs          # Author extraction
    metadata.rs        # Metadata extraction
    reference.rs       # Reference extraction
    section.rs         # Section parsing
    xml_utils.rs       # XML utilities
```

### Formatter (`pubmed-formatter/src/`)

```
lib.rs                 # Re-exports: ExportFormat, PmcMarkdownConverter, etc.

pubmed/
  export.rs            # ExportFormat trait: to_bibtex(), to_ris(), to_csl_json(), to_nbib()
                       # Batch helpers: articles_to_bibtex(), articles_to_ris(), articles_to_csl_json()

pmc/
  markdown.rs          # PmcMarkdownConverter (builder pattern)
                       # MarkdownConfig, HeadingStyle, ReferenceStyle
                       # Supports: TOC, YAML frontmatter, figure paths, ORCID links
```

### Client (`pubmed-client/src/`)

```
lib.rs                 # Entry point, unified Client struct, re-exports from parser/formatter
cache.rs               # Response caching (pluggable: memory/Redis/SQLite)
config.rs              # ClientConfig (API keys, rate limiting, caching, timeouts)
error.rs               # PubMedError enum (wraps ParseError from pubmed-parser)
rate_limit.rs          # Token bucket rate limiter for NCBI API compliance
retry.rs               # Retry with exponential backoff
time.rs                # Cross-platform time utilities (native + WASM)

pubmed/                # PubMed E-utilities API
  client/              # PubMedClient (split into focused modules)
    mod.rs             # Core client, search, EFetch
    summary.rs         # ESummary API (lightweight metadata)
    history.rs         # EPost & History server operations
    einfo.rs           # Database information (EInfo API)
    elink.rs           # Cross-database linking (ELink API)
    citmatch.rs        # Citation matching (ECitMatch API)
    egquery.rs         # Global database queries (EGQuery API)
    espell.rs          # Spell checking (ESpell API)
  responses.rs         # Internal API response deserialization types
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
  tar.rs               # PmcTarClient for tar archive extraction
```

### Key Types

- `Client` — Unified client with `pubmed` and `pmc` fields; convenience methods: `search_with_full_text`, `fetch_articles`, `fetch_summaries`, `search_and_fetch_summaries`, `get_related_articles`, `get_pmc_links`, `get_citations`, `match_citations`, `global_query`, `get_database_list`, `get_database_info`, `epost`, `fetch_all_by_pmids`, `spell_check`
- `PubMedClient` — Search, fetch metadata, ESummary, EPost/History, ELink, EInfo, ECitMatch, EGQuery, ESpell
- `PmcClient` — Fetch full-text, check availability, extract figures, download tar archives
- `SearchQuery` — Builder pattern for complex queries with filters, date ranges, boolean logic
- `PubMedArticle` — Article metadata (title, authors, abstract, MeSH, keywords, etc.) — defined in `pubmed-parser`
- `PmcFullText` — Structured full-text (sections, references, figures, tables) — defined in `pubmed-parser`
- `PmcMarkdownConverter` — Configurable markdown output with YAML frontmatter — defined in `pubmed-formatter`
- `ExportFormat` — Trait for BibTeX/RIS/CSL-JSON/NBIB export — defined in `pubmed-formatter`
- `ClientConfig` — API key, email, tool name, rate limit, cache (memory/Redis/SQLite), timeout, retry config

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
    fulltext.rs        # Full-text retrieval tool
    figures.rs         # Figure extraction tool
    summary.rs         # fetch_summaries tool (ESummary API)
    export.rs          # Citation export tool (BibTeX/RIS/CSL-JSON/NBIB)
    citmatch.rs        # Citation matching tool
    einfo.rs           # Database information tool (EInfo API)
    elink.rs           # Cross-database linking tool (ELink API)
    gquery.rs          # Global query tool
    espell.rs          # spell_check tool (ESpell API)
    convert.rs         # Converter/adapter utilities
```

### Integration Tests

XML fixtures are in `test_data/` at the workspace root (pmc_xml/ and pubmed_xml/).

- **`pubmed-parser`** tests: Parsing PubMed XML, PMC XML, supplementary materials
- **`pubmed-formatter`** tests: Markdown conversion, BibTeX/RIS/CSL-JSON/NBIB export, YAML frontmatter
- **`pubmed-client`** tests: `comprehensive_pmc_tests`, `comprehensive_pubmed_tests`, `comprehensive_elink_tests`, `comprehensive_einfo_tests`, `test_figure_extraction`, `test_tar_extraction`, `test_pmc_cache`, `test_webenv`, `test_batch_fetch_mocked`

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

**pubmed-parser**: `quick-xml`, `serde`, `serde_json`, `regex`, `thiserror`, `tracing`, `urlencoding`.

**pubmed-formatter**: `pubmed-parser`, `serde`, `serde_json`, `serde_yaml`, `regex`, `tracing`.

**pubmed-client**: `pubmed-parser`, `pubmed-formatter`, `tokio`, `reqwest`, `serde`, `moka` (caching), `rand`, `tar`, `flate2`, `image`, `futures-util`.

Optional (pubmed-client): `redis` (feature: `cache-redis`), `rusqlite` (feature: `cache-sqlite`).

Dev: `rstest`, `tracing-test`, `wiremock`, `tempfile`.

### Design Patterns

- **Layered crate architecture**: parser (pure) → formatter (pure) → client (async/HTTP)
- Async/await with tokio runtime (client crate only)
- Builder pattern for `SearchQuery`, `ClientConfig`, and `PmcMarkdownConverter`
- Module functions for parsers (not structs with static methods)
- Separation of PubMed (metadata) and PMC (full-text) concerns
- Internal response types separate from public API types
- `pubmed-client` re-exports all types from `pubmed-parser` and `pubmed-formatter`
- `tracing` for structured logging
- Token bucket rate limiting for NCBI compliance
- Response caching with moka (configurable TTL and capacity)
- Cross-platform time abstraction for native and WASM targets
