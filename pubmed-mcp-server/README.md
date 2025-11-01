# PubMed MCP Server

Model Context Protocol (MCP) server for searching and retrieving biomedical literature from PubMed and PubMed Central (PMC) databases.

## Overview

This MCP server provides tools for interacting with the PubMed and PMC APIs through the Model Context Protocol, allowing AI assistants like Claude to search and retrieve biomedical research articles.

## Features

- **PubMed Search**: Search the PubMed database for biomedical literature
- Simple, minimal implementation with one tool to start
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

Search PubMed for articles matching a query.

**Parameters:**

- `query` (string, required): Search query using PubMed syntax
  - Examples: `"COVID-19"`, `"cancer[ti] AND therapy[tiab]"`
  - Supports [PubMed field tags](https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags)
- `max_results` (integer, optional): Maximum number of results to return (default: 10, max: 100)

**Example:**

```
Search for recent COVID-19 research (max 5 results)
```

## Development

### Project Structure

```
pubmed-mcp-server/
├── Cargo.toml       # Package configuration
├── src/
│   └── main.rs      # MCP server implementation
└── README.md
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
