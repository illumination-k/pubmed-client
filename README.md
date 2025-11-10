# PubMed Client for Rust

A comprehensive Rust workspace for accessing PubMed and PMC (PubMed Central) APIs, with multiple language bindings and tools.

## Features

- **PubMed API Integration**: Search and fetch article metadata using E-utilities
- **PMC Full Text**: Retrieve and parse structured full-text articles
- **ELink API**: Discover related articles, citations, and PMC full-text availability
- **EInfo API**: Query database information and searchable fields
- **Advanced Search Builder**: Construct complex queries with filters, date ranges, and boolean logic
- **MeSH Term Support**: Extract and search using Medical Subject Headings (MeSH) vocabulary
- **Markdown Export**: Convert PMC articles to well-formatted Markdown
- **Async Support**: Built on tokio for async/await support
- **Type Safety**: Strongly typed data structures for all API responses
- **Automatic Retry**: Built-in retry logic with exponential backoff for handling transient failures
- **Rate Limiting**: Automatic compliance with NCBI API rate limits (3 req/sec default, 10 req/sec with API key)
- **Multiple Language Bindings**: Rust, JavaScript/TypeScript (WASM), and Python (PyO3)
- **Command-Line Interface**: CLI for common operations (search, conversion, figure extraction)
- **MCP Server**: Model Context Protocol server for AI assistant integration

## Workspace Structure

This is a Cargo workspace containing multiple packages:

- **[pubmed-client](pubmed-client/)** - Core Rust library
- **[pubmed-client-wasm](pubmed-client-wasm/)** - WebAssembly bindings for npm
- **[pubmed-client-py](pubmed-client-py/)** - Python bindings via PyO3
- **[pubmed-cli](pubmed-cli/)** - Command-line interface
- **[pubmed-mcp](pubmed-mcp/)** - MCP server for AI assistants

## Installation

### Rust

Add this to your `Cargo.toml`:

```toml
[dependencies]
pubmed-client-rs = "0.1.0"
```

### JavaScript/TypeScript (WASM)

```bash
npm install pubmed-client-wasm
# or
pnpm add pubmed-client-wasm
```

### Python

```bash
pip install pubmed-client-py
# or
uv add pubmed-client-py
```

### Command-Line Interface

```bash
cargo install pubmed-cli
```

## Quick Start

### Rust

#### Using the Unified Client

```rust
use pubmed_client_rs::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a unified client for both PubMed and PMC
    let client = Client::new();

    // Search PubMed for articles
    let articles = client.pubmed
        .search()
        .query("covid-19 treatment")
        .open_access_only()
        .published_after(2020)
        .limit(10)
        .search_and_fetch(&client.pubmed)
        .await?;

    for article in &articles {
        println!("Title: {}", article.title);
        println!("PMID: {}", article.pmid);
    }

    // Fetch PMC full text for the first article
    if let Some(article) = articles.first() {
        if let Ok(full_text) = client.pmc.fetch_full_text(&article.pmid).await {
            println!("\nFull text sections: {}", full_text.sections.len());
        }
    }

    Ok(())
}
```

#### Advanced Search with Filters

```rust
use pubmed_client_rs::{Client, pubmed::SearchQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // Complex search with multiple filters
    let articles = SearchQuery::new()
        .title("CRISPR")
        .author("Doudna")
        .journal("Nature")
        .published_between(2020, 2024)
        .clinical_trials_only()
        .limit(5)
        .search_and_fetch(&client.pubmed)
        .await?;

    Ok(())
}
```

#### Discovering Related Articles

```rust
use pubmed_client_rs::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // Find related articles using ELink API
    let related = client.pubmed.get_related_articles(&["31978945"]).await?;
    println!("Found {} related articles", related.related_pmids.len());

    // Check PMC full-text availability
    let pmc_links = client.pubmed.get_pmc_links(&["31978945"]).await?;
    if let Some(pmcid) = pmc_links.pmid_to_pmcid.get("31978945") {
        println!("PMC full-text available: {}", pmcid);
    }

    // Find citations
    let citations = client.pubmed.get_citations(&["31978945"]).await?;
    println!("Cited by {} articles", citations.citing_pmids.len());

    Ok(())
}
```

### JavaScript/TypeScript (WASM)

```typescript
import { WasmPubMedClient, WasmClientConfig } from 'pubmed-client-wasm';

async function main() {
    // Create client with configuration
    const config = new WasmClientConfig();
    config.with_api_key("your_api_key");
    config.with_email("you@example.com");

    const client = WasmPubMedClient.with_config(config);

    // Search for articles
    const articles = await client.search_and_fetch("covid-19", 10);

    for (const article of articles) {
        console.log(`Title: ${article.title}`);
        console.log(`PMID: ${article.pmid}`);
    }
}

main().catch(console.error);
```

