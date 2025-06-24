# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust client library for accessing PubMed and PMC (PubMed Central) APIs. It provides async interfaces for searching biomedical research articles and fetching full-text content.

## Commands

### Build & Development

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Generate and open documentation
cargo doc --open

# Check code without building
cargo check
```

### Code Quality

```bash
# Run clippy for linting
cargo clippy

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt --check

# Format non-Rust files (JSON, TOML, Markdown, YAML)
dprint fmt

# Check dprint formatting
dprint check
```

## Architecture

### Module Structure

- `src/lib.rs` - Main library entry point, re-exports public API
- `src/pubmed.rs` - PubMed API client for article metadata search and retrieval
- `src/pmc.rs` - PMC client for full-text article access
- `src/query.rs` - Query builder for constructing filtered searches
- `src/error.rs` - Error types and result aliases

### Key Types

- `Client` - Combined client that provides both PubMed and PMC functionality
- `PubMedClient` - Handles article metadata searches via PubMed E-utilities
- `PmcClient` - Fetches structured full-text from PubMed Central
- `SearchQuery` - Builder pattern for constructing complex search queries with filters

### API Design Patterns

- Async/await using tokio runtime
- Builder pattern for search queries
- Result<T> type alias for error handling
- Separation of metadata (PubMed) and full-text (PMC) concerns
- Support for custom HTTP clients via reqwest

### Dependencies

- `tokio` - Async runtime
- `reqwest` - HTTP client
- `serde` - Serialization/deserialization
- `quick-xml` - XML parsing for PMC content
- `thiserror` - Error type derivation
- `anyhow` - Error handling utilities
