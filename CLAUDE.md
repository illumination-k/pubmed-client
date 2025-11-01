# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust workspace containing PubMed and PMC (PubMed Central) API clients with multiple language bindings. The workspace includes a core Rust library, WebAssembly bindings for JavaScript/TypeScript environments, Python bindings via PyO3, a command-line interface for common operations, and a Model Context Protocol (MCP) server for AI assistant integration.

## Package Name

python bindings package name registered in PyPI: `pubmed-client-py`

## Important Guidelines for Git Operations

### File and Directory Renaming

**CRITICAL**: When renaming files or directories in this git repository, ALWAYS use `git mv` instead of shell commands like `mv`, `rename`, or file system operations.

#### Why use `git mv`?

1. **Preserves Git History**: Git properly tracks the rename, allowing `git log --follow` to show the complete file history
2. **Clean Commit Diffs**: Shows as `renamed:` instead of delete + add, making code review easier
3. **Maintains Blame Information**: File annotations and blame history follow through the rename
4. **Automatic Staging**: The rename is automatically staged for commit

#### Correct Renaming Process:

```bash
# ✅ CORRECT - Use git mv to rename files/directories
git mv old-name new-name

# Then update references in code and documentation
sed -i '' 's/old-name/new-name/g' affected-files

# Stage all changes together
git add -A

# Verify git recognizes the rename
git status  # Should show "renamed: old-name -> new-name"
```

#### Incorrect Renaming Process:

```bash
# ❌ WRONG - Do NOT use shell mv command
mv old-name new-name  # Breaks git history tracking!

# ❌ WRONG - Do NOT manually delete and recreate
rm -rf old-name
mkdir new-name  # Git sees this as delete + add, not rename
```

#### If You Accidentally Used mv:

If you've already renamed files/directories using `mv`, here's how to recover:

```bash
# 1. Restore to original state
git restore .
rm -rf new-name  # Remove manually created directory

# 2. Use git mv properly
git mv old-name new-name

# 3. Re-apply your content changes
# (edit files, update references, etc.)

# 4. Stage everything
git add -A
```

#### Examples in This Project:

- **Package Rename**: When renaming `pubmed-mcp-server` to `pubmed-mcp`:
  ```bash
  git mv pubmed-mcp-server pubmed-mcp
  # Then update Cargo.toml, documentation, etc.
  git add -A
  ```

- **File Rename**: When renaming source files:
  ```bash
  git mv src/old_module.rs src/new_module.rs
  # Update imports and references
  git add -A
  ```

### General Git Best Practices

- **Always check `git status`** before and after operations
- **Use `git diff --cached`** to review staged changes before committing
- **Test builds and tests** after renaming operations
- **Keep renames and content changes** in the same commit for context

## Version Management and Publishing

### Workspace Version Synchronization

**CRITICAL**: This workspace uses a shared version number across all packages. When bumping the version for publishing, you MUST update version requirements in all dependent packages.

#### Current Workspace Structure

The workspace version is defined in the root `Cargo.toml`:

```toml
[workspace.package]
version = "0.1.0" # Shared version for all packages
```

All packages use this workspace version via `version.workspace = true`.

#### Internal Dependencies Require Version Specifications

For crates.io publishing, all path dependencies MUST include explicit version requirements:

```toml
# ✅ CORRECT - Required for publishing to crates.io
pubmed-client-rs = { version = "0.1.0", path = "../pubmed-client" }

# ❌ WRONG - Will fail cargo package
pubmed-client-rs = { path = "../pubmed-client" }
```

**Why this is required:**

- Cargo requires version requirements for all path dependencies when packaging for crates.io
- During publishing, the `path` is removed and only the `version` is kept
- This ensures published packages depend on crates.io versions, not local paths

#### Version Bump Checklist

When bumping the version for a new release, follow these steps:

1. **Update workspace version** in root `Cargo.toml`:
   ```toml
   [workspace.package]
   version = "0.2.0" # New version
   ```

2. **Update all internal dependency versions** in these files:
   - `pubmed-cli/Cargo.toml` - Update `pubmed-client-rs` version
   - `pubmed-client-py/Cargo.toml` - Update `pubmed-client-rs` version
   - `pubmed-client-wasm/Cargo.toml` - Update `pubmed-client-rs` version
   - `pubmed-mcp/Cargo.toml` - Update `pubmed-client` version

3. **Update Cargo.lock**:
   ```bash
   cargo update --workspace
   ```

4. **Verify all packages can be packaged**:
   ```bash
   cargo package --allow-dirty
   ```

5. **Run tests** to ensure everything works:
   ```bash
   cargo test --workspace
   ```

6. **Create version bump commit**:
   ```bash
   git add Cargo.toml pubmed-cli/Cargo.toml pubmed-client-py/Cargo.toml \
           pubmed-client-wasm/Cargo.toml pubmed-mcp/Cargo.toml Cargo.lock
   git commit -m "chore: Bump version to 0.2.0"
   ```

#### Example: Bumping from 0.1.0 to 0.2.0

**Before:**

```toml
# Cargo.toml (root)
[workspace.package]
version = "0.1.0"

# pubmed-cli/Cargo.toml
pubmed-client-rs = { version = "0.1.0", path = "../pubmed-client" }

# pubmed-client-py/Cargo.toml
pubmed-client-rs = { version = "0.1.0", path = "../pubmed-client" }

# pubmed-client-wasm/Cargo.toml
pubmed-client-rs = { version = "0.1.0", path = "../pubmed-client" }

# pubmed-mcp/Cargo.toml
pubmed-client = { version = "0.1.0", path = "../pubmed-client", package = "pubmed-client-rs" }
```

**After:**

```toml
# Cargo.toml (root)
[workspace.package]
version = "0.2.0"

# pubmed-cli/Cargo.toml
pubmed-client-rs = { version = "0.2.0", path = "../pubmed-client" }

# pubmed-client-py/Cargo.toml
pubmed-client-rs = { version = "0.2.0", path = "../pubmed-client" }

# pubmed-client-wasm/Cargo.toml
pubmed-client-rs = { version = "0.2.0", path = "../pubmed-client" }

# pubmed-mcp/Cargo.toml
pubmed-client = { version = "0.2.0", path = "../pubmed-client", package = "pubmed-client-rs" }
```

#### Automated Version Update Script (Future Enhancement)

Consider creating a script to automate version updates:

