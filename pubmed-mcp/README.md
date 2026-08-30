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
- **Modular Architecture**: Tools organized in separate modules for maintainability
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

`--tools` restricts which tools are exposed (comma-separated, default: all):

```bash
docker run --rm -i ghcr.io/illumination-k/pubmed-mcp:latest --tools search,markdown
```

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
- `include_mesh` (boolean, optional): Include MeSH headings and chemical substances (default: true). Major topics are marked with `*`, qualifiers appear as `Descriptor/qualifier`.
- `include_affiliations` (boolean, optional): Include author affiliations (default: false). Off by default because PubMed repeats a collaboration's entire affiliation string on every author; when enabled, each distinct affiliation is listed once with the authors that share it.

**Returns:**

Per article: title, PMID, PMC ID, DOI, journal (with volume/issue/pages, ISO
abbreviation, ISSN, language), all authors, article types, full abstract,
author keywords, MeSH terms, and substances. PMIDs that returned no record are
listed under `Not found:` rather than silently omitted.

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

Well-formatted Markdown document containing:

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

## Development

### Project Structure

```
pubmed-mcp/
├── Cargo.toml           # Package configuration
├── src/
│   ├── main.rs          # MCP server implementation with tool router
│   ├── config.rs        # CLI flags / environment variables -> ClientConfig
│   └── tools/           # Tools module
│       ├── mod.rs       # PubMedServer definition
│       ├── search.rs    # Search tool implementation
│       ├── articles.rs  # Full PubMed record retrieval (EFetch)
│       └── markdown.rs  # Markdown conversion tool
├── tests/
│   └── integration_test.rs  # Integration tests
├── README.md            # This file
└── SEARCH_FILTERS.md    # Detailed filter documentation
```

### Dependencies

- **rmcp**: Official Rust SDK for Model Context Protocol
- **pubmed-client**: Core library for PubMed/PMC API access
- **tokio**: Async runtime
- **schemars**: JSON schema generation for tool parameters
- **tracing**: Structured logging

### Adding More Tools

To add additional tools, add methods to the `PubMedServer` impl block annotated with `#[tool]`:

```rust
#[tool(description = "Your tool description")]
async fn your_tool(
    &self,
    Parameters(params): Parameters<YourRequestStruct>,
) -> Result<CallToolResult, ErrorData> {
    // Implementation
    Ok(CallToolResult::success(vec![Content::text(result)]))
}
```

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
