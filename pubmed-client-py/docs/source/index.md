# pubmed-client-py

[![PyPI version](https://badge.fury.io/py/pubmed-client-py.svg)](https://pypi.org/project/pubmed-client-py/)
[![Python 3.12+](https://img.shields.io/badge/python-3.12+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Python bindings for the Rust-based [pubmed-client](https://github.com/illumination-k/pubmed-client) library,
providing high-performance access to [PubMed](https://pubmed.ncbi.nlm.nih.gov/) and
[PubMed Central (PMC)](https://www.ncbi.nlm.nih.gov/pmc/) APIs.

## Features

- **PubMed API** — Search and retrieve article metadata (titles, authors, abstracts, MeSH terms)
- **PMC API** — Access full-text articles with structured sections, figures, and tables
- **ELink API** — Find related articles, citations, and PMC links
- **SearchQuery builder** — Compose complex queries programmatically with filters and boolean logic
- **High performance** — Built with Rust for speed and reliability
- **Type-safe** — Full type hints via `.pyi` stubs for IDE support and mypy

## Contents

```{toctree}
:maxdepth: 2

quickstart
api
```
