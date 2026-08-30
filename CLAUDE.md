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
├── pubmed-client-go/                # Go bindings via cgo (Go module: .../pubmed-client-go)
├── pubmed-client-r/                 # R bindings via extendr (R package: pubmedclient) — NOT a workspace member
├── pubmed-cli/                      # Command-line interface
├── pubmed-mcp/                      # MCP server for AI assistant integration
├── pubmed-test-utils/               # Shared XML fixture loaders for tests (crate: pubmed-test-utils, not published)
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

# Go (requires MISE_ENV=go; run from anywhere in the workspace)
mise run go:build                    # cargo staticlib → lib/$GOOS_$GOARCH/, required before any go command
mise run go:test                     # offline tests (stub HTTP server); depends on go:build
mise run go:test-integration         # live NCBI API, opt-in
cargo test -p pubmed-client-go       # Rust FFI boundary tests

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

# Rustdoc is NOT covered by `mise r lint`, and CI builds it with -D warnings —
# a broken intra-doc link (e.g. to a private item) fails `docs.yml` and the
# `Lint and Format` job while `mise r lint` stays green. Run it before pushing:
RUSTDOCFLAGS="-D warnings --cfg docsrs" PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 \
  cargo doc --all-features --no-deps

# NAPI/WASM TypeScript (from respective directories)
pnpm run check                       # Biome lint + format
pnpm run typecheck

# Python (from pubmed-client-py/)
uv run ruff check .
uv run ruff format .
uv run mypy tests/ --strict

# Go (requires MISE_ENV=go)
mise r lint:go                       # gofmt check + go vet
mise r fmt:go                        # gofmt -w
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
lib.rs                 # Re-exports: common, error, europe_pmc, pmc, pubmed modules
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
  domain.rs            # JATS-DTD-faithful domain models: PmcArticle (front/body/back),
                       # Front, ArticleMeta, Body, Back, Section, Figure, Table, Reference, etc.
                       # Single PMC model layer; flat read access via accessor methods
  oa_api.rs            # PMC Open Access API types
  parser/              # XML parsing for PMC full-text
    mod.rs             # parse_pmc_xml, main entry (returns PmcArticle)
    author.rs          # Author extraction
    metadata.rs        # Metadata extraction
    reference.rs       # Reference extraction
    section.rs         # Section parsing
    reader_utils.rs    # quick-xml reader helpers shared by metadata.rs / section.rs
    xml_utils.rs       # Re-export shim over common/xml_utils.rs

europe_pmc/            # Europe PMC JSON response models & parsers
  models.rs            # EuropePmcResult and shared record fields
  search.rs            # EuropePmcSearchResponse, parse_search_response
  references.rs        # EuropePmcReference(List), parse_references_response
  citations.rs         # EuropePmcCitation(List), parse_citations_response
  links.rs             # EuropePmcDatabaseLink(List), parse_database_links_response
  de.rs                # Custom serde deserializers for Europe PMC quirks
```

### Formatter (`pubmed-formatter/src/`)

```
lib.rs                 # Re-exports: ExportFormat, PmcMarkdownConverter, etc.

pubmed/
  export.rs            # ExportFormat trait: to_bibtex(), to_ris(), to_csl_json(), to_nbib()
                       # Batch helpers: articles_to_bibtex(), articles_to_ris(), articles_to_csl_json()

pmc/
  markdown/            # PmcMarkdownConverter (builder pattern)
    mod.rs             # PmcMarkdownConverter, convert(), convert_with_figures()
    config.rs          # MarkdownConfig, HeadingStyle, ReferenceStyle
    frontmatter.rs     # YAML frontmatter generation
    metadata.rs        # Title / authors / journal / DOI block
    sections.rs        # Body sections, figures, tables
    references.rs      # Reference list rendering
    heading.rs         # Heading style formatting
    toc.rs             # Table of contents
    entities.rs        # XML entity / inline markup cleanup
```

### Client (`pubmed-client/src/`)

```
lib.rs                 # Entry point, unified Client struct, re-exports from parser/formatter
cache.rs               # Response caching (pluggable: memory/Redis/SQLite)
config.rs              # ClientConfig (API keys, rate limiting, caching, timeouts)
error.rs               # PubMedError enum (wraps ParseError from pubmed-parser)
rate_limit.rs          # Token bucket rate limiter for NCBI API compliance
request.rs             # RequestExecutor — URL building + rate limit + retry + error mapping,
                       # shared by every endpoint module in the crate