```bash
#!/bin/bash
# scripts/bump-version.sh

NEW_VERSION=$1
if [ -z "$NEW_VERSION" ]; then
    echo "Usage: $0 <new-version>"
    exit 1
fi

# Update workspace version
sed -i '' "s/^version = .*/version = \"$NEW_VERSION\"/" Cargo.toml

# Update internal dependencies
for file in pubmed-cli/Cargo.toml pubmed-client-py/Cargo.toml pubmed-client-wasm/Cargo.toml pubmed-mcp/Cargo.toml; do
    sed -i '' "s/version = \"[^\"]*\", path = \"/version = \"$NEW_VERSION\", path = \"/" "$file"
done

# Update Cargo.lock
cargo update --workspace

echo "Version bumped to $NEW_VERSION"
echo "Please review changes and run: cargo package --allow-dirty"
```

#### Common Errors and Solutions

**Error: "dependency does not specify a version"**

```
error: all dependencies must have a version requirement specified when packaging.
dependency `pubmed-client-rs` does not specify a version
```

**Solution:** Add explicit version to the path dependency:

```toml
pubmed-client-rs = { version = "0.1.0", path = "../pubmed-client" }
```

**Error: Version mismatch between workspace and dependencies**

If you update the workspace version but forget to update internal dependencies, the published packages will depend on the old version from crates.io.

**Solution:** Always update all version references together using the checklist above.

## Important Guidelines for PubMed Search Query Implementation

**CRITICAL**: When implementing or modifying PubMed search query functionality, ALWAYS reference the official NCBI PubMed documentation:

- **Primary Resource**: https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags
- **E-utilities Documentation**: https://www.ncbi.nlm.nih.gov/books/NBK25499/

### Search Field Tag Validation

Before implementing new search field tags or modifying existing ones:

1. **Verify field tags** against the official PubMed help documentation
2. **Use correct tag syntax** - PubMed uses short form tags (e.g., `[ti]` for Title, `[au]` for Author)
3. **Test with real API calls** when possible to ensure compatibility
4. **Document any limitations** or special requirements for field tags

### Currently Validated Field Tags

The following field tags have been verified against NCBI documentation:

- `[ti]` - Title
- `[tiab]` - Title/Abstract
- `[au]` - Author
- `[1au]` - First Author
- `[lastau]` - Last Author
- `[ad]` - Affiliation
- `[ta]` - Journal Title Abbreviation
- `[la]` - Language
- `[pt]` - Publication Type
- `[mh]` - MeSH Terms
- `[majr]` - MeSH Major Topic
- `[sh]` - MeSH Subheading
- `[gr]` - Grant Number
- `[auid]` - Author Identifier (ORCID)
- `[pdat]` - Publication Date
- `[edat]` - Entry Date
- `[mdat]` - Modification Date
- `[sb]` - Subset (e.g., "free full text[sb]")

### Invalid or Non-existent Tags

These tags do NOT exist in PubMed and should not be used:

- `[Organism]` - Use MeSH terms with `[mh]` instead
- `[lang]` - Deprecated, use `[la]`
- Long-form tags like `[Title]`, `[Author]`, etc. - Use short forms

When in doubt, always check the official documentation before implementation.

## Workspace Structure

```
pubmed-client-rs/                    # Cargo workspace root
├── Cargo.toml                       # Workspace definition
├── pubmed-client/                   # Core Rust library
│   ├── Cargo.toml                   # Rust library configuration
│   └── src/                         # Core library source code
├── pubmed-client-wasm/              # WASM bindings for npm
│   ├── Cargo.toml                   # WASM crate configuration
│   ├── package.json                 # npm package configuration
│   ├── src/lib.rs                   # WASM bindings source
│   ├── pkg/                         # Generated WASM package
│   ├── tests/                       # WASM-specific TypeScript tests
│   └── *.json, *.ts                # TypeScript/npm configuration
├── pubmed-client-py/                # Python bindings (PyO3)
│   ├── Cargo.toml                   # PyO3 crate configuration
│   ├── pyproject.toml               # Python package configuration
│   ├── src/lib.rs                   # PyO3 bindings source
│   ├── pubmed_client.pyi            # Type stub file for IDE support
│   ├── py.typed                     # PEP 561 type marker
│   ├── tests/                       # Python tests (pytest)
│   │   ├── test_config.py           # Configuration tests
│   │   ├── test_client.py           # Client initialization tests
│   │   ├── test_models.py           # Data model tests
│   │   ├── test_integration.py      # Integration tests
│   │   └── conftest.py              # Pytest fixtures
│   └── pytest.ini                   # Pytest configuration
├── pubmed-cli/                      # Command-line interface
│   ├── Cargo.toml                   # CLI crate configuration
│   └── src/                         # CLI source code
│       ├── main.rs                  # CLI entry point
│       └── commands/                # CLI subcommands
│           ├── convert.rs           # PMID to PMCID conversion
│           ├── figures.rs           # Figure extraction
│           ├── markdown.rs          # Markdown conversion
│           └── search.rs            # PubMed search
├── pubmed-mcp/               # MCP server for AI assistants
│   ├── Cargo.toml                   # MCP server configuration
│   ├── src/main.rs                  # MCP server implementation
│   ├── tests/                       # Integration tests
│   │   └── integration_test.rs      # MCP protocol tests
│   └── README.md                    # MCP server documentation
└── tests/                           # Shared integration tests
```

## Commands

### Build & Development

#### Workspace Commands (from root)

```bash
# Build all workspace members
cargo build

# Test all workspace members
cargo test

# Check all workspace members
cargo check

# Run specific integration test suite (from pubmed-client/ directory)
cd pubmed-client && cargo test --test comprehensive_pmc_tests
cd pubmed-client && cargo test --test comprehensive_pubmed_tests
cd pubmed-client && cargo test --test test_elink_integration

# Run new real API integration tests (requires network and env var, from pubmed-client/)
cd pubmed-client && PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pubmed_api_tests
cd pubmed-client && PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pmc_api_tests
cd pubmed-client && PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test error_handling_tests

# Run single unit test in core library
cargo test --lib -p pubmed-client-rs pubmed::parser::tests::test_mesh_term_parsing
```

#### Core Library Commands (from pubmed-client/)

```bash
# Build only the core library
cargo build

# Test only the core library
cargo test

# Generate documentation
cargo doc --open
```

#### WASM Package Commands (from pubmed-client-wasm/)

```bash
# Build WASM package for Node.js
pnpm run build

# Build for different targets
pnpm run build:web      # For web/browser
pnpm run build:bundler  # For bundlers
pnpm run build:all      # All targets

# Run TypeScript tests
pnpm run test
pnpm run test:watch     # Watch mode
pnpm run test:coverage  # With coverage

# Publish to npm
pnpm run publish        # wasm-pack publish --access public
```

#### Python Package Commands (from pubmed-client-py/)

