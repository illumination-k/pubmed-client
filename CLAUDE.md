# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust client library for accessing PubMed and PMC (PubMed Central) APIs. It provides async interfaces for searching biomedical research articles and fetching full-text content.

## Commands

### Build & Development

Using `mise` for task management (configured in `.mise.toml`):

```bash
# Build the project
mise r build

# Run tests with nextest (preferred test runner)
mise r test

# Run tests in watch mode
mise r test:watch

# Run tests with verbose output
mise r test:verbose

# Generate and open documentation
mise r doc

# Check code without building
mise r check

# Run specific integration test suite
cargo test --test comprehensive_pmc_tests
cargo test --test comprehensive_pubmed_tests
cargo test --test test_elink_integration

# Run new real API integration tests (requires network and env var)
PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pubmed_api_tests
PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pmc_api_tests
PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test error_handling_tests

# Run single unit test
cargo test --lib pubmed::parser::tests::test_mesh_term_parsing
```

### Code Quality

```bash
# Full linting (dprint + cargo fmt + clippy)
mise r lint

# Format code (dprint + cargo fmt)
mise r fmt
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

### Module Structure

- `src/lib.rs` - Main library entry point, re-exports public API and unified `Client`
- `src/pubmed/` - PubMed module directory
  - `client.rs` - PubMed API client for article metadata search and ELink API
  - `parser.rs` - XML parsing logic for PubMed article metadata
  - `models.rs` - Data structures for PubMed articles, ELink results
  - `responses.rs` - Internal API response structures for JSON/XML parsing
  - `mod.rs` - Module exports and public API
- `src/pmc/` - PMC module directory
  - `client.rs` - PMC client for full-text article access
  - `parser.rs` - XML parsing logic for PMC content
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
- Separation of metadata (PubMed) and full-text (PMC) concerns
- ELink API integration for discovering article relationships (related articles, citations, PMC links)
- Support for custom HTTP clients via reqwest
- Internal response types separate from public API models
- Data-driven testing with real PMC XML samples in `tests/test_data/`
- Structured logging with tracing for debugging and monitoring
- Rate limiting with token bucket algorithm for NCBI API compliance
- Configurable API keys, email, and tool identification for NCBI guidelines

### Testing

- Test runner: `cargo-nextest` for better output and parallelization
- Parameterized tests using `rstest`
- Test data: Real XML files in `tests/integration/test_data/`
- Common test utilities in `tests/integration/common/mod.rs`
- Integration tests with tracing support using `#[traced_test]`
- Mocked rate limiting tests for deterministic behavior
- ELink API integration tests covering all relationship types

#### Test Organization

- **Unit tests**: Located alongside source code in `src/` modules
- **Integration tests**: Located in `tests/integration/` directory
  - `comprehensive_pmc_tests.rs` - PMC XML parsing validation
  - `comprehensive_pubmed_tests.rs` - PubMed XML parsing validation
  - `test_elink_integration.rs` - ELink API functionality
  - `test_einfo_integration.rs` - EInfo API functionality
  - `markdown_tests.rs` - Markdown conversion testing
- **Test utilities**: Shared code in `tests/integration/common/mod.rs`

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
# Run ELink-specific integration tests
cargo test --test test_elink_integration

# Test individual ELink methods
cargo test test_get_related_articles_integration
cargo test test_get_pmc_links_integration
cargo test test_get_citations_integration
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
- **Publish to crates.io**: Automated publishing (stable releases only)

**Features:**

- Automatic prerelease detection (alpha/beta/rc tags)
- Package validation before publishing
- Secure token-based crates.io publishing

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
