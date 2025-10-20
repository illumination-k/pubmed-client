# PubMed Client - Python Bindings

Python bindings for the PubMed and PMC (PubMed Central) API client library.

## Overview

This package provides Python bindings to the Rust-based PubMed client library, enabling high-performance access to PubMed and PMC APIs from Python.

## Features

- **PubMed API**: Search and retrieve article metadata
- **PMC API**: Access full-text articles from PubMed Central
- **High Performance**: Built with Rust for speed and reliability
- **Async Support**: Asynchronous API calls for efficient data retrieval
- **Type-Safe**: Full type hints for better IDE support

## Installation

### From PyPI (when published)

```bash
pip install pubmed-client
```

### From Source with uv

```bash
# Clone the repository
git clone https://github.com/illumination-k/pubmed-client-rs.git
cd pubmed-client-rs/pubmed-client-py

# Create virtual environment and install
uv venv
uv pip install -e .
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

# Install maturin via uv
uv tool install maturin

# Create virtual environment
cd pubmed-client-py
uv venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Install in development mode
uv pip install -e .

# Or use maturin develop for faster iteration
uv run maturin develop
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
uv run mypy
```

## Usage

Coming soon! The bindings are currently in development.

## Building

```bash
# Build wheel
uv run maturin build --release

# Build and install in development mode
uv run maturin develop

# Build for distribution
uv run maturin build --release --sdist
```

## Publishing

```bash
# Publish to PyPI (requires credentials)
uv run maturin publish
```

## License

MIT

## Links

- [Repository](https://github.com/illumination-k/pubmed-client-rs)
- [Core Rust Library](../pubmed-client)
- [CLI Tool](../pubmed-cli)
- [WebAssembly Bindings](../pubmed-client-wasm)
