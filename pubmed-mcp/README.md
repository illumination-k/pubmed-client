# PubMed MCP Server

Model Context Protocol (MCP) server for searching and retrieving biomedical literature from PubMed and PubMed Central (PMC) databases.

## Overview

This MCP server provides tools for interacting with the PubMed and PMC APIs through the Model Context Protocol, allowing AI assistants like Claude to search and retrieve biomedical research articles.

## Features

- **Enhanced PubMed Search**: Search the PubMed database with advanced filtering
  - Filter by study type (RCT, meta-analysis, systematic review, etc.)
  - Filter by text availability (open access, free full text, PMC full text)
  - Support for all PubMed field tags and boolean operators
- **Full PubMed Records**: Fetch complete metadata by PMID — full abstract, MeSH headings, keywords, affiliations, and identifiers
- **PMC Markdown Conversion**: Convert PMC full-text articles to well-formatted markdown
  - Configurable metadata, table of contents, and figure captions
  - Proper handling of references, funding information, and acknowledgments
  - Clean HTML entity decoding and content formatting
- **Europe PMC Access**: Search and retrieve from Europe PMC alongside NCBI
  - Cross-source search covering preprints (PPR), patents, Agricola and CBA as well as PubMed/PMC
  - JATS full text (parsed or raw XML), reference and citation graphs
  - Cross-references to external databases (UniProt, EMBL, PDB, ...)
  - No API key required
