# PubMed Client for Rust

A Rust client library for accessing PubMed and PMC (PubMed Central) APIs.

## Features

- **PubMed API Integration**: Search and fetch article metadata
- **PMC Full Text**: Retrieve and parse structured full-text articles
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