### Python

```python
import pubmed_client

# Create a unified client
client = pubmed_client.Client()

# Search PubMed
articles = client.pubmed.search_and_fetch("covid-19 treatment", max_results=10)

for article in articles:
    print(f"Title: {article.title}")
    print(f"PMID: {article.pmid}")

# Fetch PMC full text
full_text = client.pmc.fetch_full_text("PMC7906746")
print(f"Sections: {len(full_text.sections)}")
print(f"References: {len(full_text.references)}")

# Find related articles
related = client.pubmed.get_related_articles(["31978945"])
print(f"Found {len(related.related_pmids)} related articles")
```

### Command-Line Interface

```bash
# Search PubMed
pubmed-cli search "covid-19" --max-results 10

# Convert PMID to PMCID
pubmed-cli pmid-to-pmcid 31978945 33515491

# Extract figures from PMC articles
pubmed-cli figures PMC7906746 --output figures/

# Convert PMC article to Markdown
pubmed-cli markdown PMC7906746 > article.md
```

## Advanced Features

### Converting PMC Articles to Markdown

```rust
use pubmed_client_rs::{Client, pmc::PmcMarkdownConverter, pmc::HeadingStyle, pmc::ReferenceStyle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // Fetch and parse a PMC article
    let full_text = client.pmc.fetch_full_text("PMC7906746").await?;

    // Create a markdown converter with custom configuration
    let converter = PmcMarkdownConverter::new()
        .with_include_metadata(true)
        .with_include_toc(true)
        .with_heading_style(HeadingStyle::ATX)
        .with_reference_style(ReferenceStyle::Numbered);

    // Convert to markdown
    let markdown = converter.convert(&full_text);

    // Save to file
    std::fs::write("article.md", markdown)?;

    Ok(())
}
```

### Working with MeSH Terms

Medical Subject Headings (MeSH) terms provide standardized vocabulary for biomedical literature. This library supports extracting and searching with MeSH terms.

#### Searching with MeSH Terms

```rust
use pubmed_client_rs::{Client, pubmed::SearchQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // Search using MeSH major topics
    let diabetes_articles = SearchQuery::new()
        .mesh_major_topic("Diabetes Mellitus, Type 2")
        .mesh_subheading("drug therapy")
        .published_after(2020)
        .limit(10)
        .search_and_fetch(&client.pubmed)
        .await?;

    // Search with multiple MeSH terms
    let cancer_research = SearchQuery::new()
        .mesh_terms(&["Neoplasms", "Antineoplastic Agents"])
        .clinical_trials_only()
        .limit(5)
        .search_and_fetch(&client.pubmed)
        .await?;

    Ok(())
}
```

#### Extracting MeSH Information

```rust
use pubmed_client_rs::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let article = client.pubmed.fetch_article("31978945").await?;

    // Get major MeSH terms
    let major_terms = article.get_major_mesh_terms();
    println!("Major MeSH topics: {:?}", major_terms);

    // Check for specific MeSH term
    if article.has_mesh_term("COVID-19") {
        println!("This article is about COVID-19");
    }

    // Get all MeSH terms
    let all_terms = article.get_all_mesh_terms();
    println!("All MeSH terms: {:?}", all_terms);

    // Get MeSH qualifiers for a specific term
    let qualifiers = article.get_mesh_qualifiers("COVID-19");
    println!("COVID-19 qualifiers: {:?}", qualifiers);

    // Get chemical substances
    let chemicals = article.get_chemical_names();
    println!("Chemicals mentioned: {:?}", chemicals);

    Ok(())
}
```

#### Comparing Articles by MeSH Terms

```rust
use pubmed_client_rs::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let article1 = client.pubmed.fetch_article("31978945").await?;
    let article2 = client.pubmed.fetch_article("33515491").await?;

    // Calculate MeSH term similarity (Jaccard similarity)
    let similarity = article1.mesh_term_similarity(&article2);
    println!("MeSH similarity: {:.2}%", similarity * 100.0);

    Ok(())
}
```

### MCP Server for AI Assistants

The Model Context Protocol (MCP) server enables AI assistants like Claude to interact with PubMed APIs.

#### Claude Desktop Integration

