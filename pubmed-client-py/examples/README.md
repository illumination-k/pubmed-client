# Python Examples for PubMed Client

This directory contains comprehensive examples demonstrating how to use the `pubmed-client` Python package to retrieve and analyze biomedical literature metadata from PubMed and PubMed Central (PMC).

All examples support command-line arguments with `--help` for usage information.

## Prerequisites

Before running these examples, you need to install the `pubmed-client` package:

```bash
# Install from PyPI (when published)
pip install pubmed-client

# Or install from source (development)
cd pubmed-client-py
uv run --with maturin maturin develop
```

## Quick Start Examples

### Simple Citation Count

**The simplest way to count citations for a PMID.**

```bash
# Basic usage
python examples/simple_citation_count.py 31978945

# Show more citing articles
python examples/simple_citation_count.py 31978945 --show 10

# With API key for higher rate limits
python examples/simple_citation_count.py 31978945 --api-key YOUR_KEY
```

**Example output:**

```
Counting citations for PMID: 31978945

✓ This article has been cited 14033 times

First 5 citing articles (PMIDs):
  - 30798636
  - 31382259
  - 31628064
  ... and 14028 more
```

## Detailed Examples

### 1. Fetching PubMed Metadata (`01_fetch_pubmed_metadata.py`)

Retrieve detailed article metadata from PubMed using PMIDs.

**Features:**

- Article title, journal, publication date, DOI
- Complete author information with affiliations and ORCID
- Article types and keywords
- Abstract text

**Usage:**

```bash
# Fetch single article
python examples/01_fetch_pubmed_metadata.py 31978945

# Fetch multiple articles
python examples/01_fetch_pubmed_metadata.py 31978945 33515491 25760099

# With API key
python examples/01_fetch_pubmed_metadata.py 31978945 --api-key YOUR_KEY --email you@example.com
```

**Help:**

```bash
python examples/01_fetch_pubmed_metadata.py --help
```

### 2. Fetching PMC Full-Text (`02_fetch_pmc_fulltext.py`)

Retrieve full-text articles from PubMed Central using PMCIDs.

**Features:**

- Complete article sections with content
- Figures and tables with captions
- References with DOIs and PMIDs
- Author information with emails and affiliations

**Usage:**

```bash
# Fetch single PMC article
python examples/02_fetch_pmc_fulltext.py PMC7906746

# Fetch multiple PMC articles
python examples/02_fetch_pmc_fulltext.py PMC7906746 PMC10618641

# With API key
python examples/02_fetch_pmc_fulltext.py PMC7906746 --api-key YOUR_KEY
```

**Help:**

```bash
python examples/02_fetch_pmc_fulltext.py --help
```

### 3. Searching PubMed (`03_search_pubmed.py`)

Search PubMed for articles matching a query.

**Features:**

- Search by keywords, authors, or topics
- Customizable result limit
- Display article metadata and abstracts
- Summary statistics

**Usage:**

```bash
# Basic search
python examples/03_search_pubmed.py "COVID-19 vaccine"

# Search with custom limit
python examples/03_search_pubmed.py "CRISPR gene editing" --limit 20

# With API key
python examples/03_search_pubmed.py "machine learning" --api-key YOUR_KEY
```

**Help:**

```bash
python examples/03_search_pubmed.py --help
```

### 4. Citation Analysis (`04_citation_analysis.py`)

Analyze citations and compare citation counts across articles.

**Features:**

- Count citations for one or multiple PMIDs
- Display citing articles
- Compare citation counts
- Citation statistics (total, average, max, min)

**Usage:**

```bash
# Analyze single article
python examples/04_citation_analysis.py 31978945

# Compare multiple articles
python examples/04_citation_analysis.py 31978945 25760099 33515491

# Show more citing articles
python examples/04_citation_analysis.py 31978945 --show-citing 10

# With API key
python examples/04_citation_analysis.py 31978945 --api-key YOUR_KEY
```

**Help:**

```bash
python examples/04_citation_analysis.py --help
```