```bash
# Build Python package with maturin
uv run --with maturin maturin develop        # Development build
uv run --with maturin maturin develop --quiet # Quiet mode

# Run Python tests (all tests)
uv run pytest                                 # Run all tests
uv run pytest -v                              # Verbose output
uv run pytest -m "not integration"            # Unit tests only
uv run pytest -m integration                  # Integration tests only

# Run specific test files
uv run pytest tests/test_config.py            # Configuration tests
uv run pytest tests/test_client.py            # Client tests
uv run pytest tests/test_models.py            # Data model tests
uv run pytest tests/test_integration.py       # Integration tests

# Run tests with coverage
uv run pytest --cov=pubmed_client --cov-report=html
uv run pytest --cov=pubmed_client --cov-report=term

# Type checking with mypy
uv run mypy tests/ --strict                   # Type check tests
uv run mypy tests/ --strict --no-error-summary # Quiet mode

# Code quality checks
uv run ruff check .                           # Linting
uv run ruff format .                          # Formatting

# Install development dependencies
uv sync --group dev                           # Sync all dev dependencies
```

#### CLI Commands (from workspace root)

```bash
# Run CLI with cargo from workspace root
cargo run -p pubmed-cli -- <COMMAND>

# Example: Get help
cargo run -p pubmed-cli -- --help

# Example: Extract figures from PMC articles
cargo run -p pubmed-cli -- figures PMC7906746

# Example: Extract figures and save failed PMC IDs to file
cargo run -p pubmed-cli -- figures PMC7906746 PMC123456 --failed-output failed_pmcids.txt

# Example: Convert PMC to markdown
cargo run -p pubmed-cli -- markdown PMC7906746

# Example: Convert PMID to PMCID (JSON format)
cargo run -p pubmed-cli -- pmid-to-pmcid 31978945

# Example: Convert PMID to PMCID (CSV format)
cargo run -p pubmed-cli -- pmid-to-pmcid 31978945 --format csv

# Example: Convert PMID to PMCID (TXT format - PMCIDs only)
cargo run -p pubmed-cli -- pmid-to-pmcid 31978945 --format txt

# Example: Convert multiple PMIDs
cargo run -p pubmed-cli -- pmid-to-pmcid 31978945 33515491

# Example: Convert many PMIDs with custom batch size (to avoid API errors)
cargo run -p pubmed-cli -- pmid-to-pmcid 31978945 33515491 25760099 --batch-size 50

# Example: Process large list with smaller batches
cargo run -p pubmed-cli -- pmid-to-pmcid $(cat pmids.txt) --batch-size 25

# Debug CLI with verbose logging
RUST_LOG=debug cargo run -p pubmed-cli -- pmid-to-pmcid 31978945

# Use API key for higher rate limits
cargo run -p pubmed-cli -- --api-key YOUR_API_KEY pmid-to-pmcid 31978945

# Specify email and tool name
cargo run -p pubmed-cli -- --email you@example.com --tool MyApp pmid-to-pmcid 31978945
```

#### mise Commands (if available)

Using `mise` for task management (configured in `.mise.toml`):

```bash
# Build all workspace members
mise r build

# Run all workspace tests with nextest (preferred test runner)
mise r test

# Run tests in watch mode
mise r test:watch

# Run tests with verbose output
mise r test:verbose

# Generate and open documentation
mise r doc

# Check code without building
mise r check

# Core library specific
mise r core:test              # Run core library tests
mise r core:test:integration  # Run integration tests
mise r core:publish           # Publish to crates.io

# WASM package specific
mise r wasm:build            # Build for web target
mise r wasm:build:node       # Build for Node.js target
mise r wasm:test             # Build and run TypeScript tests
mise r wasm:publish          # Publish to npm
```

#### MCP Server Commands (from workspace root)

```bash
# Build MCP server
cargo build --release -p pubmed-mcp

# Run MCP server (stdio transport)
cargo run -p pubmed-mcp

# Run with debug logging
RUST_LOG=debug cargo run -p pubmed-mcp

# Run tests
cargo test -p pubmed-mcp

# Test with MCP Inspector (interactive testing tool)
npx @modelcontextprotocol/inspector cargo run -p pubmed-mcp
```

### Code Quality

#### Rust Code Quality

```bash
# Full linting (dprint + cargo fmt + clippy) for entire workspace
mise r lint

# Format code (dprint + cargo fmt) for entire workspace
mise r fmt
```

#### WASM/TypeScript Code Quality (from pubmed-client-wasm/)

The WASM package uses Biome for TypeScript/JavaScript linting and formatting:

```bash
# Format TypeScript and JavaScript files
pnpm run format

# Run linting
pnpm run lint

# Format and lint (recommended)
pnpm run check

# CI mode (fails on issues, no auto-fix)
pnpm run ci
```

### Code Coverage

