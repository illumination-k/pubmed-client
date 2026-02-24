# pubmed-client: A high-performance, multi-language Rust library for programmatic access to PubMed and PubMed Central

**illumination-k**

Correspondence: <https://github.com/illumination-k/pubmed-client>

---

## Abstract

**Summary:** Programmatic access to biomedical literature through the PubMed and PubMed Central (PMC) APIs requires handling complex XML serialization, NCBI rate-limit policies, transient network failures, and supporting researchers in multiple programming environments. We present **pubmed-client**, a comprehensive, asynchronous Rust library providing type-safe bindings to the full suite of NCBI E-utilities (ESearch, EFetch, ELink, EInfo, ECitMatch, EGQuery, EPost). The library exposes idiomatic, first-class language bindings for Python (via PyO3/maturin), Node.js (via napi-rs native addons), and browser environments (via WebAssembly/wasm-pack), all compiled from a single Rust core. A command-line interface and a Model Context Protocol (MCP) server extend accessibility to shell pipelines and large language model (LLM) assistant workflows, respectively. Built-in token-bucket rate limiting, exponential-backoff retry, and response caching ensure robust, policy-compliant operation in production settings.

**Availability and Implementation:** pubmed-client is freely available under the MIT licence at <https://github.com/illumination-k/pubmed-client>. The core library is published to crates.io (`pubmed-client`), Python bindings to PyPI (`pubmed-client-py`), and JavaScript/TypeScript bindings to npm (`pubmed-client`, `pubmed-client-wasm`). Rust 1.70+, Python 3.12+, and Node.js 18+ are supported.

**Contact:** <https://github.com/illumination-k/pubmed-client/issues>

**Supplementary information:** Code examples and API documentation are available at <https://docs.rs/pubmed-client>.

---

## 1 Introduction

The National Center for Biotechnology Information (NCBI) maintains PubMed, a database of more than 37 million biomedical citations, and PubMed Central (PMC), an open-access archive of more than 9 million full-text articles (Sayers _et al._, 2022). Access to these resources is provided through the Entrez Programming Utilities (E-utilities), a REST API returning structured XML responses (Sayers, 2010). Although the API is freely available, building robust client software is non-trivial: responses require careful XML parsing; NCBI enforces a rate limit of 3 requests per second (10 with a registered API key); transient server errors demand retry logic; and researchers working across Python, R, JavaScript, and other languages currently must use separate, independently maintained client libraries with inconsistent feature sets.

Existing solutions address parts of this problem space. Python libraries such as `pymed` (Wobben, 2019), `biopython.Entrez` (Cock _et al._, 2009), and `metapub` provide PubMed access but lack PMC full-text parsing, MeSH-aware query builders, or async execution. JavaScript clients (`ncbi-eutils`, `node-ncbi`) cover Node.js but offer no native type safety or cross-platform binary distribution. No prior Rust-native client with comprehensive E-utilities coverage and multi-language bindings has been described.

The rapid growth of large language model (LLM)-assisted research workflows introduces an additional requirement: enabling AI assistants to retrieve and reason over primary literature in real time. The Model Context Protocol (MCP; Anthropic, 2024) defines a standardised interface for connecting LLM runtimes to external tools. To our knowledge, pubmed-client is the first published library to expose PubMed search and PMC full-text retrieval as an MCP server, enabling AI assistants such as Claude to query biomedical databases during inference.

Here we describe pubmed-client, a workspace of six Rust crates that collectively provide: (i) a high-performance async core library, (ii) structured PMC full-text parsing with Markdown export, (iii) MeSH-aware query construction, (iv) multi-language bindings (Python, Node.js, WebAssembly), (v) a command-line interface, and (vi) an MCP server for LLM integration.

---

## 2 Description

### 2.1 Architecture and workspace layout

pubmed-client is organised as a Cargo workspace with six crates (Figure 1):

- **`pubmed-client`** – the async Rust core library
- **`pubmed-client-py`** – Python bindings (PyO3/maturin; PyPI: `pubmed-client-py`)
- **`pubmed-client-napi`** – Native Node.js bindings (napi-rs; npm: `pubmed-client`)
- **`pubmed-client-wasm`** – WebAssembly bindings (wasm-pack; npm: `pubmed-client-wasm`)
- **`pubmed-cli`** – command-line interface
- **`pubmed-mcp`** – MCP server for LLM assistants

All language bindings are thin wrappers over the Rust core, guaranteeing behavioural consistency across languages from a single, audited implementation.

