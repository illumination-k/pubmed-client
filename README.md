# PubMed Client for Rust

A Rust client library for accessing PubMed and PMC (PubMed Central) APIs.

## Features

- **PubMed API Integration**: Search and fetch article metadata
- **PMC Full Text**: Retrieve and parse structured full-text articles
- **MeSH Term Support**: Extract and search using Medical Subject Headings (MeSH) vocabulary
- **Markdown Export**: Convert PMC articles to well-formatted Markdown
- **Async Support**: Built on tokio for async/await support
- **Type Safety**: Strongly typed data structures for all API responses

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
pubmed-client-rs = "0.1.0"
```

## Quick Start

### Searching PubMed

```rust
use pubmed_client_rs::PubMedClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PubMedClient::new();

    // Search for articles
    let articles = client
        .search()
        .query("covid-19 treatment")
        .open_access_only()
        .published_after(2020)
        .limit(10)
        .search_and_fetch(&client)
        .await?;

    for article in articles {
        println!("Title: {}", article.title);
        println!("Authors: {}", article.authors.join(", "));
    }

    Ok(())
}
```

### Fetching PMC Full Text

```rust
use pubmed_client_rs::PmcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PmcClient::new();

    // Check if PMC full text is available
    if let Some(pmcid) = client.check_pmc_availability("33515491").await? {
        // Fetch structured full text
        let full_text = client.fetch_full_text(&pmcid).await?;

        println!("Title: {}", full_text.title);
        println!("Sections: {}", full_text.sections.len());
        println!("References: {}", full_text.references.len());
    }

    Ok(())
}
```

### Converting to Markdown

```rust
use pubmed_client_rs::{PmcClient, PmcMarkdownConverter, HeadingStyle, ReferenceStyle};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PmcClient::new();

    // Fetch and parse a PMC article
    if let Ok(full_text) = client.fetch_full_text("PMC1234567").await {
        // Create a markdown converter with custom configuration
        let converter = PmcMarkdownConverter::new()
            .with_include_metadata(true)
            .with_include_toc(true)
            .with_heading_style(HeadingStyle::ATX)
            .with_reference_style(ReferenceStyle::Numbered);

        // Convert to markdown
        let markdown = converter.convert(&full_text);
        println!("{}", markdown);

        // Or save to file
        std::fs::write("article.md", markdown)?;
    }

    Ok(())
}
```

### Working with MeSH Terms

Medical Subject Headings (MeSH) terms provide standardized vocabulary for biomedical literature. This library supports extracting and searching with MeSH terms.

#### Searching with MeSH Terms

```rust
use pubmed_client_rs::{PubMedClient, pubmed::SearchQuery};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PubMedClient::new();

    // Search using MeSH major topics
    let diabetes_articles = SearchQuery::new()
        .mesh_major_topic("Diabetes Mellitus, Type 2")
        .mesh_subheading("drug therapy")
        .published_after(2020)
        .limit(10)
        .search_and_fetch(&client)
        .await?;

    // Search with multiple MeSH terms
    let cancer_research = SearchQuery::new()
        .mesh_terms(&["Neoplasms", "Antineoplastic Agents"])
        .clinical_trials_only()
        .limit(5)
        .search_and_fetch(&client)
        .await?;

    Ok(())
}
```

#### Extracting MeSH Information

```rust
use pubmed_client_rs::PubMedClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PubMedClient::new();
    let article = client.fetch_article("31978945").await?;

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
use pubmed_client_rs::PubMedClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PubMedClient::new();

    let article1 = client.fetch_article("31978945").await?;
    let article2 = client.fetch_article("33515491").await?;

    // Calculate MeSH term similarity (Jaccard similarity)
    let similarity = article1.mesh_term_similarity(&article2);
    println!("MeSH similarity: {:.2}%", similarity * 100.0);

    Ok(())
}
```

## Development

### Prerequisites

- Rust 1.70 or later
- [mise](https://mise.jdx.dev/) (optional, for tool management)

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

### Running Tests

This project uses [nextest](https://nexte.st/) for running tests:

```bash
# Run all tests
cargo nextest run

# Run tests with output
cargo nextest run --nocapture

# Run specific test
cargo nextest run test_name

# Using mise tasks
mise run test
mise run test:verbose
```

### Code Quality

```bash
# Format code
cargo fmt
mise run fmt

# Run linter
cargo clippy
mise run lint

# Check code
cargo check
mise run check
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

Generate and view documentation:

```bash
cargo doc --open
mise run doc
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
