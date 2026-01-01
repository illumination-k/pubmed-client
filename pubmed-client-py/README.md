# PubMed Client - Python Bindings

[![PyPI version](https://badge.fury.io/py/pubmed-client-py.svg)](https://pypi.org/project/pubmed-client-py/)
[![Python 3.12+](https://img.shields.io/badge/python-3.12+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Python bindings for the PubMed and PMC (PubMed Central) API client library.

## Overview

This package provides Python bindings to the Rust-based PubMed client library, enabling high-performance access to PubMed and PMC APIs from Python.

## Features

- **PubMed API**: Search and retrieve article metadata
- **PMC API**: Access full-text articles from PubMed Central
- **ELink API**: Get related articles, citations, and PMC links
- **SearchQuery Builder**: Build complex queries programmatically with filters
- **High Performance**: Built with Rust for speed and reliability
- **Type-Safe**: Full type hints for better IDE support

## Installation

### From PyPI

```bash
pip install pubmed-client-py
```

### With uv

```bash
uv add pubmed-client-py
```

### From Source

```bash
# Clone the repository
git clone https://github.com/illumination-k/pubmed-client.git
cd pubmed-client/pubmed-client-py

# Create virtual environment and install
uv venv
uv run --with maturin maturin develop
```

## Quick Start

### Basic Usage

```python
import pubmed_client

# Create a unified client
client = pubmed_client.Client()

# Search PubMed
articles = client.pubmed.search_and_fetch("covid-19 vaccine", max_results=10)

for article in articles:
    print(f"Title: {article.title}")
    print(f"PMID: {article.pmid}")
    print(f"Journal: {article.journal}")
    print()
```

### Fetch Single Article

```python
import pubmed_client

client = pubmed_client.Client()

# Fetch article by PMID
article = client.pubmed.fetch_article("31978945")

print(f"Title: {article.title}")
print(f"Authors: {article.author_count}")
print(f"Abstract: {article.abstract_text[:200]}...")

# Access authors and affiliations
for author in article.authors():
    print(f"  {author.full_name}")
    if author.orcid:
        print(f"    ORCID: {author.orcid}")
```

### Fetch PMC Full-Text

```python
import pubmed_client

client = pubmed_client.Client()

# Fetch full text from PMC
full_text = client.pmc.fetch_full_text("PMC7906746")

print(f"Title: {full_text.title}")
print(f"Sections: {len(full_text.sections())}")
print(f"References: {len(full_text.references())}")

# Access article sections
for section in full_text.sections():
    print(f"Section: {section.title}")
    print(f"Content: {section.content[:200]}...")

# Access figures and tables
for figure in full_text.figures():
    print(f"Figure: {figure.label}")
    print(f"Caption: {figure.caption}")

# Convert to Markdown
markdown = full_text.to_markdown()
print(markdown)
```

### Citation Analysis

```python
import pubmed_client

client = pubmed_client.Client()

# Get citations for an article
citations = client.get_citations([31978945])
print(f"Citation count: {len(citations)}")

# Get citing article PMIDs
for citing_pmid in citations.citing_pmids[:10]:
    print(f"Cited by: {citing_pmid}")
```

### Related Articles and PMC Links

```python
import pubmed_client

client = pubmed_client.Client()

# Find related articles
related = client.get_related_articles([31978945])
print(f"Found {len(related.related_pmids)} related articles")

# Check PMC full-text availability
pmc_links = client.get_pmc_links([31978945])
print(f"PMC IDs available: {pmc_links.pmc_ids}")
```

### Using SearchQuery Builder

```python
import pubmed_client

client = pubmed_client.Client()

# Build a complex query
query = (
    pubmed_client.SearchQuery()
    .query("cancer")
    .published_between(2020, 2024)
    .article_type("Clinical Trial")
    .free_full_text_only()
    .limit(50)
)

# Execute the search
articles = client.pubmed.search_and_fetch(query, 0)  # limit ignored when using SearchQuery

for article in articles:
    print(f"[{article.pmid}] {article.title}")
```

### Boolean Query Combinations

```python
import pubmed_client

# Build complex queries with boolean logic
q1 = pubmed_client.SearchQuery().query("covid-19")
q2 = pubmed_client.SearchQuery().query("vaccine")
q3 = pubmed_client.SearchQuery().query("efficacy")

# Combine with AND
combined = q1.and_(q2).and_(q3)
print(combined.build())  # ((covid-19) AND (vaccine)) AND (efficacy)

# Combine with OR
either = q1.or_(q2)
print(either.build())  # (covid-19) OR (vaccine)

# Exclude specific terms
base = pubmed_client.SearchQuery().query("treatment")
excluded = pubmed_client.SearchQuery().query("animal studies")
human_only = base.exclude(excluded)
print(human_only.build())  # (treatment) NOT (animal studies)
```

### Extract Figures from PMC Articles

```python
import pubmed_client

client = pubmed_client.Client()

# Download and extract figures with captions
figures = client.pmc.extract_figures_with_captions("PMC7906746", "./output")

for fig in figures:
    print(f"Figure: {fig.figure.label}")
    print(f"Caption: {fig.figure.caption}")
    print(f"File: {fig.extracted_file_path}")
    print(f"Size: {fig.file_size} bytes")
    if fig.dimensions:
        print(f"Dimensions: {fig.dimensions[0]}x{fig.dimensions[1]}")
```

## Configuration

### With API Key

Using an NCBI API key increases rate limits from 3 to 10 requests per second:

```python
import pubmed_client

config = (
    pubmed_client.ClientConfig()
    .with_api_key("your_ncbi_api_key")
    .with_email("your@email.com")
    .with_tool("YourAppName")
)

client = pubmed_client.Client.with_config(config)
```

### Rate Limiting

```python
import pubmed_client

config = (
    pubmed_client.ClientConfig()
    .with_rate_limit(2.0)  # 2 requests per second
    .with_timeout_seconds(60)  # 60 second timeout
)

client = pubmed_client.Client.with_config(config)
```

## Type Hints and IDE Support

The package includes complete type stubs (`.pyi` files) for full IDE autocomplete and type checking:

```python
import pubmed_client

# Type hints work automatically
config: pubmed_client.ClientConfig = pubmed_client.ClientConfig()
client: pubmed_client.Client = pubmed_client.Client.with_config(config)
articles: list[pubmed_client.PubMedArticle] = client.pubmed.search_and_fetch("query", 10)
```

Run type checking with mypy:

```bash
mypy your_script.py
```

## Development

### Prerequisites

- Python >= 3.12
- Rust toolchain (installed via rustup)
- uv (for Python package management)
- maturin (for building Python bindings)

### Setup Development Environment

```bash
# Install uv if not already installed
curl -LsSf https://astral.sh/uv/install.sh | sh

# Create virtual environment
cd pubmed-client-py
uv venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Build and install in development mode
uv run --with maturin maturin develop
```

### Running Tests

```bash
# Install dev dependencies
uv sync --group dev

# Run tests
uv run pytest

# Run tests with coverage
uv run pytest --cov=pubmed_client
```

### Code Quality

```bash
# Format code
uv run ruff format

# Lint code
uv run ruff check

# Type checking
uv run mypy tests/
```

## Building

```bash
# Build wheel
uv run --with maturin maturin build --release

# Build for distribution
uv run --with maturin maturin build --release --sdist
```

## Publishing

```bash
# Publish to PyPI (requires credentials)
uv run --with maturin maturin publish
```

## License

MIT

## Links

- [Repository](https://github.com/illumination-k/pubmed-client)
- [PyPI Package](https://pypi.org/project/pubmed-client-py/)
- [Core Rust Library](../pubmed-client)
- [CLI Tool](../pubmed-cli)
- [WebAssembly Bindings](../pubmed-client-wasm)
- [NCBI E-utilities Documentation](https://www.ncbi.nlm.nih.gov/books/NBK25501/)