- **Structured output**: Every tool answers with JSON `structuredContent` and advertises the matching `outputSchema`
- **Modular Architecture**: Tools organized in separate modules for maintainability
- **Backtesting**: `--as-of` pins the whole server to a past day, so no tool returns a record that became available later
- **Configurable client**: NCBI API key, contact e-mail, tool name, rate limit, timeout, retries, and response caching, via CLI flags or environment variables
- Built with [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - the official Rust SDK for MCP
- Uses stdio transport for communication

## Installation

### Container image (GHCR)

Every release publishes a multi-arch (`linux/amd64` + `linux/arm64`) image to the
GitHub Container Registry:

```bash
docker pull ghcr.io/illumination-k/pubmed-mcp:latest
```

Available tags: `latest`, `X.Y.Z` (immutable), and `X.Y` (moves with the newest
patch of that minor). Prereleases publish `X.Y.Z-rc.N` only — they never move
`latest` or `X.Y`.

### Building from Source

```bash
# From workspace root
cargo build --release -p pubmed-mcp

# The binary will be at:
# target/release/pubmed-mcp
```

Or build the image yourself — note the build context is the **workspace root**,
since the crate depends on the sibling crates by path:

```bash
docker build -f pubmed-mcp/Dockerfile -t pubmed-mcp .
```

## Usage

### Running the Server

The server communicates via standard input/output (stdio):

```bash
cargo run -p pubmed-mcp

# ...or from the container image (`-i` is required: stdio is the transport)
docker run --rm -i ghcr.io/illumination-k/pubmed-mcp:latest
```

With `--port`, it serves the streamable HTTP transport at `/mcp` instead:

```bash
docker run --rm -p 8080:8080 ghcr.io/illumination-k/pubmed-mcp:latest --port 8080
```

The HTTP transport only accepts requests whose `Host` header is a loopback
name (`localhost`, `127.0.0.1`, `::1`) and answers anything else with
`403 Forbidden`, as protection against DNS rebinding. Behind a reverse proxy,
load balancer, or container platform ingress, pass the name clients use with
`--allowed-hosts` (or `PUBMED_MCP_ALLOWED_HOSTS`), comma-separated, as `host`
or `host:port`. The list replaces the loopback default, so add `localhost`
too if local clients still need to connect:

```bash
docker run --rm -p 8080:8080 ghcr.io/illumination-k/pubmed-mcp:latest \
  --port 8080 --allowed-hosts mcp.example.com,localhost
```

`--tools` restricts which tools are exposed (comma-separated, default: all):

```bash
docker run --rm -i ghcr.io/illumination-k/pubmed-mcp:latest --tools search,markdown
```

### Backtesting: `--as-of`

`--as-of YYYY-MM-DD` (or `PUBMED_MCP_AS_OF`) makes the server behave as if it
ran on that day. It is meant for backtests — "given only what was available
before date D, would an agent have found X?" — where the date has to be
enforced by the server rather than left to the model to remember:

```bash
docker run --rm -i ghcr.io/illumination-k/pubmed-mcp:latest --as-of 2020-02-01
```

No tool returns a record that became available **on or after** D, and such a
record looks exactly like one that does not exist: the same empty result,
`not_found` entry or "not available" error. Nothing reports how many records
were held back, because that count would itself reveal that later evidence
exists. What every answer does carry is an `as_of` field with the date (it is
also in the server's `instructions`, and in `search_pubmed`'s
`filters_applied`), so the agent and the person reading its transcript know the
view is bounded. The date must be a full ISO date; anything else stops the
server from starting.

| Tool                                                      | With `--as-of D`                                                                                                                                                                |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `search_pubmed`                                           | Bounded by ESearch's own date parameters (`datetype=crdt`, up to D-1), then each hit is checked. A later `end_year` is pulled back to D-1's year. `count` is the bounded count. |
| `fetch_articles`, `fetch_summaries`, `export_citations`   | Later records are left out (`fetch_articles` lists them under `not_found`).                                                                                                     |
| `get_pmc_markdown`, `get_pmc_fulltext`, `get_pmc_figures` | Refused unless the article's PubMed record is visible.                                                                                                                          |
| `get_related_articles`, `get_citations`, `get_pmc_links`  | Later records are dropped, and `total` counts only what is left. A later source PMID yields nothing.                                                                            |
| `match_citations`, `pmid_to_pmcid`                        | A citation or PMID that resolves to a later record resolves to nothing.                                                                                                         |
| `europe_pmc_search`                                       | `FIRST_IDATE:[* TO D-1]` is ANDed onto the query, then each record is checked.                                                                                                  |
| `europe_pmc_citations`, `europe_pmc_references`           | Empty for a later record; entries matched to a later record are dropped.                                                                                                        |
| `europe_pmc_fulltext`, `europe_pmc_database_links`        | Refused / empty for a later record.                                                                                                                                             |
| `global_query`                                            | **Not offered**: per-database hit counts cannot all be bounded by date.                                                                                                         |
| `spell_check`, `list_databases`, `get_database_info`      | Unchanged (they report on today's databases).                                                                                                                                   |

**Which date.** "Available" means the earliest day a reader could have found
the record, not its publication date — an epub-ahead-of-print article is in
PubMed before the issue it belongs to, and is returned even when that issue is
dated after D.

- **PubMed:** the later of the record's _create date_ (`CRDT`) and its _Entrez
  date_ (`EDAT`). The Entrez date alone is not safe: a record added to PubMed
  more than a year after publication has its `EDAT` set back to the publication
  date, and a year-only publication date becomes January 1st. PMID 34695337,
  published in September 2020 and added in October 2021, carries `EDAT`
  2020-01-01 and would otherwise be handed to a backtest pinned at February 2020. The create date is when the record really appeared; requiring both
  to precede D admits nothing that either would refuse.
- **Europe PMC:** the first index date (`firstIndexDate`, the `FIRST_IDATE`
  search field).

A record lacking these falls back to the earliest of its other history dates,
and a record with no usable date at all is hidden.

**Limits worth knowing.**

- The rule errs towards hiding. A paper published before D but added to PubMed
  after it is hidden, and Europe PMC only began indexing around 2007–2010, so
  with an early D its tools return very little (a 1991 article has a first
  index date in 2009).
- Records are returned **as they are today**. MeSH headings, corrections,
  retraction notices and PMC IDs added after D are still in a visible record,
  and PMC full text is today's version. Europe PMC citation counts are the
  exception: they are removed, and sorting `europe_pmc_search` by citation
  count is refused, since both describe what happened to a record afterwards.
- Only the returned records are bounded. Relevance ranking is computed by NCBI
  and Europe PMC over today's corpus.
- `europe_pmc_search` rejects a query with unbalanced parentheses or quotes,
  which could otherwise close the bound's parenthesis early.
- Each bounded call costs one or two extra requests for the date lookup (an
  ESummary per 200 PMIDs, or a Europe PMC search).

### Client Configuration

Every option below is available both as a CLI flag and as an environment
variable, since MCP hosts differ in which one is easier to set. Flags win over
environment variables.

| Flag               | Environment variable        | Default                  | Description                                                                   |
| ------------------ | --------------------------- | ------------------------ | ----------------------------------------------------------------------------- |
| `--api-key`        | `NCBI_API_KEY`              | _(none)_                 | NCBI E-utilities API key. Raises the rate limit from 3 to 10 requests/second. |
| `--email`          | `NCBI_EMAIL`                | _(none)_                 | Contact e-mail sent to NCBI (recommended by their usage guidelines).          |
| `--tool`           | `NCBI_TOOL`                 | `pubmed-mcp`             | Tool name sent to NCBI.                                                       |
| `--rate-limit`     | `NCBI_RATE_LIMIT`           | 3, or 10 with an API key | Requests per second. Overrides the API-key-based default.                     |
| `--timeout`        | `PUBMED_MCP_TIMEOUT`        | `30`                     | HTTP request timeout, in seconds.                                             |
| `--max-retries`    | `PUBMED_MCP_MAX_RETRIES`    | `3`                      | Retries for transient failures (exponential backoff).                         |
| `--base-url`       | `PUBMED_MCP_BASE_URL`       | NCBI E-utilities         | Alternate E-utilities base URL, for proxies or test environments.             |
| `--cache`          | `PUBMED_MCP_CACHE`          | off                      | Enable the in-memory response cache.                                          |
| `--cache-capacity` | `PUBMED_MCP_CACHE_CAPACITY` | `1000`                   | Maximum number of cached responses. Implies `--cache`.                        |
| `--cache-ttl`      | `PUBMED_MCP_CACHE_TTL`      | `604800` (7 days)        | Time-to-live for cached responses, in seconds. Implies `--cache`.             |

`NCBI_API_KEY`, `NCBI_EMAIL`, and `NCBI_TOOL` are the same variables `pubmed-cli`
reads, so a shell that is already set up for the CLI needs no extra
configuration. `PUBMED_MCP_CACHE` accepts any boolish value (`1`, `true`,
`yes`, `on`, and their negatives).

Getting an API key is worthwhile for anything beyond casual use — it more than
triples the request rate. Register at
[NCBI account settings](https://www.ncbi.nlm.nih.gov/account/settings/).

```bash
# Authenticated, with responses cached for an hour
NCBI_API_KEY=your_key NCBI_EMAIL=you@example.edu \
  cargo run -p pubmed-mcp -- --cache --cache-ttl 3600
```

### Configuration with Claude Desktop

Add to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pubmed": {
      "command": "/path/to/pubmed-client/target/release/pubmed-mcp",
      "env": {
        "NCBI_API_KEY": "your_api_key_here",
        "NCBI_EMAIL": "you@example.edu",
        "PUBMED_MCP_CACHE": "true"
      }
    }
  }
}
```

Using the container image instead (no local Rust toolchain needed). Note that
`docker run` does not inherit the host environment, so the variables have to be
forwarded with `-e`:

```json
{
  "mcpServers": {
    "pubmed": {
      "command": "docker",
      "args": [
        "run",
        "--rm",
        "-i",
        "-e",
        "NCBI_API_KEY",
        "-e",
        "NCBI_EMAIL",
        "ghcr.io/illumination-k/pubmed-mcp:latest",
        "--cache"
      ],
      "env": {
        "NCBI_API_KEY": "your_api_key_here",
        "NCBI_EMAIL": "you@example.edu"
      }
    }
  }
}
```

### Available Tools

Every tool returns **structured output**: the answer is a JSON object in the
response's `structuredContent`, and each tool advertises the JSON Schema for it
as its `outputSchema`, so a client can validate or destructure the result
instead of parsing prose. The same JSON is echoed as a text content block, so
clients that only read `content` still receive the whole answer.

Fields that have no value are omitted rather than sent as `null`; list fields
are always present, empty when there is nothing to report (including when a
flag such as `include_mesh` turned them off).

#### `search_pubmed`

Search PubMed for articles with advanced filtering options.

**Parameters:**

- `query` (string, required): Search query using PubMed syntax
  - Examples: `"COVID-19"`, `"cancer[ti] AND therapy[tiab]"`
  - Supports [PubMed field tags](https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags)
- `max_results` (integer, optional): Maximum number of results to return (default: 10, max: 100)
- `study_type` (enum, optional): Filter by study type
  - `randomized_controlled_trial` - RCTs
  - `clinical_trial` - Clinical trials
  - `meta_analysis` - Meta-analyses
  - `systematic_review` - Systematic reviews
  - `review` - Review articles
  - `observational_study` - Observational studies
  - `case_report` - Case reports
- `text_availability` (enum, optional): Filter by text availability
  - `free_full_text` - Free full text only (includes PMC, Bookshelf, and publishers' websites)
  - `full_text` - Full text links (including subscription-based)
  - `pmc_only` - PMC full text only
- `start_year` (integer, optional): Start year for date range filter (inclusive)
- `end_year` (integer, optional): End year for date range filter (inclusive, optional)
- `include_abstract` (boolean, optional): Include abstract preview in results (default: true)

**Examples:**

```
Search for RCTs on COVID-19 treatment (max 20 results)
```

```
Search for free full text meta-analyses on cancer immunotherapy
```

For detailed examples and filter combinations, see [SEARCH_FILTERS.md](SEARCH_FILTERS.md).

#### `fetch_articles`

Fetch complete PubMed records for PMIDs you already have (EFetch). Use this when
`search_pubmed`'s 200-character abstract preview or `fetch_summaries`'
bibliographic overview is not enough — this is the only tool that returns full
abstracts and MeSH indexing.

**Parameters:**

- `pmids` (array of strings, required): PubMed IDs, e.g. `["31978945", "33515491"]`. At most 100 per call.
- `include_abstract` (boolean, optional): Include the full abstract (default: true). Structured abstracts keep their `BACKGROUND`/`METHODS`/`RESULTS` labels.
- `include_mesh` (boolean, optional): Include MeSH headings and chemical substances (default: true). Each heading keeps its `major_topic` flag and its qualifiers.
- `include_affiliations` (boolean, optional): Include author affiliations (default: false). Off by default because PubMed repeats a collaboration's entire affiliation string on every author; when enabled, each distinct affiliation appears once with the authors that share it.

**Returns:**

`{ requested, count, not_found, articles }`. Each article carries `pmid`,
`title`, `pmc_id`, `doi`, a `journal` object (title, ISO abbreviation, ISSN,
publication date, volume, issue, pages), `language`, `authors`, `author_count`,
`affiliations`, `article_types`, `abstract_text` or `structured_abstract`,
`keywords`, `mesh_terms` and `substances`. Requested PMIDs that returned no
record are reported in `not_found` rather than silently omitted.

**Examples:**

```
Get the full records for PMIDs 31978945 and 33515491
```

```
What MeSH terms is PMID 31978945 indexed under?
```

#### `get_pmc_markdown`

Convert a PMC (PubMed Central) full-text article to well-formatted Markdown.

**Parameters:**

- `pmc_id` (string, required): PMC ID with or without "PMC" prefix
  - Examples: `"PMC7906746"` or `"7906746"`
- `include_metadata` (boolean, optional): Include article metadata section (default: true)
  - Title, authors, journal, publication date, identifiers, keywords
- `include_figure_captions` (boolean, optional): Include figure and table captions (default: true)

**Returns:**

`{ pmc_id, title, markdown }`, where `markdown` is a whole document — this
tool's product is prose, so it is not split into fields; use `get_pmc_fulltext`
for the section tree. The document contains:

- Article metadata (title, authors, journal, identifiers)
- Full article text organized by sections
- References with DOI/PMID links
- Funding information, acknowledgments, and data availability statements
- Figure and table captions

**Examples:**

```
Get markdown for PMC article PMC7906746 with all metadata
```

```
Get markdown for PMC article 7906746 without table of contents
```

```
Get markdown for PMC7906746 with minimal formatting (no metadata or captions)
```

#### Europe PMC tools

[Europe PMC](https://europepmc.org) is a complementary index to the NCBI E-utilities: it covers
preprints, patents and agricultural literature in addition to PubMed and PMC, and needs no API key.

Every Europe PMC record is addressed by a `(source, id)` pair. All of the tools below accept the id
either bare (`"PMC3258128"`, `"33515491"`) or fully qualified (`"PPR/PPR123456"`), and take an
optional `source` (`MED`, `PMC`, `PPR`, `AGR`, `CBA`, `PAT`). With no `source`, a `PMC`-prefixed id
is read as a PMC record and anything else as a PubMed (`MED`) record.

##### `europe_pmc_search`

Search across every Europe PMC source.

- `query` (string, required): Europe PMC query syntax, e.g. `"TITLE:CRISPR AND SRC:PPR"`
- `max_results` (integer, optional): default 10, max 100
- `result_type` (enum, optional): `id_list`, `lite` (default), or `core` (adds abstracts and citation counts)
- `sort` (string, optional): Europe PMC sort expression, e.g. `"P_PDATE_D desc"`, `"CITED desc"`

##### `europe_pmc_fulltext`

Fetch full text for a record.

- `id` (string, required), `source` (string, optional)
- `raw_xml` (boolean, optional): return the raw JATS XML instead of parsed sections (default: false).
  Required for non-`PMC` sources, since parsing into an article requires a PMC id.
- `max_sections` (integer, optional): limit the number of body sections returned

##### `europe_pmc_references`

List the works cited by a record (title, authors, journal, PMID, DOI where matched).

- `id` (string, required), `source` (string, optional)
- `max_results` (integer, optional): default 50, max 100

##### `europe_pmc_citations`

List the articles citing a record. Broader than `get_citations`, which is PubMed-only.

- `id` (string, required), `source` (string, optional)
- `max_results` (integer, optional): default 50, max 100

##### `europe_pmc_database_links`

List cross-references from a record to external biological databases.

- `id` (string, required), `source` (string, optional)
- `db_name` (string, optional): filter to a single database, e.g. `"UNIPROT"`
- `max_entries_per_db` (integer, optional): entries shown per database (default: 20)

## Development

### Project Structure

```
pubmed-mcp/
├── Cargo.toml           # Package configuration
├── src/
│   ├── main.rs          # MCP server implementation with tool router
│   ├── config.rs        # CLI flags / environment variables -> ClientConfig
│   └── tools/           # Tools module
│       ├── mod.rs         # PubMedServer definition
│       ├── output.rs      # Structured output types shared by the tools
│       ├── search.rs      # Search tool implementation
│       ├── articles.rs    # Full PubMed record retrieval (EFetch)
│       ├── markdown.rs    # Markdown conversion tool
│       └── europe_pmc.rs  # Europe PMC search / full text / citation graph tools
├── tests/
│   └── integration_test.rs  # Integration tests
├── README.md            # This file
└── SEARCH_FILTERS.md    # Detailed filter documentation
```

### Dependencies

- **rmcp**: Official Rust SDK for Model Context Protocol
- **pubmed-client**: Core library for PubMed/PMC API access
- **tokio**: Async runtime
- **schemars**: JSON schema generation for tool parameters and structured output
- **tracing**: Structured logging

### Adding More Tools

To add additional tools, add methods to the `PubMedServer` impl block annotated with `#[tool]`:

