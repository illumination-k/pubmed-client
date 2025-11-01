# PubMed MCP Server

Model Context Protocol (MCP) server for searching and retrieving biomedical literature from PubMed and PubMed Central (PMC) databases.

## Overview

This MCP server provides tools for interacting with the PubMed and PMC APIs through the Model Context Protocol, allowing AI assistants like Claude to search and retrieve biomedical research articles.

## Features

- **Enhanced PubMed Search**: Search the PubMed database with advanced filtering
  - Filter by study type (RCT, meta-analysis, systematic review, etc.)
  - Filter by text availability (open access, free full text, PMC full text)
  - Support for all PubMed field tags and boolean operators
- **Modular Architecture**: Tools organized in separate modules for maintainability
- Built with [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - the official Rust SDK for MCP
- Uses stdio transport for communication

## Installation

### Building from Source

```bash
# From workspace root
cargo build --release -p pubmed-mcp-server

# The binary will be at:
# target/release/pubmed-mcp-server
```

## Usage

### Running the Server

The server communicates via standard input/output (stdio):

```bash
cargo run -p pubmed-mcp-server
```

### Configuration with Claude Desktop

Add to your Claude Desktop configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pubmed": {
      "command": "/path/to/pubmed-client-rs/target/release/pubmed-mcp-server"
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

## Development

### Project Structure

```
pubmed-mcp-server/
├── Cargo.toml           # Package configuration
├── src/
│   ├── main.rs          # MCP server implementation with tool router
│   └── tools/           # Tools module
│       ├── mod.rs       # PubMedServer definition
│       └── search.rs    # Search tool implementation
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
RUST_LOG=info cargo run -p pubmed-mcp-server
RUST_LOG=debug cargo run -p pubmed-mcp-server  # More verbose
```

## Testing

### Testing with MCP Inspector

The [MCP Inspector](https://github.com/modelcontextprotocol/inspector) is a useful tool for testing MCP servers:

```bash
npx @modelcontextprotocol/inspector cargo run -p pubmed-mcp-server
```

## License

MIT

## References

- [Model Context Protocol](https://modelcontextprotocol.io/)
- [rmcp - Rust SDK for MCP](https://github.com/modelcontextprotocol/rust-sdk)
- [PubMed API Documentation](https://www.ncbi.nlm.nih.gov/books/NBK25499/)