Add to your Claude Desktop configuration:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pubmed": {
      "command": "/path/to/pubmed-mcp"
    }
  }
}
```

After restarting Claude Desktop, you can ask Claude to search PubMed:

```
User: Search PubMed for recent COVID-19 vaccine research

Claude: [Uses the search_pubmed tool automatically]
Found 10 articles about COVID-19 vaccines:
1. mRNA vaccine effectiveness... (PMID: 12345678)
2. Booster dose efficacy... (PMID: 23456789)
...
```

#### Available MCP Tools

- **search_pubmed**: Search PubMed with query string and result limit
- Support for all PubMed field tags ([ti], [au], [ta], etc.)
- Future: PMC full-text retrieval, citation networks

For more details, see the [MCP server README](pubmed-mcp/README.md).

## Configuration

### Rate Limiting and API Keys

The client automatically handles rate limiting according to NCBI guidelines:

```rust
use pubmed_client_rs::{Client, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Default: 3 requests per second without API key
    let client = Client::new();

    // With API key: 10 requests per second
    let config = ClientConfig::new()
        .with_api_key("your_ncbi_api_key")
        .with_email("your@email.com")
        .with_tool("YourAppName");

    let client = Client::with_config(config);

    Ok(())
}
```

### Retry Configuration

The client includes automatic retry logic with exponential backoff for handling transient failures:

```rust
use pubmed_client_rs::{Client, ClientConfig, retry::RetryConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Custom retry configuration
    let retry_config = RetryConfig::new()
        .with_max_retries(5)                            // Maximum 5 retry attempts
        .with_initial_delay(Duration::from_secs(2))     // Start with 2 second delay
        .with_max_delay(Duration::from_secs(60))        // Cap at 60 seconds
        .without_jitter();                              // Disable jitter for predictable delays

    let config = ClientConfig::new()
        .with_api_key("your_api_key")
        .with_retry_config(retry_config);

    let client = Client::with_config(config);

    // The client will automatically retry on:
    // - Network timeouts
    // - HTTP 5xx server errors
    // - HTTP 429 rate limit responses
    // - Connection failures

    Ok(())
}
```

## Development

### Prerequisites

- Rust 1.70 or later
- [mise](https://mise.jdx.dev/) (optional, for tool management)
- Node.js and pnpm (for WASM development)
- Python 3.12+ and uv (for Python bindings)

### Setup

Clone the repository:

```bash
git clone https://github.com/illumination-k/pubmed-client-rs.git
cd pubmed-client-rs
```

If using mise, install tools:

```bash
mise install
```

### Workspace Commands

```bash
# Build all workspace members
cargo build

# Test all workspace members
cargo test

# Build specific package
cargo build -p pubmed-client-rs     # Core library
cargo build -p pubmed-client-wasm   # WASM bindings
cargo build -p pubmed-client-py     # Python bindings
cargo build -p pubmed-cli           # CLI
cargo build -p pubmed-mcp    # MCP server
```

### Running Tests

This project uses [nextest](https://nexte.st/) for Rust tests:

```bash
# Run all Rust tests
cargo nextest run

# Run tests for specific package
cargo nextest run -p pubmed-client-rs
cargo nextest run -p pubmed-mcp

# Run specific integration test suite
cd pubmed-client && cargo test --test comprehensive_pubmed_tests
cd pubmed-client && cargo test --test comprehensive_pmc_tests

# Using mise tasks
mise run test
mise run test:verbose

# WASM tests (TypeScript)
cd pubmed-client-wasm && pnpm run test

# Python tests
cd pubmed-client-py && uv run pytest
cd pubmed-client-py && uv run pytest -m "not integration"  # Skip network tests
```

### Code Quality

#### Rust Code Quality

```bash
# Format code (dprint + cargo fmt)
cargo fmt
mise run fmt

# Run linter
cargo clippy
mise run lint

# Check code
cargo check
mise run check
```

#### WASM/TypeScript Code Quality

```bash
# From pubmed-client-wasm/
pnpm run format     # Format TypeScript
pnpm run lint       # Lint TypeScript
pnpm run check      # Format + lint
```

#### Python Code Quality

```bash
# From pubmed-client-py/
uv run ruff check .        # Linting
uv run ruff format .       # Formatting
uv run mypy tests/         # Type checking
```

### Code Coverage

Generate and view test coverage reports:

```bash
# Generate HTML coverage report
cargo llvm-cov nextest --all-features --html
mise run coverage

# Generate and open HTML report
cargo llvm-cov nextest --all-features --html --open
mise run coverage:open