```
┌─────────────────────────────────────────────────┐
│               pubmed-client (core)               │
│  PubMedClient · PmcClient · SearchQuery builder  │
│  Rate limiter · Retry · Cache · XML parsers       │
└────────────┬──────────────────────┬──────────────┘
             │                      │
    ┌────────▼────────┐   ┌─────────▼────────────┐
    │  Language       │   │  Applications        │
    │  Bindings       │   │                      │
    │  ┌───────────┐  │   │  ┌────────────────┐  │
    │  │ Python    │  │   │  │ pubmed-cli     │  │
    │  │ (PyO3)    │  │   │  └────────────────┘  │
    │  ├───────────┤  │   │  ┌────────────────┐  │
    │  │ Node.js   │  │   │  │ pubmed-mcp     │  │
    │  │ (napi-rs) │  │   │  │ (MCP server)   │  │
    │  ├───────────┤  │   │  └────────────────┘  │
    │  │ WASM      │  │   │                      │
    │  │ (wasm-pack│  │   └──────────────────────┘
    │  └───────────┘  │
    └─────────────────┘
```

**Figure 1.** Workspace architecture. All language bindings and application layers compile against a single Rust core library, ensuring consistent behaviour.

### 2.2 Core library

The core library exposes two primary clients: `PubMedClient` for article metadata and `PmcClient` for full-text retrieval. A `Client` struct provides a unified entry point with both clients as public fields, plus high-level convenience methods (`search_with_full_text`, `get_related_articles`, `get_pmc_links`, `get_citations`).

**E-utilities coverage.** pubmed-client provides idiomatic Rust bindings for the complete E-utilities suite: ESearch (queried results), EFetch (record retrieval), ELink (citation and related-article graphs), EInfo (database metadata and searchable fields), ECitMatch (unstructured citation parsing), EGQuery (cross-database global query counts), and EPost (History server session management for large result sets).

**SearchQuery builder.** Complex queries are constructed through a fluent builder API:

```rust
let articles = SearchQuery::new()
    .title("CRISPR")
    .author("Doudna")
    .journal("Nature")
    .mesh_major_topic("Gene Editing")
    .published_between(2020, 2024)
    .clinical_trials_only()
    .free_full_text_only()
    .limit(20)
    .search_and_fetch(&client.pubmed)
    .await?;
```

Field tags are validated against the official NCBI vocabulary at compile time; invalid long-form tags (e.g., `[Title]`, `[Author]`) are rejected in favour of validated short-form equivalents (`[ti]`, `[au]`).

**MeSH term support.** Medical Subject Headings (MeSH) are first-class query citizens. The library supports major topic filtering (`[majr]`), subheading qualifiers (`[sh]`), and multi-term boolean combinations. On retrieved articles, helper methods expose MeSH extraction (`get_major_mesh_terms()`), term presence checks (`has_mesh_term()`), qualifier lookup (`get_mesh_qualifiers()`), and pairwise article similarity via Jaccard coefficient (`mesh_term_similarity()`).

### 2.3 PMC full-text parsing and Markdown export

PMC full-text articles in JATS XML format are parsed into a structured `PmcFullText` type containing typed fields for metadata (title, authors, affiliations, identifiers), an ordered `Vec<ArticleSection>`, `Vec<Figure>`, `Vec<Table>`, and `Vec<Reference>`. The `PmcMarkdownConverter` converts this representation to Markdown with configurable options:

```rust
let markdown = PmcMarkdownConverter::new()
    .with_include_metadata(true)
    .with_include_toc(true)
    .with_heading_style(HeadingStyle::ATX)
    .with_reference_style(ReferenceStyle::Numbered)
    .convert(&full_text);
```

This export format is optimised for downstream consumption by LLMs and text analysis pipelines, where structured Markdown with inline citations and figure captions preserves semantic structure more reliably than raw XML or plain text.

### 2.4 Reliability: rate limiting, retry, and caching

NCBI requires clients to observe rate limits of 3 requests per second without an API key and 10 requests per second with one. pubmed-client enforces these limits with a token bucket implementation that operates transparently across all API calls. A configurable exponential-backoff retry policy handles transient HTTP 5xx errors, connection failures, and HTTP 429 responses:

```rust
let retry_config = RetryConfig::new()
    .with_max_retries(5)
    .with_initial_delay(Duration::from_secs(1))
    .with_max_delay(Duration::from_secs(60));
```

Response caching (via `moka`) reduces redundant network requests in batch workflows, with configurable TTL and capacity.

### 2.5 Multi-language bindings

**Python.** The `pubmed-client-py` package (PyPI) provides a synchronous Python API backed by an embedded Tokio runtime, making async Rust execution transparent to Python callers:

```python
import pubmed_client
client = pubmed_client.Client()
articles = client.pubmed.search_and_fetch("CRISPR therapy", max_results=10)
full_text = client.pmc.fetch_full_text("PMC7906746")
```