```rust
#[tool(description = "Your tool description")]
async fn your_tool(
    &self,
    Parameters(params): Parameters<YourRequestStruct>,
) -> Result<Json<YourOutputStruct>, ErrorData> {
    // Implementation
    Ok(Json(output))
}
```

Return `Json<T>`, never a bare `CallToolResult`: rmcp derives the tool's
`outputSchema` from `T` and puts the serialized value in `structuredContent`.
`T` needs `Serialize` and `schemars::JsonSchema`, and its root must be a
struct — the MCP spec requires an object at the top level. Shapes reused by
more than one tool live in `src/tools/output.rs`.

### Logging

Enable logging with the `RUST_LOG` environment variable:

```bash
RUST_LOG=info cargo run -p pubmed-mcp
RUST_LOG=debug cargo run -p pubmed-mcp  # More verbose
```

## Testing

### Testing with MCP Inspector

The [MCP Inspector](https://github.com/modelcontextprotocol/inspector) is a useful tool for testing MCP servers:

```bash
npx @modelcontextprotocol/inspector cargo run -p pubmed-mcp
```

## License

MIT

## References

- [Model Context Protocol](https://modelcontextprotocol.io/)
- [rmcp - Rust SDK for MCP](https://github.com/modelcontextprotocol/rust-sdk)
- [PubMed API Documentation](https://www.ncbi.nlm.nih.gov/books/NBK25499/)