```bash
# Generate HTML coverage report and open in browser
mise r coverage:open

# Generate coverage report (HTML format)
mise r coverage

# Generate LCOV format for CI
mise r coverage:lcov

# Generate JSON format for tooling
mise r coverage:json
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

### Core Library Architecture (pubmed-client/)

- `src/lib.rs` - Main library entry point, re-exports public API and unified `Client`
- `src/pubmed/` - PubMed module directory
  - `client.rs` - PubMed API client for article metadata search and ELink API
  - `parser.rs` - XML parsing module functions for PubMed article metadata (refactored from empty struct)
  - `models.rs` - Data structures for PubMed articles, ELink results
  - `responses.rs` - Internal API response structures for JSON/XML parsing
  - `mod.rs` - Module exports and public API
- `src/pmc/` - PMC module directory
  - `client.rs` - PMC client for full-text article access
  - `parser/` - XML parsing modules for PMC content (refactored to module functions)
    - `mod.rs` - Main parser module with `parse_pmc_xml()` function
    - `author.rs` - Author extraction functions
    - `metadata.rs` - Metadata extraction functions
    - `reference.rs` - Reference extraction functions
    - `section.rs` - Section parsing functions
    - `xml_utils.rs` - XML utility functions
  - `models.rs` - Data structures for PMC articles
  - `markdown.rs` - Converter from PMC XML to Markdown format
- `src/pubmed/query/` - Advanced query builder system with filters, date ranges, boolean logic
  - `builder.rs` - Main SearchQuery builder implementation
  - `filters.rs` - Field-specific search filters (title, author, journal, etc.)
  - `dates.rs` - Date range and publication date filtering
  - `boolean.rs` - Boolean logic operations (AND, OR, NOT)
  - `advanced.rs` - Advanced search features (MeSH terms, article types)
  - `validation.rs` - Query validation and error handling
- `src/error.rs` - Error types and result aliases
- `src/config.rs` - Client configuration options for API keys and rate limiting
- `src/rate_limit.rs` - Rate limiting implementation using token bucket algorithm
- `src/retry.rs` - Retry logic with exponential backoff for network failures

### WASM Bindings Architecture (pubmed-client-wasm/)

The WASM package provides JavaScript/TypeScript bindings for the core Rust library:

- `src/lib.rs` - WASM bindings entry point with JavaScript-compatible types
- `package.json` - npm package configuration for publishing
- `pkg/` - Generated WASM package output (created by wasm-pack)
- `tests/` - TypeScript tests for WASM bindings using Vitest
- Configuration files: `tsconfig.json`, `vitest.config.ts`, `biome.json`

**Key WASM Types:**

- `WasmPubMedClient` - JavaScript wrapper around the core Rust client
- `WasmClientConfig` - JavaScript-friendly configuration object
- `JsArticle`, `JsFullText` - JavaScript-compatible data structures
- Promise-based async API matching modern JavaScript patterns

### Key Types

- `Client` - Combined client that provides both PubMed and PMC functionality
- `PubMedClient` - Handles article metadata searches and ELink API via PubMed E-utilities
- `PmcClient` - Fetches structured full-text from PubMed Central
- `SearchQuery` - Builder pattern for constructing complex search queries with filters
- `PubMedArticle` - Article metadata from PubMed (title, authors, abstract, etc.)
- `PmcFullText` - Structured representation of PMC full-text articles
- `RelatedArticles`, `PmcLinks`, `Citations` - ELink API result types for discovering article relationships
- `ClientConfig` - Configuration for API keys, rate limiting, and client behavior
- `RateLimiter` - Token bucket rate limiter for NCBI API compliance

### API Design Patterns

- Async/await using tokio runtime
- Builder pattern for search queries (`SearchQuery`)
- Result<T> type alias for error handling
- Module functions instead of empty structs for parsers (clean functional design)
- Separation of metadata (PubMed) and full-text (PMC) concerns
- ELink API integration for discovering article relationships (related articles, citations, PMC links)
- Support for custom HTTP clients via reqwest
- Internal response types separate from public API models
- Data-driven testing with real PMC XML samples in `tests/test_data/`
- Structured logging with tracing for debugging and monitoring
- Rate limiting with token bucket algorithm for NCBI API compliance
- Configurable API keys, email, and tool identification for NCBI guidelines

**Parser Design Philosophy:**
The parsers have been refactored from empty structs with static methods to module functions, following Rust's idiomatic patterns. This eliminates unnecessary type definitions while maintaining clear module boundaries and namespacing. Key parser functions include:

- `parse_article_from_xml()` - Main PubMed XML parser
- `parse_pmc_xml()` - Main PMC XML parser
- Module-specific extraction functions (e.g., `extract_authors()`, `extract_references()`)

### Testing

- Test runner: `cargo-nextest` for better output and parallelization
- Parameterized tests using `rstest`
- Test data: Real XML files in `pubmed-client/tests/integration/test_data/`
- Common test utilities in `pubmed-client/tests/integration/common/mod.rs`
- Integration tests with tracing support using `#[traced_test]`
- Mocked rate limiting tests for deterministic behavior
- ELink API integration tests covering all relationship types

#### Test Organization

- **Unit tests**: Located alongside source code in `pubmed-client/src/` modules
- **Integration tests**: Located in `pubmed-client/tests/integration/` directory
  - `comprehensive_pmc_tests.rs` - PMC XML parsing validation
  - `comprehensive_pubmed_tests.rs` - PubMed XML parsing validation
  - `test_elink_integration.rs` - ELink API functionality
  - `test_einfo_integration.rs` - EInfo API functionality
  - `markdown_tests.rs` - Markdown conversion testing
- **Test utilities**: Shared code in `pubmed-client/tests/integration/common/mod.rs`

### Dependencies

- `tokio` - Async runtime
- `tokio-util` - Utilities for tokio, used for rate limiting
- `reqwest` - HTTP client
- `serde` - Serialization/deserialization
- `quick-xml` - XML parsing for PMC content
- `thiserror` - Error type derivation
- `anyhow` - Error handling utilities
- `tracing` - Structured logging and instrumentation
- `urlencoding` - URL encoding for API parameters
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

#### Logging Guidelines

**DO NOT use `println!` or `eprintln!` in production code.** This project uses structured logging with the `tracing` crate for better observability and debugging.

**Allowed uses of `println!`:**

- Documentation examples in doc comments (`///` or `//!`)
- Code examples in README files
- Demo applications or example code

**Instead of print statements, use appropriate tracing macros:**

```rust
// ❌ AVOID - Don't use println! in library code
println!("Processing article {}", pmid);
println!("Found {} results", count);
eprintln!("Error: {}", error);

// ✅ PREFER - Use structured tracing
info!(pmid = %pmid, "Processing article");
info!(result_count = count, "Search completed");
warn!(error = %error, "Operation failed");
```

**Structured logging benefits:**

- Machine-readable logs for monitoring and analysis
- Consistent format across the entire codebase
- Better integration with observability tools
- Filterable and searchable log fields
- Performance benefits over string formatting

## Rate Limiting & NCBI API Compliance

This library implements automatic rate limiting to ensure compliance with NCBI E-utilities usage guidelines.

### NCBI Rate Limits

- **Without API key**: Maximum 3 requests per second
- **With API key**: Maximum 10 requests per second
- **Consequences**: Violations can result in IP blocking

### Configuration

#### Basic Usage (Default Rate Limiting)

```rust
use pubmed_client_rs::PubMedClient;

// Uses default rate limiting (3 req/sec, no API key)
let client = PubMedClient::new();
```

#### With API Key (Recommended for Production)

```rust
use pubmed_client_rs::{PubMedClient, ClientConfig};

let config = ClientConfig::new()
    .with_api_key("your_ncbi_api_key_here")
    .with_email("your.email@institution.edu")
    .with_tool("YourApplicationName");

let client = PubMedClient::with_config(config);
```

#### Custom Rate Limiting

```rust
use pubmed_client_rs::{PubMedClient, ClientConfig};

// Custom rate limit (e.g., for testing or special cases)
let config = ClientConfig::new()
    .with_rate_limit(5.0) // 5 requests per second
    .with_api_key("your_key");

let client = PubMedClient::with_config(config);
```

### How It Works

The library uses a **token bucket algorithm** for rate limiting:

1. **Token Bucket**: Each client has a bucket with a limited number of tokens
2. **Token Consumption**: Each API request consumes one token
3. **Token Refill**: Tokens are automatically refilled at the configured rate
4. **Automatic Waiting**: When no tokens are available, requests automatically wait