# Generate LCOV format for CI
cargo llvm-cov nextest --all-features --lcov --output-path coverage.lcov
mise run coverage:lcov

# Generate JSON format
cargo llvm-cov nextest --all-features --json --output-path coverage.json
mise run coverage:json
```

### Documentation

Generate and view Rust documentation:

```bash
cargo doc --open
mise run doc
```

Online documentation is available at:

- **Core library**: [docs.rs/pubmed-client-rs](https://docs.rs/pubmed-client-rs)
- **WASM package**: [npm package](https://www.npmjs.com/package/pubmed-client-wasm)
- **Python package**: [PyPI](https://pypi.org/project/pubmed-client)

## API Coverage

This library provides comprehensive access to NCBI E-utilities:

- **ESearch**: Search PubMed with advanced query builder
- **EFetch**: Retrieve article metadata and PMC full-text
- **ELink**: Discover related articles, citations, and PMC links
- **EInfo**: Query database information and searchable fields
- **PMC OAI**: Access PMC full-text articles in structured XML format

See [CLAUDE.md](CLAUDE.md) for implementation details and API design patterns.

## Examples

The repository includes comprehensive examples:

- **[Rust examples](pubmed-client/examples/)** - Core library usage
- **[WASM examples](pubmed-client-wasm/tests/)** - TypeScript integration
- **[Python examples](pubmed-client-py/examples/)** - Python usage patterns
- **[CLI usage](pubmed-cli/README.md)** - Command-line examples

## CI/CD and GitHub Actions

The project uses comprehensive GitHub Actions workflows:

- **Test**: Cross-platform testing (Ubuntu, Windows, macOS) with coverage
- **Docs**: Auto-deploy documentation to GitHub Pages
- **Release**: Automated publishing to crates.io, npm, and PyPI

See [.github/workflows/](.github/workflows/) for workflow details.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

When contributing, please:

- Follow the existing code style and patterns
- Add tests for new features
- Update documentation as needed
- Ensure all CI checks pass

## Related Projects

There are several excellent PubMed and NCBI E-utilities client libraries available in different programming languages:

### Python

- **[biocommons/eutils](https://github.com/biocommons/eutils)** - A comprehensive Python package for simplified searching, fetching, and parsing records from NCBI using their E-utilities interface. Features automatic request throttling (3/10 req/sec without/with API key) and promise-based piping.

- **[gijswobben/pymed](https://github.com/gijswobben/pymed)** - PyMed provides access to PubMed through the PubMed API in a consistent, readable and performant way. Available on [PyPI](https://pypi.org/project/pymed/).

- **[metapub/metapub](https://github.com/metapub/metapub)** - Python toolkit for NCBI metadata (via eutils) and PubMed article text mining. Provides abstraction layers over Medgen, PubMed, ClinVar, and CrossRef for building scholarly paper libraries.

- **[krassowski/easy-entrez](https://github.com/krassowski/easy-entrez)** - Retrieve PubMed articles, text-mining annotations, or molecular data from >35 Entrez databases via easy-to-use Python package built on top of Entrez E-utilities API.

### JavaScript/TypeScript

- **[linjoey/ncbi-eutils](https://github.com/linjoey/ncbi-eutils)** - NCBI E-utilities client for Node.js and the browser. Uses ES6 promises to support "piping" to combine successive E-utility calls (e.g., piping esearch results to elink, then to esummary).

- **[node-ncbi](https://www.npmjs.com/package/node-ncbi)** - A Node.js wrapper for the NCBI eUtils API that allows searching PubMed or other databases with results returned as JavaScript objects.

### Go

- **[biogo/ncbi](https://github.com/biogo/ncbi)** - Package entrez provides support for interaction with the NCBI Entrez Utility Programs (E-utilities). Part of the biogo bioinformatics library collection.

### PHP

- **[jorgetutor/ncbi](https://github.com/jorgetutor/ncbi)** - Provides an implementation of the Guzzle library to query NCBI E-Utils service.

### Rust

- **[pubmed crate](https://crates.io/crates/pubmed)** - A Rust crate that implements reading publications from the PubMed API.

All of these libraries interact with the [NCBI E-utilities](https://www.ncbi.nlm.nih.gov/books/NBK25501/), which provide the public API to the NCBI Entrez system for accessing PubMed, PMC, Gene, and other databases.

## Acknowledgments

This project relies on NCBI's excellent E-utilities API. Please cite NCBI appropriately when using this library in research:

> National Center for Biotechnology Information (NCBI). E-utilities API. https://www.ncbi.nlm.nih.gov/books/NBK25501/