Wheels are pre-built for Python 3.12 and 3.13 on Linux, macOS, and Windows via maturin.

**Node.js (native).** The `pubmed-client` npm package provides native pre-built binaries for seven platforms (x86\_64 and ARM64 on Linux, macOS, and Windows; Linux musl) via napi-rs, with full TypeScript type declarations:

```typescript
const client = PubMedClient.withConfig({ apiKey: process.env.NCBI_API_KEY });
const articles = await client.executeQuery(
    new SearchQuery().query("cancer immunotherapy").publishedBetween(2022, 2024)
);
```

**WebAssembly.** The `pubmed-client-wasm` npm package compiles the core library to WebAssembly for use in browser environments and edge runtimes, enabling client-side biomedical literature retrieval without a server proxy.

### 2.6 MCP server for LLM integration

The `pubmed-mcp` crate implements an MCP server (Anthropic, 2024) built with the official Rust MCP SDK (rmcp). It exposes two tools to connected LLM runtimes:

- **`search_pubmed`**: Full-featured PubMed search with filters for study type (RCT, systematic review, meta-analysis, case report, etc.), text availability (open access, PMC full text, subscription), and date range.
- **`get_pmc_markdown`**: Fetches and converts a PMC article to structured Markdown, suitable for direct inclusion in an LLM context window.

The server communicates over stdio and is configured in the Claude Desktop `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "pubmed": { "command": "/path/to/pubmed-mcp" }
  }
}
```

Once connected, an LLM assistant can issue natural-language literature queries that are automatically translated into structured E-utilities calls and returned as formatted results within the conversation.

### 2.7 Command-line interface

`pubmed-cli` provides shell access to core features:

```bash
# Search PubMed
pubmed-cli search "CRISPR off-target" --max-results 20

# Convert PMC article to Markdown
pubmed-cli markdown PMC7906746 > article.md

# Extract figures from a PMC article
pubmed-cli figures PMC7906746 --output ./figures/

# Resolve PMID to PMCID
pubmed-cli pmid-to-pmcid 31978945
```

---

## 3 Implementation

The library is written in Rust (edition 2021) and targets stable Rust 1.70+. HTTP requests are issued via `reqwest` with connection pooling. XML parsing uses `quick-xml` with custom deserialisation logic to handle the JATS and PubMed DTD schemas. Structured logging throughout uses the `tracing` framework. The cross-platform time abstraction (`time.rs`) ensures identical behaviour between native targets and WebAssembly, where the standard library clock is unavailable.

The workspace is tested with 406+ test cases across all crates: 203 Rust tests (parsing fixtures, wiremock-mocked integration tests, and opt-in real API tests), 107 Python tests, 76 Node.js TypeScript tests, and 17 WebAssembly tests. Continuous integration runs on GitHub Actions across Linux, macOS, and Windows using matrix jobs for Python 3.12/3.13 and seven native binary targets for the NAPI package.

---

## 4 Discussion

pubmed-client provides the bioinformatics community with a single, audited implementation of NCBI E-utilities access that can be consumed natively in Rust, Python, JavaScript/TypeScript (Node.js and browser), from the command line, or by an LLM assistant. By compiling all language bindings from one Rust core, the library avoids the fragmentation and inconsistency inherent in separately maintained per-language clients. Built-in rate limiting and retry logic ensure policy-compliant behaviour by default, lowering the barrier for researchers building large-scale literature mining pipelines.

The MCP server integration represents a novel capability: to our knowledge, pubmed-client is the first published software to expose PubMed and PMC as first-class tools for AI assistants, enabling LLMs to retrieve, parse, and reason over primary biomedical literature in real time without custom prompt engineering or separate retrieval infrastructure.

Future work includes expanding the MCP server tool suite (citation network traversal, batch article comparison), adding R bindings via the `extendr` framework, and implementing BibTeX/RIS export for reference manager integration.

---

## Funding

No external funding declared.

## Conflict of Interest

None declared.

---

## References

Anthropic (2024). Model Context Protocol specification. <https://modelcontextprotocol.io/>

Cock P.J.A. _et al._ (2009). Biopython: freely available Python tools for computational molecular biology and bioinformatics. _Bioinformatics_, **25**(11), 1422–1423.

National Center for Biotechnology Information (NCBI). NCBI E-utilities. <https://www.ncbi.nlm.nih.gov/books/NBK25499/>

Sayers E. (2010). A General Introduction to the E-utilities. In: Entrez Programming Utilities Help. NCBI.

Sayers E.W. _et al._ (2022). Database resources of the National Center for Biotechnology Information. _Nucleic Acids Research_, **50**(D1), D20–D26.

Wobben G. (2019). PyMed – PubMed Access through Python. <https://github.com/gijswobben/pymed>