### Configuration Options

| Option       | Description                | Default                        |
| ------------ | -------------------------- | ------------------------------ |
| `api_key`    | NCBI E-utilities API key   | None                           |
| `rate_limit` | Custom requests per second | 3.0 (no key) / 10.0 (with key) |
| `email`      | Contact email for NCBI     | None                           |
| `tool`       | Application name for NCBI  | "pubmed-client-rs"             |
| `timeout`    | HTTP request timeout       | 30 seconds                     |
| `base_url`   | Custom NCBI base URL       | Default NCBI E-utilities       |

### Rate Limiting Examples

#### Sequential Requests (Automatically Rate Limited)

```rust
use pubmed_client_rs::PubMedClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PubMedClient::new();

    // These requests will be automatically rate limited to 3/second
    for pmid in ["31978945", "33515491", "32887691"] {
        let article = client.fetch_article(pmid).await?;
        println!("Title: {}", article.title);
    }

    Ok(())
}
```

#### Concurrent Requests (Shared Rate Limiting)

```rust
use pubmed_client_rs::{PubMedClient, ClientConfig};
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::new()
        .with_api_key("your_key")  // 10 req/sec with API key
        .with_email("you@example.com");

    let client = PubMedClient::with_config(config);
    let mut tasks = JoinSet::new();

    // Spawn concurrent tasks - rate limiting is automatically coordinated
    for pmid in ["31978945", "33515491", "32887691"] {
        let client = client.clone();
        let pmid = pmid.to_string();
        tasks.spawn(async move {
            client.fetch_article(&pmid).await
        });
    }

    // All tasks respect the shared rate limit
    while let Some(result) = tasks.join_next().await {
        match result? {
            Ok(article) => println!("Fetched: {}", article.title),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
```

### Getting an NCBI API Key