retry.rs               # Retry with exponential backoff
time.rs                # Cross-platform time utilities (native + WASM)
tls.rs                 # rustls crypto provider installation (rustls-tls feature)

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
  cloud.rs             # PmcCloudClient - per-file download from the PMC OA Cloud (AWS S3)
  common.rs            # Shared PMC helpers (normalize_pmcid, ...)
  extracted.rs         # ExtractedFigure / downloaded-file result types

europe_pmc/            # Europe PMC REST API (EBI; complements NCBI E-utilities)
  client.rs            # EuropePmcClient - construction, cache, executor plumbing
  id.rs                # EuropePmcId / EuropePmcSource — (source, id) addressing
  search.rs            # Cross-source search (cursorMark pagination)
  fulltext.rs          # JATS full text -> PmcArticle, and raw XML
  paged.rs             # PagedList trait + shared page-number pagination
  references.rs        # /references endpoint
  citations.rs         # /citations endpoint
  links.rs             # /databaseLinks endpoint
  supplementary.rs     # Supplementary file download (non-WASM)
```

### Key Types

- `Client` — Unified client with `pubmed` and `pmc` fields; convenience methods: `search_with_full_text`, `fetch_articles`, `fetch_summaries`, `search_and_fetch_summaries`, `get_related_articles`, `get_pmc_links`, `get_citations`, `match_citations`, `global_query`, `get_database_list`, `get_database_info`, `epost`, `fetch_all_by_pmids`, `spell_check`
- `PubMedClient` — Search, fetch metadata, ESummary, EPost/History, ELink, EInfo, ECitMatch, EGQuery, ESpell
- `PmcClient` — Fetch full-text, check availability, extract figures, download OA files from the PMC OA Cloud (AWS S3)
- `EuropePmcClient` — Europe PMC REST API: cross-source search, JATS full text, references/citations/database links, supplementary downloads. Addressed by `EuropePmcId` (`(source, id)`); needs no API key
- `SearchQuery` — Builder pattern for complex queries with filters, date ranges, boolean logic
- `PubMedArticle` — Article metadata (title, authors, abstract, MeSH, keywords, etc.) — defined in `pubmed-parser`
- `PmcArticle` — Structured JATS full-text (front/body/back; sections, references, figures, tables) — defined in `pubmed-parser`
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

**Type stub (`pubmed_client.pyi`) is generated, never hand-edited.** It is produced from the `#[gen_stub_pyclass]`/`#[gen_stub_pymethods]` annotations by `src/bin/stub_gen.rs`, which also splices in the members `pyo3-stub-gen` can't see (`__version__` and the `create_exception!` hierarchy — the latter listed explicitly in `stub_gen.rs`). Every new `#[pyclass]` needs `#[gen_stub_pyclass]` and every `#[pymethods]` block needs `#[gen_stub_pymethods]`, or it silently disappears from the stub. After changing the PyO3 API: `MISE_ENV=python mise run stubgen:py` to regenerate, `MISE_ENV=python mise run stubtest:py` to verify against the compiled module, then commit the `.pyi`. CI's `Python Type Stub Check` job (`ci-python.yml`) fails on `git diff` if the checked-in stub is stale and runs `stubtest` so it can't drift from runtime. `stubtest-allowlist.txt` records intentionally-unstubbed names.

### Go Bindings (`pubmed-client-go/`)

Go bindings via cgo. Go module: `github.com/illumination-k/pubmed-client/pubmed-client-go`, package name `pubmedclient`. Synchronous but cancellable API with an internal Tokio runtime (same pattern as Python/R). Covers PubMed search/metadata (ESearch, EFetch, ESummary), the discovery APIs (ELink, EInfo, EGQuery, ECitMatch, ESpell), PMC full text / XML / Markdown / OA downloads, a query builder, and citation export.