## Common Options

All examples support the following common options:

- `--api-key KEY` - NCBI API key for higher rate limits (10 req/s vs 3 req/s)
- `--email EMAIL` - Your email address for NCBI
- `--tool NAME` - Tool name for NCBI (default: "pubmed-client")
- `--help` - Show help message and exit

## Getting an NCBI API Key

1. Visit [NCBI API Keys](https://ncbiinsights.ncbi.nlm.nih.gov/2017/11/02/new-api-keys-for-the-e-utilities/)
2. Create an NCBI account if you don't have one
3. Generate an API key in your account settings
4. Use the key in your commands with `--api-key YOUR_KEY`

**Benefits of using an API key:**

- Higher rate limit (10 requests/second vs 3 requests/second)
- More stable access during high-traffic periods
- Better compliance with NCBI guidelines

## Programming Examples

### Count Citations for a PMID

```python
import pubmed_client

client = pubmed_client.PubMedClient()
citations = client.get_citations([31978945])

print(f"Citation count: {len(citations)}")

# Get PMIDs of citing articles
for citing_pmid in citations.citing_pmids[:10]:
    print(f"Cited by: {citing_pmid}")
```

**Important Note on Citation Counts:**

Citation counts from `get_citations()` may be **lower** than Google Scholar or scite.ai because:

- **PubMed** (this API): Only peer-reviewed articles in PubMed (~14,000 for PMID 31978945)
- **Google Scholar/scite.ai**: Includes preprints, books, conferences (~23,000 for PMID 31978945)

This is expected behavior - the API provides accurate PubMed-specific citation data.

### Fetch Article Metadata

```python
import pubmed_client

client = pubmed_client.PubMedClient()
article = client.fetch_article("31978945")

print(f"Title: {article.title}")
print(f"Authors: {article.author_count}")
print(f"Journal: {article.journal}")

# Access authors and affiliations
for author in article.authors():
    print(f"  {author.full_name}")
    for aff in author.affiliations():
        print(f"    {aff.institution}")
```

### Fetch PMC Full-Text

```python
import pubmed_client

client = pubmed_client.PmcClient()
article = client.fetch_full_text("PMC7906746")

# Access article structure
for section in article.sections():
    print(f"Section: {section.title}")
    print(f"Content: {section.content[:200]}...")

# Access figures and tables
for figure in article.figures():
    print(f"Figure: {figure.label}")
    print(f"Caption: {figure.caption}")
```

### Search PubMed

```python
import pubmed_client

client = pubmed_client.PubMedClient()
articles = client.search_and_fetch("COVID-19 vaccine", 10)

for article in articles:
    print(f"[{article.pmid}] {article.title}")
    print(f"  Journal: {article.journal}")
```

## Type Hints and IDE Support

The package includes complete type stubs (`.pyi` files) for full IDE autocomplete and type checking:

```python
import pubmed_client

# Type hints work automatically
config: pubmed_client.ClientConfig = pubmed_client.ClientConfig()
client: pubmed_client.PubMedClient = pubmed_client.PubMedClient.with_config(config)
articles: list[pubmed_client.PubMedArticle] = client.search_and_fetch("query", 10)
```

Run type checking with mypy:

```bash
mypy your_script.py
```

## Additional Resources

- **Package Documentation**: See the main [README.md](../README.md)
- **Type Stubs**: See [pubmed_client.pyi](../pubmed_client.pyi) for complete API reference
- **Tests**: See [tests/](../tests/) directory for more usage examples
- **NCBI E-utilities Documentation**: https://www.ncbi.nlm.nih.gov/books/NBK25499/
- **NCBI API Keys**: https://ncbiinsights.ncbi.nlm.nih.gov/2017/11/02/new-api-keys-for-the-e-utilities/

## Contributing

Found an issue or have a suggestion for improving these examples? Please open an issue or submit a pull request on GitHub.

## License

These examples are part of the pubmed-client-rs project and are released under the same license as the main project.
