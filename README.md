# pubmed-client

> Type-safe PubMed & PMC API client for Rust, Node.js, WebAssembly, and Python

[![Crates.io](https://img.shields.io/crates/v/pubmed-client)](https://crates.io/crates/pubmed-client)
[![npm](https://img.shields.io/npm/v/pubmed-client)](https://www.npmjs.com/package/pubmed-client)
[![PyPI](https://img.shields.io/pypi/v/pubmed-client-py)](https://pypi.org/project/pubmed-client-py/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/illumination-k/pubmed-client/actions/workflows/rust.yml/badge.svg)](https://github.com/illumination-k/pubmed-client/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/illumination-k/pubmed-client/graph/badge.svg)](https://codecov.io/gh/illumination-k/pubmed-client)

Search and retrieve biomedical literature from [PubMed](https://pubmed.ncbi.nlm.nih.gov/) and [PMC](https://www.ncbi.nlm.nih.gov/pmc/) across multiple languages and runtimes.

## Features

- **Multi-language** — Rust, Node.js (native), WebAssembly, Python
- **Full API coverage** — ESearch, EFetch, ESummary, EPost, ELink, EInfo, ECitMatch, EGQuery, ESpell, PMC OAI
- **Advanced search builder** — filters, date ranges, MeSH terms, boolean logic
- **Full-text retrieval** — structured PMC articles with sections, figures, and tables
- **Markdown export** — convert PMC articles to well-formatted Markdown
- **Rate limiting & retry** — automatic NCBI compliance (3–10 req/sec)
- **Caching** — pluggable backends: in-memory (moka), Redis (`cache-redis` feature), SQLite (`cache-sqlite` feature)
- **MCP server** — integrate with AI assistants

## Installation

| Language         | Command                                         |
| ---------------- | ----------------------------------------------- |
| Rust             | `cargo add pubmed-client`                       |
| Node.js (native) | `npm install pubmed-client`                     |
| WebAssembly      | `npm install pubmed-client-wasm`                |
| Python           | `pip install pubmed-client-py`                  |
| Go               | [build from source](pubmed-client-go/)          |
| CLI              | `cargo install pubmed-cli`                      |
| MCP server       | `docker pull ghcr.io/illumination-k/pubmed-mcp` |

## Quick Start

### Rust

```rust
use pubmed_client::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let articles = client.pubmed
        .search()
        .query("covid-19 treatment")
        .open_access_only()
        .published_after(2020)
        .limit(10)
        .search_and_fetch(&client.pubmed)
        .await?;

    for article in &articles {
        println!("{} — {}", article.pmid, article.title);
    }

    Ok(())
}
```

### Node.js

```typescript
import { PubMedClient } from 'pubmed-client';

const client = PubMedClient.withConfig({ apiKey: process.env.NCBI_API_KEY });
const articles = await client.search('covid-19 vaccine', 10);

articles.forEach(a => console.log(`${a.pmid}: ${a.title}`));
```

### Python

```python
import pubmed_client

client = pubmed_client.Client()
articles = client.pubmed.search_and_fetch("covid-19 treatment", max_results=10)

for article in articles:
    print(f"{article.pmid}: {article.title}")
```

### Go

```go
client, err := pubmedclient.New(&pubmedclient.Config{Email: "you@example.com"})
if err != nil {
	log.Fatal(err)
}
defer client.Close()

articles, err := client.SearchAndFetch("covid-19 treatment", 10)
for _, article := range articles {
	fmt.Printf("%s: %s\n", article.PMID, article.Title)
}
```

### CLI

```bash
pubmed-cli search "covid-19" --max-results 10
pubmed-cli markdown PMC7906746 > article.md
pubmed-cli figures PMC7906746 --output figures/

# Europe PMC — preprints, patents and non-PubMed sources, no API key needed
pubmed-cli europe-pmc search "TITLE:CRISPR AND SRC:PPR" --max 10
pubmed-cli europe-pmc citations PMC3258128
```

## Documentation

Full documentation, examples, and API reference are available at:

**<https://illumination-k.github.io/pubmed-client/>**

| Language | Reference                                                                                            |
| -------- | ---------------------------------------------------------------------------------------------------- |
| Rust     | [docs.rs/pubmed-client](https://docs.rs/pubmed-client)                                               |
| Node.js  | [illumination-k.github.io/pubmed-client/node/](https://illumination-k.github.io/pubmed-client/node/) |
| Python   | [pypi.org/project/pubmed-client-py](https://pypi.org/project/pubmed-client-py/)                      |

## Workspace

| Package                                     | Description                       |
| ------------------------------------------- | --------------------------------- |
| [`pubmed-client`](pubmed-client/)           | Core Rust library                 |
| [`pubmed-client-napi`](pubmed-client-napi/) | Native Node.js bindings (napi-rs) |
| [`pubmed-client-wasm`](pubmed-client-wasm/) | WebAssembly bindings              |
| [`pubmed-client-py`](pubmed-client-py/)     | Python bindings (PyO3)            |
| [`pubmed-client-go`](pubmed-client-go/)     | Go bindings (cgo)                 |
| [`pubmed-client-r`](pubmed-client-r/)       | R bindings (extendr)              |
| [`pubmed-cli`](pubmed-cli/)                 | Command-line interface            |
| [`pubmed-mcp`](pubmed-mcp/)                 | MCP server for AI assistants      |
