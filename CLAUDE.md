# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust client library for accessing PubMed and PMC (PubMed Central) APIs. It provides async interfaces for searching biomedical research articles and fetching full-text content.

## Commands

### Build & Development

```bash
# Build the project
cargo build

# Run tests with nextest (preferred test runner)
mise r test

# Run tests with cargo (fallback)
cargo test

# Run a specific test
cargo nextest run test_name

# Run tests in watch mode
mise r test:watch

# Generate and open documentation
cargo doc --open

# Check code without building
cargo check
```

### Code Quality

```bash
# Full linting (dprint + cargo fmt + clippy)
mise r lint

# Format code (dprint + cargo fmt)
mise r fmt

# Run clippy only
cargo clippy -- -D warnings
```

### Code Coverage

```bash
# Generate HTML coverage report and open in browser
mise r coverage:open

# Generate coverage report (HTML format)
mise r coverage

# Generate LCOV format for CI
mise r coverage:lcov
```

### Debugging & Logging

```bash
# Run tests with tracing output (structured logging)
RUST_LOG=debug cargo test -- --nocapture

# Run specific test with tracing
RUST_LOG=pubmed_client_rs=debug cargo test --test test_integration_abstract -- --nocapture

# Different log levels
RUST_LOG=info cargo test        # Info level and above
RUST_LOG=debug cargo test       # Debug level and above
RUST_LOG=trace cargo test       # All tracing output
```

## Architecture

### Module Structure

- `src/lib.rs` - Main library entry point, re-exports public API
- `src/pubmed.rs` - PubMed API client for article metadata search and retrieval
- `src/pmc/` - PMC module directory
  - `client.rs` - PMC client for full-text article access
  - `parser.rs` - XML parsing logic for PMC content
  - `models.rs` - Data structures for PMC articles
  - `markdown.rs` - Converter from PMC XML to Markdown format
- `src/query.rs` - Query builder for constructing filtered searches
- `src/error.rs` - Error types and result aliases

### Key Types

- `Client` - Combined client that provides both PubMed and PMC functionality
- `PubMedClient` - Handles article metadata searches via PubMed E-utilities
- `PmcClient` - Fetches structured full-text from PubMed Central
- `SearchQuery` - Builder pattern for constructing complex search queries with filters
- `PmcArticle` - Structured representation of PMC full-text articles

### API Design Patterns

- Async/await using tokio runtime
- Builder pattern for search queries
- Result<T> type alias for error handling
- Separation of metadata (PubMed) and full-text (PMC) concerns
- Support for custom HTTP clients via reqwest
- Data-driven testing with real PMC XML samples in `tests/test_data/`
- Structured logging with tracing for debugging and monitoring

### Testing

- Test runner: `cargo-nextest` for better output and parallelization
- Parameterized tests using `rstest`
- Test data: Real PMC XML files in `tests/test_data/pmc_xml/`
- Common test utilities in `tests/common/mod.rs`
- Integration tests with tracing support using `#[traced_test]`

### Dependencies

- `tokio` - Async runtime
- `reqwest` - HTTP client
- `serde` - Serialization/deserialization
- `quick-xml` - XML parsing for PMC content
- `thiserror` - Error type derivation
- `anyhow` - Error handling utilities
- `tracing` - Structured logging and instrumentation
- `rstest` - Parameterized testing (dev dependency)
- `tracing-subscriber` - Tracing output formatting (dev dependency)
- `tracing-test` - Tracing support for tests (dev dependency)

### Logging & Debugging

The library uses `tracing` for structured logging. Key instrumentation points include:

- API requests to NCBI E-utilities (ESearch, EFetch)
- XML parsing operations with performance metrics
- Article retrieval with metadata extraction
- Error conditions with context

Enable logging in your application:

```rust
use tracing_subscriber;

// Initialize tracing subscriber
tracing_subscriber::fmt::init();

// Library operations will now produce structured logs
let client = pubmed_client_rs::PubMedClient::new();
let article = client.fetch_article("31978945").await?;
```

Log levels and structured fields:

- `INFO`: High-level operations (article fetched, search completed)
- `DEBUG`: Detailed operations (API requests, XML parsing steps)
- `WARN`: Error conditions and fallbacks
- Structured fields: `pmid`, `title`, `authors_count`, `has_abstract`, `abstract_length`