1. Visit [NCBI API Keys](https://ncbiinsights.ncbi.nlm.nih.gov/2017/11/02/new-api-keys-for-the-e-utilities/)
2. Create an NCBI account if you don't have one
3. Generate an API key in your account settings
4. Use the key in your application configuration

### Testing Rate Limiting

#### Mock Rate Limiting Tests (Default)

```bash
# Run mocked rate limiting tests (default, no network calls)
cargo test test_rate_limiting -- --nocapture

# Test with custom rate limits
RUST_LOG=debug cargo test test_rate_limiter_basic_functionality -- --nocapture

# Run all mocked rate limiting tests
cargo test --test test_rate_limiting_mocked -- --nocapture
```

#### Real API Rate Limiting Tests (Optional)

⚠️ **Warning**: These tests make actual network calls to NCBI APIs and are NOT run by default.

```bash
# Enable real API tests (requires internet connection)
export PUBMED_REAL_API_TESTS=1

# Run real API rate limiting tests
cargo test --test test_real_api_rate_limiting -- --nocapture

# Run with debug logging to see rate limiting behavior
RUST_LOG=debug cargo test --test test_real_api_rate_limiting -- --nocapture

# Test with API key (optional, set your NCBI API key)
export NCBI_API_KEY="your_api_key_here"
RUST_LOG=debug cargo test test_real_api_with_api_key -- --nocapture
```

**Real API Test Features:**

- Tests actual rate limiting with NCBI E-utilities
- Verifies 3 req/sec limit without API key, 10 req/sec with API key
- Tests concurrent request handling
- Validates end-to-end search and fetch workflows
- Tests server-side rate limit responses (429 errors)

**Real API Test Safety:**

- Conservative rate limits to avoid overwhelming NCBI servers
- Proper email and tool identification in requests
- Graceful handling of network errors and timeouts
- Respectful delays between requests

### Monitoring Rate Limits

Enable tracing to monitor rate limiting behavior:

```rust
use tracing_subscriber;

// Initialize logging
tracing_subscriber::fmt::init();

// Rate limiting events will be logged
let client = PubMedClient::new();
// Logs: "Need to wait for token", "Token acquired", etc.
```

## NCBI E-utilities API Support

The library implements multiple NCBI E-utilities APIs for comprehensive biomedical data access:

### ELink API for Article Relationships

1. **Related Articles** (`get_related_articles`) - Find articles related by subject/content
2. **PMC Links** (`get_pmc_links`) - Check PMC full-text availability
3. **Citations** (`get_citations`) - Find articles that cite the given articles

### EInfo API for Database Information

1. **Database List** (`get_database_list`) - Get all available NCBI databases
2. **Database Details** (`get_database_info`) - Get detailed information about specific databases including searchable fields and links

### Implementation Details

- **Rate Limited**: All ELink methods respect the same rate limiting as other API calls
- **Deduplication**: Results automatically remove duplicates and filter out source PMIDs
- **Error Handling**: Comprehensive error handling with tracing integration
- **Empty Input**: Graceful handling of empty PMID lists
- **Batch Processing**: Support for multiple PMIDs in a single request

### ELink Response Processing

The implementation uses internal response types (`ELinkResponse`, `ELinkSet`, `ELinkSetDb`) for JSON parsing, then converts to clean public API types (`RelatedArticles`, `PmcLinks`, `Citations`). This separation ensures API stability while handling NCBI's complex response format.

### Testing ELink Functionality

```bash
# Run ELink-specific integration tests (from pubmed-client/ directory)
cd pubmed-client && cargo test --test test_elink_integration

# Test individual ELink methods (from pubmed-client/ directory)
cd pubmed-client && cargo test test_get_related_articles_integration
cd pubmed-client && cargo test test_get_pmc_links_integration
cd pubmed-client && cargo test test_get_citations_integration
```

## WASM Development and Publishing

The workspace includes a complete npm package for WebAssembly bindings.

### WASM Package Publishing

The WASM package can be automatically published to npm:

```bash
# From pubmed-client-wasm/ directory
pnpm run publish    # Equivalent to: wasm-pack publish --access public
```

The workspace structure ensures:

- Correct package name (`pubmed-client-wasm`) generated automatically
- All npm dependencies and TypeScript configuration contained within the WASM package
- Independent versioning and publishing from the core Rust library

### WASM TypeScript Testing

Comprehensive TypeScript tests using Vitest:

```bash
# From pubmed-client-wasm/ directory
pnpm run test              # Run TypeScript tests
pnpm run test:watch        # Watch mode
pnpm run test:coverage     # With coverage
pnpm run typecheck         # Type checking only
```

**Test Coverage:**

- All WASM exported functions with success/failure scenarios
- Live API tests using real PMIDs (e.g., `31978945` for COVID-19 research)
- Error handling for invalid inputs and network failures
- Promise-based async API validation

## Python Bindings Development and Publishing

The workspace includes Python bindings built with PyO3 and maturin.

### Python Bindings Architecture (pubmed-client-py/)

The Python package provides native Python bindings for the core Rust library:

- `src/lib.rs` - PyO3 bindings entry point with Python-compatible types (1144 lines)
- `pubmed_client.pyi` - Type stub file for IDE autocomplete and mypy support
- `py.typed` - PEP 561 marker for type information
- `pyproject.toml` - Python package configuration (maturin build system)
- `pytest.ini` - Pytest configuration with test markers
- `tests/` - Python tests using pytest
- Configuration files: `pyproject.toml`, `.python-version`, `uv.lock`

**Key Python Types:**

- `Client` - Combined client with both `pubmed` and `pmc` properties
- `PubMedClient` - Python wrapper around Rust PubMed client
- `PmcClient` - Python wrapper around Rust PMC client
- `ClientConfig` - Configuration with builder pattern for method chaining
- `PubMedArticle`, `PmcFullText` - Python-friendly data structures
- `RelatedArticles`, `PmcLinks`, `Citations` - ELink API results
- `DatabaseInfo` - EInfo API database information

**Implementation Details:**

- **Blocking API**: Synchronous Python API using Tokio runtime internally
- **GIL Release**: Uses `py.allow_threads()` for non-blocking I/O operations
- **Runtime Management**: Creates Tokio runtime per call with `get_runtime()`
- **Error Handling**: Converts Rust errors to Python exceptions
- **Arc-based Sharing**: Thread-safe client cloning with `Arc<T>`
- **Builder Pattern**: ClientConfig supports method chaining
- **Recursive Collection**: Figures and tables collected from nested article sections

### Python Testing Strategy

Comprehensive test suite with 35 tests (100% passing):

**Test Organization:**

- `tests/test_config.py` - Configuration and builder pattern (9 tests)
- `tests/test_client.py` - Client initialization and properties (8 tests)
- `tests/test_models.py` - Data model validation (9 tests)
- `tests/test_integration.py` - Live API integration tests (10 tests)
- `tests/conftest.py` - Shared pytest fixtures and configuration

**Test Markers:**

- `@pytest.mark.integration` - Tests requiring network access to NCBI APIs
- `@pytest.mark.slow` - Long-running tests

**Test Fixtures:**

```python
# Available fixtures (from conftest.py)
@pytest.fixture
def pubmed_client() -> PubMedClient:
    """PubMed client with conservative rate limiting."""

@pytest.fixture
def pmc_client() -> PmcClient:
    """PMC client with conservative rate limiting."""

@pytest.fixture
def client() -> Client:
    """Combined client with conservative rate limiting."""
```

**Running Tests:**

```bash
# Unit tests only (fast, no network)
uv run pytest -m "not integration"

# Integration tests only (requires network)
uv run pytest -m integration

# All tests
uv run pytest

# With coverage report
uv run pytest --cov=pubmed_client --cov-report=html
```

### Type Stubs and IDE Support

The Python package includes comprehensive type annotations:

**Type Stub File (`pubmed_client.pyi`):**

- Complete type annotations for all 18 classes
- Full method signatures with parameter and return types
- Proper `Optional[T]` and `list[T]` type hints
- Docstrings for all public APIs

**Benefits:**

- Full IDE autocomplete (VS Code, PyCharm, etc.)
- mypy static type checking support
- Parameter hints and documentation
- Return type inference

**Type Checking:**

```bash
# Run mypy on test files
uv run mypy tests/ --strict

# Type check specific file
uv run mypy tests/test_integration.py
```

**Example Usage with Types:**

```python
import pubmed_client

# Type annotations work automatically
config: pubmed_client.ClientConfig = pubmed_client.ClientConfig()
config.with_api_key("key").with_email("user@example.com")

client: pubmed_client.Client = pubmed_client.Client.with_config(config)
articles: list[pubmed_client.PubMedArticle] = client.pubmed.search_and_fetch("covid-19", 10)
full_text: pubmed_client.PmcFullText = client.pmc.fetch_full_text("PMC7906746")
```

### Python Package Publishing

The Python package can be published to PyPI using maturin:

```bash
# From pubmed-client-py/ directory

# Build wheel for current platform
uv run --with maturin maturin build --release

# Build wheels for multiple platforms (requires CI)
uv run --with maturin maturin build --release --target universal2-apple-darwin  # macOS
uv run --with maturin maturin build --release --target x86_64-unknown-linux-gnu # Linux
uv run --with maturin maturin build --release --target x86_64-pc-windows-msvc   # Windows

# Publish to PyPI (requires PYPI_TOKEN)
uv run --with maturin maturin publish
```

**Package Configuration:**

- Package name: `pubmed-client`
- Module name: `pubmed_client`
- Includes type stubs automatically via maturin
- PEP 561 compliant (includes `py.typed`)
- Supports Python 3.12+

### Python Code Quality

The Python package uses ruff for linting and formatting:

```bash
# From pubmed-client-py/ directory

# Linting
uv run ruff check .                   # Check for issues
uv run ruff check --fix .             # Auto-fix issues

# Formatting
uv run ruff format .                  # Format code
uv run ruff format --check .          # Check formatting

# Type checking
uv run mypy tests/ --strict           # Strict type checking
```

**Ruff Configuration:**

- Line length: 100 characters
- Target version: Python 3.12
- Enabled rules: ALL (with selective ignores)
- Compatible with Black formatter
- Configured in `pyproject.toml`

### Python Dependencies

**Runtime Dependencies:**

- None (pure PyO3 extension module)

**Development Dependencies:**

- `pytest>=8.0` - Testing framework
- `pytest-cov>=6.0` - Coverage reporting
- `mypy>=1.0` - Static type checking
- `ruff>=0.7` - Linting and formatting

**Build Dependencies:**

- `maturin>=1.0,<2.0` - PyO3 build tool
- `uv` - Fast Python package manager (recommended)

### Python Binding Implementation Notes

**Key Design Decisions:**

1. **Synchronous API**: Used blocking API instead of async for simplicity
   - Creates Tokio runtime per call
   - Releases GIL during I/O with `py.allow_threads()`
   - Easier to use from Python

2. **Data Model Wrapping**: All Rust types wrapped in Python-friendly classes
   - `#[pyclass]` for data models
   - `#[pymethods]` for methods
   - Properties exposed with `#[pyo3(get)]`

3. **Error Handling**: Rust errors converted to Python exceptions
   - `to_py_err()` function converts `PubMedError` to `PyException`
   - Preserves error messages

4. **Builder Pattern**: ClientConfig uses method chaining
   - Returns `PyRefMut<Self>` to enable chaining
   - Follows Python conventions

5. **Recursive Collection**: Figures/tables collected from nested sections
   - PMC stores figures/tables in article sections
   - Recursive helper functions flatten the structure
   - Returns flat list for Python convenience

**PyO3 Features Used:**

- Extension modules with `#[pymodule]`
- Class definitions with `#[pyclass]`
- Method definitions with `#[pymethods]`
- Property access with `#[pyo3(get)]`
- GIL management with `py.allow_threads()`
- Type conversion with `From` trait implementations

## MCP Server for AI Assistants

The workspace includes a Model Context Protocol (MCP) server that allows AI assistants like Claude to interact with PubMed and PMC APIs.

### MCP Server Overview

The MCP server (`pubmed-mcp`) provides a standardized interface for AI assistants to:

- Search PubMed for biomedical literature
- Retrieve article metadata and abstracts
- Access full-text articles from PMC (future)

**Key Features:**

- Built with [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - Official Rust SDK for MCP
- Uses stdio transport for communication
- JSON-RPC 2.0 protocol implementation
- Automatic JSON schema generation for tool parameters
- Integration with Claude Desktop and other MCP clients

### MCP Server Architecture (pubmed-mcp/)

```
pubmed-mcp/
├── Cargo.toml              # Package configuration
├── src/
│   └── main.rs             # MCP server implementation (119 lines)
├── tests/
│   └── integration_test.rs # MCP protocol integration tests
└── README.md               # MCP-specific documentation
```

**Implementation Details:**

- `PubMedServer` struct - Main server implementation with tool router
- Tool definitions using `#[tool]` macro from rmcp
- Automatic parameter validation with JSON schema
- Structured logging to stderr (doesn't interfere with stdio protocol)
- Protocol version: `V_2024_11_05`

### Available MCP Tools

#### `search_pubmed`

Search PubMed for articles matching a query.

**Parameters:**

- `query` (string, required): Search query using PubMed syntax
  - Examples: `"COVID-19"`, `"cancer[ti] AND therapy[tiab]"`
  - Supports all [PubMed field tags](https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags)
- `max_results` (integer, optional): Maximum number of results (default: 10, max: 100)

**Returns:**

- Formatted text with article titles and PMIDs
- Article count and numbered list

**Example Usage (from Claude Desktop):**

```
User: Search for recent COVID-19 vaccine research (max 5 results)

Claude uses tool: search_pubmed
  query: "COVID-19 vaccine[ti]"
  max_results: 5

Result: Found 5 articles:
1. COVID-19 vaccine effectiveness... (PMID: 12345678)
2. mRNA vaccines for COVID-19... (PMID: 23456789)
...
```

### Claude Desktop Integration

To use the MCP server with Claude Desktop, add it to your configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pubmed": {
      "command": "/path/to/pubmed-client-rs/target/release/pubmed-mcp"
    }
  }
}
```

After restarting Claude Desktop, the server will be available with the search_pubmed tool.

### MCP Server Development

#### Adding New Tools

To add additional MCP tools, add methods to the `PubMedServer` impl block:

```rust
#[tool(description = "Fetch full-text article from PMC")]
async fn fetch_pmc_article(
    &self,
    Parameters(params): Parameters<PmcRequest>,
) -> Result<CallToolResult, ErrorData> {
    // Implementation using self.client.pmc
    let article = self.client.pmc.fetch_full_text(&params.pmc_id).await
        .map_err(|e| ErrorData { /* error conversion */ })?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
```

The `#[tool]` macro automatically:

- Registers the tool with the MCP server
- Generates JSON schema from parameter types
- Handles tool routing and parameter validation

#### MCP Testing

The MCP server includes comprehensive integration tests:

```bash
# Run MCP protocol tests
cargo test -p pubmed-mcp

# Test server initialization
cargo test -p pubmed-mcp test_mcp_server_initialize

# Test tool listing
cargo test -p pubmed-mcp test_mcp_server_list_tools

# Test server capabilities
cargo test -p pubmed-mcp test_mcp_server_capabilities
```

**Test Coverage:**

- Server initialization and peer info verification
- Tool registration and listing
- Server capabilities and protocol version
- Tool execution with real PubMed API (requires network)

#### MCP Inspector for Interactive Testing

The [MCP Inspector](https://github.com/modelcontextprotocol/inspector) provides an interactive UI for testing MCP servers:

```bash
# Launch MCP Inspector with the server
npx @modelcontextprotocol/inspector cargo run -p pubmed-mcp

# Inspector opens in browser, allowing:
# - Interactive tool testing
# - Parameter input with schema validation
# - Response inspection
# - Server capability exploration
```

### MCP Dependencies

- **rmcp**: Official Rust SDK for Model Context Protocol (v0.8)
  - Features: `server`, `transport-io` (runtime), `client`, `transport-child-process` (tests)
- **schemars**: JSON schema generation for tool parameters (v0.8)
- **tokio**: Async runtime with full feature set
- **tracing-subscriber**: Structured logging to stderr
- **pubmed-client**: Core library for PubMed/PMC API access

### MCP Protocol Details

**Transport**: stdio (stdin/stdout)

- Server reads JSON-RPC requests from stdin
- Server writes JSON-RPC responses to stdout
- Logging goes to stderr to avoid protocol interference

**Protocol Version**: `V_2024_11_05`

**Capabilities**:

- `tools`: Server provides searchable tools
- Tool listing with `tools/list`
- Tool execution with `tools/call`

**Message Format**: JSON-RPC 2.0

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "search_pubmed",
    "arguments": {
      "query": "COVID-19",
      "max_results": 10
    }
  }
}
```

## Test Fixtures and Data

### Downloading Test Data

The project includes scripts to download real XML responses for comprehensive integration testing:

```bash
# Download PMC XML test fixtures (already included)
./scripts/download_all_verified_xml.sh