- **Two layers**: a C-ABI shim crate at `rust/` (crate `pubmed-client-go`, `crate-type = ["staticlib", "lib"]`, `publish = false`) and the Go package at the directory root. Unlike the R crate this **is** a workspace member — no external toolchain is needed to compile it — so workspace clippy/nextest cover it automatically.
- **JSON boundary**: values cross FFI as JSON strings rather than mirrored C structs. Each call returns one owned `char *` (null + `out_err` on failure); Go re-parses into the typed structs in `models.go`. This keeps the C surface at a few dozen functions, and because `encoding/json` ignores unknown keys the Rust models can gain fields without breaking Go. `PmcArticleDto` in `rust/src/dto.rs` flattens the nested JATS tree via `PmcArticle`'s accessors.
- **Errors cross as a JSON envelope** (`{"kind": …, "message": …, "status": …}`, see `rust/src/error.rs`). The `kind` is what lets Go expose sentinels (`ErrNotFound`, `ErrPMCNotAvailable`, `ErrRateLimited`, …) instead of matching on message text. Go treats an unparseable envelope as `KindUnknown` with the whole string as the message, so new kinds never break an older caller. `From<PubMedError>` matches exhaustively on purpose: a new upstream variant should fail the build rather than silently degrade to `internal`.
- **`context.Context` is real cancellation, not advisory.** Every call takes a nullable `PubmedCancel` token (`rust/src/cancel.rs`, a `tokio::sync::watch` channel selected against the request future). Go's `Client.call` allocates one, wires a watchdog goroutine to `ctx.Done()`, and joins that goroutine before freeing the token — firing a freed token would be a use-after-free. A context that can never be cancelled (`context.Background()`) skips the token and the goroutine. Note `watch::Sender::send` fails when there are no receivers, so the trigger uses `send_replace`; `send` would silently lose a token fired before the call subscribed.
- **The query builder is replayed, not reimplemented** (`rust/src/query.rs`). Go's `SearchQuery` records the builder calls as a JSON op list and ships them to Rust, which replays them onto the real `SearchQuery`. Field tags therefore have one implementation across every binding. Ops serialize through a hand-written `MarshalJSON` rather than struct tags: the operations take genuinely different arguments, and `omitempty` on a shared struct would quietly drop a zero year or an empty term.
- **Export moves data back into Rust** (`rust/src/export.rs`). Go marshals `Article` with `omitempty`, so unset fields arrive missing, which `PubMedArticle` (no `#[serde(default)]`) rejects — including nested `Author.affiliations` and `MeshTerm.qualifiers`. Each object is merged over a template first; arrays in the template carry a prototype element applied to each incoming element, and a missing array defaults to empty. The template is written out field by field so a new model field breaks the build instead of failing at runtime.
- **Panics are caught** (`catch_unwind`) at every boundary function — an `extern "C"` fn that unwinds would abort the Go process.
- `guard` in `rust/src/ffi.rs` is `pub(crate)` on purpose: it writes through `out_err` without being an `unsafe fn`, which clippy's `not_unsafe_ptr_arg_deref` rejects for publicly reachable functions.
- **rustls, not native-tls**: the shim depends on `pubmed-client` with `default-features = false, features = ["rustls-tls"]` so the archive needs no system OpenSSL. Build it with `cargo build -p pubmed-client-go`; under `cargo build --workspace` feature unification pulls `native-tls` back in and the archive would reference OpenSSL again.
- **The `pubmed-client` dep is path-only** (no `version`, not `workspace = true`): inheriting a workspace dependency forbids overriding `default-features`, and the crate is never published. Nothing for `scripts/sync-versions.sh` to sync.
- Build: there is no Makefile — everything is a mise task in `mise.go.toml` (`MISE_ENV=go`). `go:build` compiles the archive into `lib/$GOOS_$GOARCH/` (gitignored, ~75 MB); `go:test` and `lint:go` `depends` on it, since a cgo package cannot even be type-checked without the archive. `ffi.go` is the only file that touches C; the cgo link line carries per-platform `LDFLAGS`.
- `lint:go` captures `gofmt -l` **stdout only**. Merging stderr picks up mise's own debug chatter (CI runs mise at debug level) and reports a spurious formatting failure on a clean tree.
- mise runs task scripts with `sh`, not bash — no `pipefail`.
- Tests: offline Go tests point `Config.BaseURL` at an `httptest` server (`stub_test.go`, one canned payload per E-utilities endpoint), exercising the whole chain (Go → cgo → Rust → HTTP → parse → JSON → Go structs) with no network. Live tests are behind the `integration` build tag plus `PUBMED_REAL_API_TESTS=1`. The Rust shim has its own boundary tests (null pointers, invalid JSON, double free, cancellation, query replay, export merge). When writing a test that blocks an `httptest` handler until the test releases it, register `defer server.Close()` **before** `defer close(release)` — `Server.Close` joins outstanding requests, so the other order deadlocks.
- CI: `.github/workflows/ci-go.yml` — `lint` job (gofmt + vet) and a `test` matrix over ubuntu/macOS, since the cgo link line differs per platform. Windows is unverified (cgo needs the `x86_64-pc-windows-gnu` target, not Cargo's default msvc).

### R Bindings (`pubmed-client-r/`)

R bindings via [extendr](https://extendr.github.io/). R package name: `pubmedclient`. Synchronous API with internal Tokio runtime (same pattern as Python). Currently an **MVP** covering core operations: `pubmed_client()`, `pubmed_search()`, `pubmed_fetch()`, `pubmed_search_and_fetch()`, `pmc_fulltext()`, `pmc_to_markdown()`.

- **NOT a Cargo workspace member**: the inner crate (`src/rust/`) declares an empty `[workspace]` table so the root `cargo build`/CI never tries to build it — linking requires the R toolchain (`libR`), which isn't always present.
- Rust source: `src/rust/src/lib.rs` (extendr free functions; client handle passed as an `ExternalPtr`). R-level API: `R/pubmed-client.R`. Low-level `.Call` wrappers: `R/extendr-wrappers.R` (regenerate with `rextendr::document("pubmed-client-r")` and keep in sync with the `extendr_module!` block).
- The inner crate depends on the **published `pubmed-client`** (crates.io), not a workspace path — `R CMD check`/source-tarball installs build in an isolated copy where a relative path outside the package can't resolve. Keep its version in lock-step with the workspace; for local dev against unpublished changes, temporarily switch to `{ path = "../../../pubmed-client" }`.
- Build/install: `R CMD INSTALL pubmed-client-r` or `remotes::install_local("pubmed-client-r")` (requires `cargo`/`rustc`).
- Tests: `testthat` (edition 3) in `tests/testthat/`. Offline tests cover client construction + input validation; live-API tests are gated behind `PUBMED_REAL_API_TESTS=1` (same convention as the Rust crate).
- CI: `.github/workflows/ci-r.yml` — `rust-fmt` job (rustfmt over the non-workspace inner crate) + `R-CMD-check` (ubuntu, R via apt + Posit binary CRAN mirror; runs `rcmdcheck`). Avoids `r-lib/actions` so every action stays pinned to a full commit SHA (enforced by `ghalint`).

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

MCP server for AI assistants (Claude Desktop, etc.) built with rmcp. Communicates via stdio, or over
streamable HTTP at `/mcp` with `--port`.

Released as a multi-arch container image to GHCR (`ghcr.io/illumination-k/pubmed-mcp`) alongside the
crates.io publish, from `pubmed-mcp/Dockerfile` (build context is the **workspace root** — the crate
depends on its siblings by path; `.dockerignore` at the root trims the context but must keep every
workspace member, or the manifest fails to load). `ARG RUST_VERSION` must match `rust-toolchain.toml`;
`ci-docker.yml` fails the build if it drifts and also smoke-tests the image (`--help` + an MCP
`initialize` handshake over stdio).

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
- **`pubmed-client`** tests: `comprehensive_pmc_tests`, `comprehensive_pubmed_tests`, `comprehensive_elink_tests`, `comprehensive_einfo_tests`, `test_figure_extraction`, `mocked_cloud` (PMC OA Cloud/S3 listing & download), `test_pmc_cache`, `test_webenv`, `test_batch_fetch_mocked`

## Guidelines

### Releasing & Versioning

All publishable packages share **one unified version**, sourced from `[workspace.package] version`
in the root `Cargo.toml` and propagated by `scripts/sync-versions.sh` (napi `package.json` +
optionalDependencies, wasm `package.json`, py `pyproject.toml`). CI enforces it via the
**Version Consistency** job. Never hand-edit a single package's version — run `mise run release <ver>`.
A single `v<semver>` tag publishes everything (crates.io + npm + PyPI) via `release.yml`.
See [RELEASING.md](RELEASING.md) for the full process.

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

**pubmed-formatter**: `pubmed-parser`, `serde`, `serde_json`, `serde_norway` (maintained `serde_yaml` fork), `regex`, `tracing`.

**pubmed-client**: `pubmed-parser`, `pubmed-formatter`, `tokio`, `reqwest`, `serde`, `moka` (caching), `rand`, `image`, `futures-util`. PMC OA files are downloaded per-file from the PMC OA Cloud (AWS S3) over plain HTTP via `reqwest` — no tar/gzip deps.

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
