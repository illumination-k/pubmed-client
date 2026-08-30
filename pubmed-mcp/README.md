# PubMed MCP Server

Model Context Protocol (MCP) server for searching and retrieving biomedical literature from PubMed and PubMed Central (PMC) databases.

## Overview

This MCP server provides tools for interacting with the PubMed and PMC APIs through the Model Context Protocol, allowing AI assistants like Claude to search and retrieve biomedical research articles.

## Features

- **Enhanced PubMed Search**: Search the PubMed database with advanced filtering
  - Filter by study type (RCT, meta-analysis, systematic review, etc.)
  - Filter by text availability (open access, free full text, PMC full text)
  - Support for all PubMed field tags and boolean operators
- **PMC Markdown Conversion**: Convert PMC full-text articles to well-formatted markdown
  - Configurable metadata, table of contents, and figure captions
  - Proper handling of references, funding information, and acknowledgments
  - Clean HTML entity decoding and content formatting
- **Europe PMC Access**: Search and retrieve from Europe PMC alongside NCBI
  - Cross-source search covering preprints (PPR), patents, Agricola and CBA as well as PubMed/PMC
  - JATS full text (parsed or raw XML), reference and citation graphs
  - Cross-references to external databases (UniProt, EMBL, PDB, ...)
  - No API key required
- **Modular Architecture**: Tools organized in separate modules for maintainability
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

### Configuration with Claude Desktop

Add to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pubmed": {
      "command": "/path/to/pubmed-client/target/release/pubmed-mcp"
    }
  }
}
```

Using the container image instead (no local Rust toolchain needed):

```json
{
  "mcpServers": {
    "pubmed": {
      "command": "docker",
      "args": ["run", "--rm", "-i", "ghcr.io/illumination-k/pubmed-mcp:latest"]
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
│   └── tools/           # Tools module
│       ├── mod.rs         # PubMedServer definition
│       ├── search.rs      # Search tool implementation
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