# Download PubMed XML test fixtures
./scripts/download_pubmed_xml.sh
```

### Test Data Structure

```
tests/integration/test_data/
├── pmc_xml/         # PMC full-text XML files (18 files)
│   ├── PMC10000000.xml
│   ├── PMC10618641.xml
│   └── ...
└── pubmed_xml/      # PubMed article XML files (15 files)
    ├── 25760099.xml  # CRISPR-Cas9 research
    ├── 31978945.xml  # COVID-19 research
    ├── 33515491.xml  # Cancer treatment
    └── ...
```

### Comprehensive Integration Tests

The test suites provide extensive coverage of XML parsing and content analysis:

- **PMC Tests**: `cargo test --test comprehensive_pmc_tests`
  - XML parsing validation
  - Content structure analysis
  - Author and metadata extraction
  - Figures and tables processing

- **PubMed Tests**: `cargo test --test comprehensive_pubmed_tests`
  - Article metadata validation
  - MeSH term extraction and analysis
  - Abstract content analysis
  - Author details and affiliations
  - Chemical substances parsing
  - Article type distribution

Both test suites include statistical analysis and success rate validation to ensure robust parsing across diverse article types and content structures.

## GitHub Actions CI/CD

The project uses comprehensive GitHub Actions workflows for continuous integration and deployment.

### Workflows

#### Test Workflow (`.github/workflows/test.yml`)

**Jobs:**

- **Lint and Format**: Code quality checks (rustfmt, dprint, clippy, docs)
- **Test Suite**: Cross-platform testing (Ubuntu, Windows, macOS) with Rust stable/beta
- **Code Coverage**: Coverage reporting with Codecov integration

**Key Features:**

- Git LFS support for test fixtures
- Excludes real API tests from regular CI (network-dependent)
- Matrix testing with reduced beta combinations for efficiency
- Comprehensive integration test coverage

#### Documentation Workflow (`.github/workflows/docs.yml`)

**Jobs:**

- **Build Documentation**: Generate rustdoc with all features
- **Deploy Documentation**: Auto-deploy to GitHub Pages (main branch only)

**Features:**

- Documentation warnings as errors (`RUSTDOCFLAGS`)
- GitHub Pages integration
- Artifact upload for PR previews

#### Release Workflow (`.github/workflows/release.yml`)

**Jobs:**

- **Create Release**: GitHub release creation on version tags
- **Test Before Release**: Full test suite validation
- **Publish to crates.io**: Automated publishing using reusable workflow (stable releases only)
  - **publish-core**: Publishes `pubmed-client` (always runs for non-prerelease tags)
  - **publish-cli**: Publishes `pubmed-cli` (only if tag contains 'cli')
  - **publish-mcp**: Publishes `pubmed-mcp` (only if tag contains 'mcp')

**Features:**

- Automatic prerelease detection (alpha/beta/rc tags)
- Package validation before publishing
- Secure token-based crates.io publishing
- Uses reusable workflow for consistent publishing logic
- Dependency ordering (CLI and MCP depend on core library)
- Tag-based selective publishing

#### Reusable Publish Workflow (`.github/workflows/publish-crates.yml`)

A reusable workflow for publishing Rust packages to crates.io. This workflow can be called from other workflows to standardize the publishing process.

**Inputs:**

- `package` (required): Package name to publish (e.g., `pubmed-client`, `pubmed-cli`)
- `package-path` (optional): Path to the package directory (defaults to package name)
- `dry-run` (optional): Perform a dry run without actually publishing (default: `false`)
- `check-version` (optional): Check if Cargo.toml version matches git tag (default: `true`)

**Secrets:**

- `CRATES_IO_TOKEN` (required): Token for publishing to crates.io

**Features:**

- Automatic version validation against git tags
- Package validation with `cargo package`
- Support for dry-run mode for testing
- Automatic generation of publish summary with crates.io link
- Flexible package path configuration
- Proper dependency caching with package-specific keys

**Example Usage:**

```yaml
jobs:
  publish-package:
    uses: ./.github/workflows/publish-crates.yml
    with:
      package: pubmed-client
      package-path: pubmed-client
      check-version: true
      dry-run: false
    secrets:
      CRATES_IO_TOKEN: ${{ secrets.CRATES_IO_TOKEN }}
```

**Tag Naming Conventions:**

- Core library: `v1.0.0` (will publish `pubmed-client`)
- CLI: `v1.0.0` or `cli-v1.0.0` (tag must contain 'cli' to publish `pubmed-cli`)
- MCP: `v1.0.0` or `mcp-v1.0.0` (tag must contain 'mcp' to publish `pubmed-mcp`)
- Prerelease: `v1.0.0-alpha`, `v1.0.0-beta`, `v1.0.0-rc1` (creates GitHub release but skips crates.io)

**Testing Releases with Workflow Dispatch:**

The release workflow can be triggered manually for testing purposes:

```bash
# Trigger from GitHub UI: Actions -> Release -> Run workflow
# Or use GitHub CLI:
gh workflow run release.yml
```

**Workflow Dispatch Behavior:**

When manually triggering the workflow via `workflow_dispatch`:

- **Always runs in dry-run mode**: Validates packages but does NOT publish to crates.io
- **Tests ALL packages**: Always validates all packages (core, CLI, and MCP)
- **No inputs required**: Simply trigger the workflow - no configuration needed

This ensures safe testing of the release workflow without any risk of accidental publishing.

**Example:**

```bash
# Test publishing all packages (always dry-run)
gh workflow run release.yml
```

**Important:** To actually publish to crates.io, you must create and push a version tag. workflow_dispatch is exclusively for testing and validation.

**Key Differences from Tag-based Releases:**

When triggered via workflow_dispatch (vs. pushing a tag):

- **No GitHub release created**: Skips GitHub release creation step
- **No version checking**: `check-version` is disabled (no git tag to compare against)
- **All packages tested**: Always validates all packages (core, CLI, MCP) regardless of tag naming
- **Always dry-run**: NEVER publishes to crates.io - validation only
- **Test suite always runs**: Validates build and tests before attempting dry-run publish

This allows you to:

- Test the complete release workflow without any risk of accidental publishing
- Validate all package configurations before creating tags
- Debug workflow issues in isolation
- Verify version compatibility and dependencies across all packages
- Ensure all workspace members can be successfully published together

**To actually publish:** Create and push a version tag (e.g., `v0.1.0`). Only tag-based releases will publish to crates.io.

### Git LFS Configuration

Large test data files are managed with Git LFS:

```gitattributes
# Track all test data files with Git LFS
tests/integration/test_data/**/*.xml filter=lfs diff=lfs merge=lfs -text
tests/integration/test_data/**/*.json filter=lfs diff=lfs merge=lfs -text
```

**Benefits:**

- Keeps repository lightweight
- Efficient CI/CD with large fixtures
- Proper version control for binary-like test data

### Running Real API Tests in CI

Real API tests are opt-in to avoid overwhelming NCBI servers:

```bash
# Automatically runs on main branch pushes
git push origin main

# Or add label "test-real-api" to PR
gh pr edit --add-label "test-real-api"
```

### Coverage Integration

Code coverage is automatically generated and uploaded to Codecov:

- Excludes real API tests from coverage (network-dependent)
- Requires `CODECOV_TOKEN` secret in repository settings
- Generates LCOV format for broad tool compatibility
