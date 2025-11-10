# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust workspace containing PubMed and PMC (PubMed Central) API clients with multiple language bindings. The workspace includes a core Rust library, WebAssembly bindings for JavaScript/TypeScript environments, Python bindings via PyO3, a command-line interface for common operations, and a Model Context Protocol (MCP) server for AI assistant integration.

## Package Name

python bindings package name registered in PyPI: `pubmed-client-py`

## Workspace Structure

```
pubmed-client/                    # Cargo workspace root
├── Cargo.toml                       # Workspace definition
├── pubmed-client/                   # Core Rust library
│   ├── Cargo.toml                   # Rust library configuration
│   └── src/                         # Core library source code
├── pubmed-client-wasm/              # WASM bindings for npm
│   ├── Cargo.toml                   # WASM crate configuration
│   ├── package.json                 # npm package configuration
│   ├── src/lib.rs                   # WASM bindings source
│   ├── pkg/                         # Generated WASM package
│   ├── tests/                       # WASM-specific TypeScript tests
│   └── *.json, *.ts                # TypeScript/npm configuration
├── pubmed-client-py/                # Python bindings (PyO3)
│   ├── Cargo.toml                   # PyO3 crate configuration
│   ├── pyproject.toml               # Python package configuration
│   ├── src/lib.rs                   # PyO3 bindings source
│   ├── pubmed_client.pyi            # Type stub file for IDE support
│   ├── py.typed                     # PEP 561 type marker
│   ├── tests/                       # Python tests (pytest)
│   │   ├── test_config.py           # Configuration tests
│   │   ├── test_client.py           # Client initialization tests
│   │   ├── test_models.py           # Data model tests
│   │   ├── test_integration.py      # Integration tests
│   │   └── conftest.py              # Pytest fixtures
│   └── pytest.ini                   # Pytest configuration
├── pubmed-cli/                      # Command-line interface
│   ├── Cargo.toml                   # CLI crate configuration
│   └── src/                         # CLI source code
│       ├── main.rs                  # CLI entry point
│       └── commands/                # CLI subcommands
│           ├── convert.rs           # PMID to PMCID conversion
│           ├── figures.rs           # Figure extraction
│           ├── markdown.rs          # Markdown conversion
│           └── search.rs            # PubMed search
├── pubmed-mcp/               # MCP server for AI assistants
│   ├── Cargo.toml                   # MCP server configuration
│   ├── src/main.rs                  # MCP server implementation
│   ├── tests/                       # Integration tests
│   │   └── integration_test.rs      # MCP protocol tests
│   └── README.md                    # MCP server documentation
└── tests/                           # Shared integration tests
```

## Important Guidelines for Git Operations

### File and Directory Renaming

**CRITICAL**: When renaming files or directories in this git repository, ALWAYS use `git mv` instead of shell commands like `mv`, `rename`, or file system operations.

#### Why use `git mv`?

1. **Preserves Git History**: Git properly tracks the rename, allowing `git log --follow` to show the complete file history
2. **Clean Commit Diffs**: Shows as `renamed:` instead of delete + add, making code review easier
3. **Maintains Blame Information**: File annotations and blame history follow through the rename
4. **Automatic Staging**: The rename is automatically staged for commit

#### Correct Renaming Process:

```bash
# ✅ CORRECT - Use git mv to rename files/directories
git mv old-name new-name

# Then update references in code and documentation
sed -i '' 's/old-name/new-name/g' affected-files

# Stage all changes together
git add -A

# Verify git recognizes the rename
git status  # Should show "renamed: old-name -> new-name"
```

#### Incorrect Renaming Process:

```bash
# ❌ WRONG - Do NOT use shell mv command
mv old-name new-name  # Breaks git history tracking!
```

### General Git Best Practices

- **Always check `git status`** before and after operations
- **Use `git diff --cached`** to review staged changes before committing
- **Test builds and tests** after renaming operations
- **Keep renames and content changes** in the same commit for context

## Important Guidelines for PubMed Search Query Implementation

**CRITICAL**: When implementing or modifying PubMed search query functionality, ALWAYS reference the official NCBI PubMed documentation:

- **Primary Resource**: https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags
- **E-utilities Documentation**: https://www.ncbi.nlm.nih.gov/books/NBK25499/

### Search Field Tag Validation

Before implementing new search field tags or modifying existing ones:

1. **Verify field tags** against the official PubMed help documentation
2. **Use correct tag syntax** - PubMed uses short form tags (e.g., `[ti]` for Title, `[au]` for Author)
3. **Test with real API calls** when possible to ensure compatibility
4. **Document any limitations** or special requirements for field tags

### Currently Validated Field Tags

The following field tags have been verified against NCBI documentation:

- `[ti]` - Title
- `[tiab]` - Title/Abstract
- `[au]` - Author
- `[1au]` - First Author
- `[lastau]` - Last Author
- `[ad]` - Affiliation
- `[ta]` - Journal Title Abbreviation
- `[la]` - Language
- `[pt]` - Publication Type
- `[mh]` - MeSH Terms
- `[majr]` - MeSH Major Topic
- `[sh]` - MeSH Subheading
- `[gr]` - Grant Number
- `[auid]` - Author Identifier (ORCID)
- `[pdat]` - Publication Date
- `[edat]` - Entry Date
- `[mdat]` - Modification Date
- `[sb]` - Subset (e.g., "free full text[sb]")

### Invalid or Non-existent Tags

These tags do NOT exist in PubMed and should not be used:

- `[Organism]` - Use MeSH terms with `[mh]` instead
- `[lang]` - Deprecated, use `[la]`
- Long-form tags like `[Title]`, `[Author]`, etc. - Use short forms

When in doubt, always check the official documentation before implementation.

## Commands

### Build & Development

#### Workspace Commands (from root)

```bash
# Build all workspace members
cargo build

# Test all workspace members
cargo test

# Check all workspace members
cargo check

# Run specific integration test suite (from pubmed-client/ directory)
cd pubmed-client && cargo test --test comprehensive_pmc_tests
cd pubmed-client && cargo test --test comprehensive_pubmed_tests
cd pubmed-client && cargo test --test test_elink_integration

# Run new real API integration tests (requires network and env var, from pubmed-client/)
cd pubmed-client && PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pubmed_api_tests
cd pubmed-client && PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test pmc_api_tests
cd pubmed-client && PUBMED_REAL_API_TESTS=1 cargo test --features integration-tests --test error_handling_tests

# Run single unit test in core library
cargo test --lib -p pubmed-client pubmed::parser::tests::test_mesh_term_parsing
```

#### Core Library Commands (from pubmed-client/)

```bash
# Build only the core library
cargo build

# Test only the core library
cargo test

# Generate documentation
cargo doc --open
```

#### WASM Package Commands (from pubmed-client-wasm/)

```bash
# Build WASM package for Node.js
pnpm run build

# Build for different targets
pnpm run build:web      # For web/browser
pnpm run build:bundler  # For bundlers
pnpm run build:all      # All targets

# Run TypeScript tests
pnpm run test
pnpm run test:watch     # Watch mode
pnpm run test:coverage  # With coverage

# Publish to npm
pnpm run publish        # wasm-pack publish --access public
```

#### Python Package Commands (from pubmed-client-py/)

```bash
# Build Python package with maturin
uv run --with maturin maturin develop        # Development build
uv run --with maturin maturin develop --quiet # Quiet mode

# Run Python tests (all tests)
uv run pytest                                 # Run all tests
uv run pytest -v                              # Verbose output
uv run pytest -m "not integration"            # Unit tests only
uv run pytest -m integration                  # Integration tests only

# Run specific test files
uv run pytest tests/test_config.py            # Configuration tests
uv run pytest tests/test_client.py            # Client tests
uv run pytest tests/test_models.py            # Data model tests
uv run pytest tests/test_integration.py       # Integration tests

# Run tests with coverage
uv run pytest --cov=pubmed_client --cov-report=html
uv run pytest --cov=pubmed_client --cov-report=term

# Type checking with mypy
uv run mypy tests/ --strict                   # Type check tests
uv run mypy tests/ --strict --no-error-summary # Quiet mode

# Code quality checks
uv run ruff check .                           # Linting
uv run ruff format .                          # Formatting

# Install development dependencies
uv sync --group dev                           # Sync all dev dependencies
```

#### CLI Commands (from workspace root)

```bash
# Help and basic usage
cargo run -p pubmed-cli -- --help
cargo run -p pubmed-cli -- <COMMAND>

# Extract figures from PMC articles
cargo run -p pubmed-cli -- figures PMC7906746 [PMC123456...] [--failed-output failed.txt]

# Convert PMC to markdown
cargo run -p pubmed-cli -- markdown PMC7906746

# Convert PMID to PMCID (supports --format json|csv|txt, --batch-size N)
cargo run -p pubmed-cli -- pmid-to-pmcid 31978945 [33515491...] [--format csv] [--batch-size 50]

# With API key, email, and tool name
cargo run -p pubmed-cli -- --api-key KEY --email you@example.com --tool MyApp <COMMAND>

# Debug logging
RUST_LOG=debug cargo run -p pubmed-cli -- <COMMAND>
```

#### mise Commands (if available)

Using `mise` for task management (configured in `.mise.toml`):

```bash
# Build all workspace members
mise r build

# Run all workspace tests with nextest (preferred test runner)
mise r test

# Run tests in watch mode
mise r test:watch

# Run tests with verbose output
mise r test:verbose

# Generate and open documentation
mise r doc

# Check code without building
mise r check

# Core library specific
mise r core:test              # Run core library tests
mise r core:test:integration  # Run integration tests
mise r core:publish           # Publish to crates.io

# WASM package specific
mise r wasm:build            # Build for web target
mise r wasm:build:node       # Build for Node.js target
mise r wasm:test             # Build and run TypeScript tests
mise r wasm:publish          # Publish to npm
```

#### MCP Server Commands (from workspace root)

```bash
# Build MCP server
cargo build --release -p pubmed-mcp

# Run MCP server (stdio transport)
cargo run -p pubmed-mcp

# Run with debug logging
RUST_LOG=debug cargo run -p pubmed-mcp

# Run tests
cargo test -p pubmed-mcp

# Test with MCP Inspector (interactive testing tool)
npx @modelcontextprotocol/inspector cargo run -p pubmed-mcp
```

### Code Quality

#### Rust Code Quality

```bash
# Full linting (dprint + cargo fmt + clippy) for entire workspace
mise r lint

# Format code (dprint + cargo fmt) for entire workspace
mise r fmt
```

#### WASM/TypeScript Code Quality (from pubmed-client-wasm/)

The WASM package uses Biome for TypeScript/JavaScript linting and formatting:

```bash
# Format TypeScript and JavaScript files
pnpm run format

# Run linting
pnpm run lint

# Format and lint (recommended)
pnpm run check

# CI mode (fails on issues, no auto-fix)
pnpm run ci
```

### Code Coverage

```bash
# Generate HTML coverage report and open in browser
mise r coverage:open

# Generate coverage report (HTML format)
mise r coverage

# Generate LCOV format for CI
mise r coverage:lcov

# Generate JSON format for tooling
mise r coverage:json
```

### Debugging & Logging

```bash
# Run tests with tracing output (structured logging)
RUST_LOG=debug cargo test -- --nocapture

# Run specific test with tracing
RUST_LOG=pubmed_client_rs=debug cargo test --test test_integration_abstract -- --nocapture

# Different log levels
RUST_LOG=info cargo test        # Info level and above
RUST_LOG=debug cargo test       # Debug level and above
RUST_LOG=trace cargo test       # All tracing output
```

## Architecture

### Core Library Architecture (pubmed-client/)

- `src/lib.rs` - Main library entry point, re-exports public API and unified `Client`
- `src/pubmed/` - PubMed module directory
  - `client.rs` - PubMed API client for article metadata search and ELink API
  - `parser.rs` - XML parsing module functions for PubMed article metadata (refactored from empty struct)
  - `models.rs` - Data structures for PubMed articles, ELink results
  - `responses.rs` - Internal API response structures for JSON/XML parsing
  - `mod.rs` - Module exports and public API
- `src/pmc/` - PMC module directory
  - `client.rs` - PMC client for full-text article access
  - `parser/` - XML parsing modules for PMC content (refactored to module functions)
    - `mod.rs` - Main parser module with `parse_pmc_xml()` function
    - `author.rs` - Author extraction functions
    - `metadata.rs` - Metadata extraction functions
    - `reference.rs` - Reference extraction functions
    - `section.rs` - Section parsing functions
    - `xml_utils.rs` - XML utility functions
  - `models.rs` - Data structures for PMC articles
  - `markdown.rs` - Converter from PMC XML to Markdown format
- `src/pubmed/query/` - Advanced query builder system with filters, date ranges, boolean logic
  - `builder.rs` - Main SearchQuery builder implementation
  - `filters.rs` - Field-specific search filters (title, author, journal, etc.)
  - `dates.rs` - Date range and publication date filtering
  - `boolean.rs` - Boolean logic operations (AND, OR, NOT)
  - `advanced.rs` - Advanced search features (MeSH terms, article types)
  - `validation.rs` - Query validation and error handling
- `src/error.rs` - Error types and result aliases
- `src/config.rs` - Client configuration options for API keys and rate limiting
- `src/rate_limit.rs` - Rate limiting implementation using token bucket algorithm
- `src/retry.rs` - Retry logic with exponential backoff for network failures

### WASM Bindings Architecture (pubmed-client-wasm/)

The WASM package provides JavaScript/TypeScript bindings for the core Rust library:

- `src/lib.rs` - WASM bindings entry point with JavaScript-compatible types
- `package.json` - npm package configuration for publishing
- `pkg/` - Generated WASM package output (created by wasm-pack)
- `tests/` - TypeScript tests for WASM bindings using Vitest
- Configuration files: `tsconfig.json`, `vitest.config.ts`, `biome.json`

**Key WASM Types:**

- `WasmPubMedClient` - JavaScript wrapper around the core Rust client
- `WasmClientConfig` - JavaScript-friendly configuration object
- `JsArticle`, `JsFullText` - JavaScript-compatible data structures
- Promise-based async API matching modern JavaScript patterns

### Key Types

- `Client` - Combined client that provides both PubMed and PMC functionality
- `PubMedClient` - Handles article metadata searches and ELink API via PubMed E-utilities
- `PmcClient` - Fetches structured full-text from PubMed Central
- `SearchQuery` - Builder pattern for constructing complex search queries with filters
- `PubMedArticle` - Article metadata from PubMed (title, authors, abstract, etc.)
- `PmcFullText` - Structured representation of PMC full-text articles
- `RelatedArticles`, `PmcLinks`, `Citations` - ELink API result types for discovering article relationships
- `ClientConfig` - Configuration for API keys, rate limiting, and client behavior
- `RateLimiter` - Token bucket rate limiter for NCBI API compliance

### API Design Patterns

- Async/await using tokio runtime
- Builder pattern for search queries (`SearchQuery`)
- Result<T> type alias for error handling
- Module functions instead of empty structs for parsers (clean functional design)
- Separation of metadata (PubMed) and full-text (PMC) concerns
- ELink API integration for discovering article relationships (related articles, citations, PMC links)
- Support for custom HTTP clients via reqwest
- Internal response types separate from public API models
- Data-driven testing with real PMC XML samples in `tests/test_data/`
- Structured logging with tracing for debugging and monitoring
- Rate limiting with token bucket algorithm for NCBI API compliance
- Configurable API keys, email, and tool identification for NCBI guidelines

**Parser Design Philosophy:**
The parsers have been refactored from empty structs with static methods to module functions, following Rust's idiomatic patterns. This eliminates unnecessary type definitions while maintaining clear module boundaries and namespacing. Key parser functions include:

- `parse_article_from_xml()` - Main PubMed XML parser
- `parse_pmc_xml()` - Main PMC XML parser
- Module-specific extraction functions (e.g., `extract_authors()`, `extract_references()`)

### Testing

- Test runner: `cargo-nextest` for better output and parallelization
- Parameterized tests using `rstest`
- Test data: Real XML files in `pubmed-client/tests/integration/test_data/`
- Common test utilities in `pubmed-client/tests/integration/common/mod.rs`
- Integration tests with tracing support using `#[traced_test]`
- Mocked rate limiting tests for deterministic behavior
- ELink API integration tests covering all relationship types

#### Test Organization

- **Unit tests**: Located alongside source code in `pubmed-client/src/` modules
- **Integration tests**: Located in `pubmed-client/tests/integration/` directory
  - `comprehensive_pmc_tests.rs` - PMC XML parsing validation
  - `comprehensive_pubmed_tests.rs` - PubMed XML parsing validation
  - `test_elink_integration.rs` - ELink API functionality
  - `test_einfo_integration.rs` - EInfo API functionality
  - `markdown_tests.rs` - Markdown conversion testing
- **Test utilities**: Shared code in `pubmed-client/tests/integration/common/mod.rs`

### Dependencies

- `tokio` - Async runtime
- `tokio-util` - Utilities for tokio, used for rate limiting
- `reqwest` - HTTP client
- `serde` - Serialization/deserialization
- `quick-xml` - XML parsing for PMC content
- `thiserror` - Error type derivation
- `anyhow` - Error handling utilities
- `tracing` - Structured logging and instrumentation
- `urlencoding` - URL encoding for API parameters
- `rstest` - Parameterized testing (dev dependency)
- `tracing-subscriber` - Tracing output formatting (dev dependency)
- `tracing-test` - Tracing support for tests (dev dependency)

### Logging & Debugging

**CRITICAL**: DO NOT use `println!` or `eprintln!` in production code. Use `tracing` macros instead.

```rust
// ❌ AVOID
println!("Processing article {}", pmid);

// ✅ PREFER - Use structured tracing
info!(pmid = %pmid, "Processing article");
```

Allowed uses of `println!`: Documentation examples, README code samples, demo applications only.

## NCBI E-utilities API Support

The library implements multiple NCBI E-utilities APIs for comprehensive biomedical data access:

### ELink API for Article Relationships

1. **Related Articles** (`get_related_articles`) - Find articles related by subject/content
2. **PMC Links** (`get_pmc_links`) - Check PMC full-text availability
3. **Citations** (`get_citations`) - Find articles that cite the given articles

### EInfo API for Database Information

1. **Database List** (`get_database_list`) - Get all available NCBI databases
2. **Database Details** (`get_database_info`) - Get detailed information about specific databases including searchable fields and links

### Implementation Details

- **Rate Limited**: All ELink methods respect the same rate limiting as other API calls
- **Deduplication**: Results automatically remove duplicates and filter out source PMIDs
- **Error Handling**: Comprehensive error handling with tracing integration
- **Empty Input**: Graceful handling of empty PMID lists
- **Batch Processing**: Support for multiple PMIDs in a single request

### ELink Response Processing

The implementation uses internal response types (`ELinkResponse`, `ELinkSet`, `ELinkSetDb`) for JSON parsing, then converts to clean public API types (`RelatedArticles`, `PmcLinks`, `Citations`). This separation ensures API stability while handling NCBI's complex response format.

### Testing ELink Functionality

```bash
# Run ELink-specific integration tests (from pubmed-client/ directory)
cd pubmed-client && cargo test --test test_elink_integration

# Test individual ELink methods (from pubmed-client/ directory)
cd pubmed-client && cargo test test_get_related_articles_integration
cd pubmed-client && cargo test test_get_pmc_links_integration
cd pubmed-client && cargo test test_get_citations_integration
```

## WASM Development and Publishing

The workspace includes a complete npm package for WebAssembly bindings.

### WASM Package Publishing

The WASM package can be automatically published to npm:

```bash
# From pubmed-client-wasm/ directory
pnpm run publish    # Equivalent to: wasm-pack publish --access public
```

The workspace structure ensures:

- Correct package name (`pubmed-client-wasm`) generated automatically
- All npm dependencies and TypeScript configuration contained within the WASM package
- Independent versioning and publishing from the core Rust library

### WASM TypeScript Testing

Comprehensive TypeScript tests using Vitest:

```bash
# From pubmed-client-wasm/ directory
pnpm run test              # Run TypeScript tests
pnpm run test:watch        # Watch mode
pnpm run test:coverage     # With coverage
pnpm run typecheck         # Type checking only
```

**Test Coverage:**

- All WASM exported functions with success/failure scenarios
- Live API tests using real PMIDs (e.g., `31978945` for COVID-19 research)
- Error handling for invalid inputs and network failures
- Promise-based async API validation

## Python Bindings Development and Publishing

The workspace includes Python bindings built with PyO3 and maturin.

### Python Bindings Architecture (pubmed-client-py/)

The Python package provides native Python bindings for the core Rust library:

- `src/lib.rs` - PyO3 bindings entry point with Python-compatible types (1144 lines)
- `pubmed_client.pyi` - Type stub file for IDE autocomplete and mypy support
- `py.typed` - PEP 561 marker for type information
- `pyproject.toml` - Python package configuration (maturin build system)
- `pytest.ini` - Pytest configuration with test markers
- `tests/` - Python tests using pytest
- Configuration files: `pyproject.toml`, `.python-version`, `uv.lock`

**Key Python Types:**

- `Client` - Combined client with both `pubmed` and `pmc` properties
- `PubMedClient` - Python wrapper around Rust PubMed client
- `PmcClient` - Python wrapper around Rust PMC client
- `ClientConfig` - Configuration with builder pattern for method chaining
- `PubMedArticle`, `PmcFullText` - Python-friendly data structures
- `RelatedArticles`, `PmcLinks`, `Citations` - ELink API results
- `DatabaseInfo` - EInfo API database information

**Implementation Details:**

- **Blocking API**: Synchronous Python API using Tokio runtime internally
- **GIL Release**: Uses `py.allow_threads()` for non-blocking I/O operations
- **Runtime Management**: Creates Tokio runtime per call with `get_runtime()`
- **Error Handling**: Converts Rust errors to Python exceptions
- **Arc-based Sharing**: Thread-safe client cloning with `Arc<T>`
- **Builder Pattern**: ClientConfig supports method chaining
- **Recursive Collection**: Figures and tables collected from nested article sections

### Maturin Best Practices for PyO3 Development

**CRITICAL**: When developing Python bindings with maturin and PyO3, follow these best practices:

#### Module Registration

1. **Always add new #[pyclass] types to the #[pymodule] function:**
   ```rust
   #[pymodule]
   fn pubmed_client(m: &Bound<'_, PyModule>) -> PyResult<()> {
       // Add ALL pyclass types here
       m.add_class::<PyYourNewClass>()?;
       Ok(())
   }
   ```

2. **Keep #[pyclass] and #[pymethods] in the same file:**
   - PyO3 requires both to be in the same Rust module
   - Splitting them across files will cause silent export failures

3. **Verify module exports after adding new classes:**
   ```bash
   # Rebuild with maturin
   uv run --with maturin maturin develop

   # Test that class is accessible
   uv run python -c "from pubmed_client import YourNewClass; print('Success!')"

   # Or inspect the module
   uv run python -c "import pubmed_client; print('YourNewClass' in dir(pubmed_client))"
   ```

#### Build and Testing Workflow

1. **Always rebuild after Rust changes:**
   ```bash
   # Development build (faster, with debug symbols)
   uv run --with maturin maturin develop

   # Release build (optimized)
   uv run --with maturin maturin develop --release
   ```

2. **Clean builds when troubleshooting:**
   ```bash
   # Clean Rust artifacts
   cargo clean -p pubmed-client-py

   # Rebuild
   uv run --with maturin maturin develop
   ```

3. **Test immediately after adding new bindings:**
   ```bash
   # Run specific test file
   uv run pytest tests/test_your_feature.py -v

   # Run all tests
   uv run pytest
   ```

#### Type Stubs (.pyi files)

1. **Keep type stubs in sync with Rust implementation:**
   - Update `pubmed_client.pyi` when adding new classes or methods
   - Add new class names to `__all__` list in the stub file

2. **Use covariant types for collections:**
   ```python
   from collections.abc import Sequence

   # ✅ Good - accepts both list[str] and list[str | None]
   def terms(self, terms: Sequence[str | None] | None) -> SearchQuery: ...

   # ❌ Bad - list is invariant, only accepts exact type
   def terms(self, terms: list[str | None] | None) -> SearchQuery: ...
   ```

3. **Remove docstrings from .pyi files:**
   - Type stub files should only contain type signatures
   - Docstrings belong in the Rust source code

#### Common Pitfalls

1. **Module name confusion:**
   - The `module-name` in `pyproject.toml` creates a nested structure
   - For `module-name = "pubmed_client_py.pubmed_client"`:
     - The .so file is `pubmed_client.cpython-312-*.so`
     - Python imports via `from pubmed_client import Class`

2. **Type mismatches:**
   - Python int → Rust usize: Negative values cause OverflowError
   - Use appropriate error handling in tests
   - PyO3 type validation happens before your Rust code runs

3. **Silent export failures:**
   - If a class compiles but doesn't export, check:
     - Is it in `m.add_class::<YourClass>()?`?
     - Are #[pyclass] and #[pymethods] in the same file?
     - Did you rebuild with maturin develop?

4. **Cache issues:**
   - Python caches imported modules
   - Restart Python interpreter after rebuilding
   - Or use `importlib.reload()` for iterative development

#### Debugging Import Issues

If a class doesn't appear in Python after adding it:

```bash
# 1. Check if it's in the compiled .so module
uv run python << 'EOF'
import pubmed_client.pubmed_client as so
print("YourClass" in dir(so))
print(so.__all__ if hasattr(so, "__all__") else "No __all__")
EOF

# 2. Check if it's exported from the package
uv run python -c "import pubmed_client; print('YourClass' in dir(pubmed_client))"

# 3. Try direct import
uv run python -c "from pubmed_client import YourClass; print(YourClass)"
```

If the class is in the .so but not in the package:
- Check `__all__` list in type stubs
- Verify __init__.py has `from .pubmed_client import *`
- Try uninstalling and reinstalling: `uv pip uninstall pubmed-client-py && uv run --with maturin maturin develop`

### Python Testing Strategy

Comprehensive test suite with 35 tests across config, client, models, and integration tests.

**Running Tests:**

```bash
uv run pytest                              # All tests
uv run pytest -m "not integration"         # Unit tests only
uv run pytest -m integration               # Integration tests only
uv run pytest --cov=pubmed_client --cov-report=html  # With coverage
```

### Type Stubs and IDE Support

The Python package includes comprehensive type annotations (`pubmed_client.pyi`) with full IDE autocomplete and mypy support.

```bash
uv run mypy tests/ --strict  # Type checking
```

### Python Package Publishing

```bash
# Build and publish to PyPI (from pubmed-client-py/)
uv run --with maturin maturin build --release
uv run --with maturin maturin publish  # Requires PYPI_TOKEN
```

### Python Code Quality

```bash
# Linting and formatting (from pubmed-client-py/)
uv run ruff check .        # Linting
uv run ruff format .       # Formatting
uv run mypy tests/ --strict  # Type checking
```

## MCP Server for AI Assistants

The workspace includes a Model Context Protocol (MCP) server that allows AI assistants like Claude to interact with PubMed and PMC APIs.

### MCP Server Overview

The MCP server (`pubmed-mcp`) provides a standardized interface for AI assistants to:

- Search PubMed for biomedical literature
- Retrieve article metadata and abstracts
- Access full-text articles from PMC (future)

**Key Features:**

- Built with [rmcp](https://github.com/modelcontextprotocol/rust-sdk) - Official Rust SDK for MCP
- Uses stdio transport for communication
- JSON-RPC 2.0 protocol implementation
- Automatic JSON schema generation for tool parameters
- Integration with Claude Desktop and other MCP clients

### MCP Server Architecture (pubmed-mcp/)

```
pubmed-mcp/
├── Cargo.toml              # Package configuration
├── src/
│   └── main.rs             # MCP server implementation (119 lines)
├── tests/
│   └── integration_test.rs # MCP protocol integration tests
└── README.md               # MCP-specific documentation
```

**Implementation Details:**

- `PubMedServer` struct - Main server implementation with tool router
- Tool definitions using `#[tool]` macro from rmcp
- Automatic parameter validation with JSON schema
- Structured logging to stderr (doesn't interfere with stdio protocol)
- Protocol version: `V_2024_11_05`

### Available MCP Tools

#### `search_pubmed`

Search PubMed for articles matching a query.

**Parameters:**

- `query` (string, required): Search query using PubMed syntax
  - Examples: `"COVID-19"`, `"cancer[ti] AND therapy[tiab]"`
  - Supports all [PubMed field tags](https://pubmed.ncbi.nlm.nih.gov/help/#using-search-field-tags)
- `max_results` (integer, optional): Maximum number of results (default: 10, max: 100)

**Returns:**

- Formatted text with article titles and PMIDs
- Article count and numbered list

**Example Usage (from Claude Desktop):**

```
User: Search for recent COVID-19 vaccine research (max 5 results)

Claude uses tool: search_pubmed
  query: "COVID-19 vaccine[ti]"
  max_results: 5

Result: Found 5 articles:
1. COVID-19 vaccine effectiveness... (PMID: 12345678)
2. mRNA vaccines for COVID-19... (PMID: 23456789)
...
```

### Claude Desktop Integration

To use the MCP server with Claude Desktop, add it to your configuration file:

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

After restarting Claude Desktop, the server will be available with the search_pubmed tool.

### MCP Server Development

#### Adding New Tools

To add additional MCP tools, add methods to the `PubMedServer` impl block:

```rust
#[tool(description = "Fetch full-text article from PMC")]
async fn fetch_pmc_article(
    &self,
    Parameters(params): Parameters<PmcRequest>,
) -> Result<CallToolResult, ErrorData> {
    // Implementation using self.client.pmc
    let article = self.client.pmc.fetch_full_text(&params.pmc_id).await
        .map_err(|e| ErrorData { /* error conversion */ })?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}
```

The `#[tool]` macro automatically:

- Registers the tool with the MCP server
- Generates JSON schema from parameter types
- Handles tool routing and parameter validation

#### MCP Testing

The MCP server includes comprehensive integration tests:

```bash
# Run MCP protocol tests
cargo test -p pubmed-mcp

# Test server initialization
cargo test -p pubmed-mcp test_mcp_server_initialize

# Test tool listing
cargo test -p pubmed-mcp test_mcp_server_list_tools

# Test server capabilities
cargo test -p pubmed-mcp test_mcp_server_capabilities
```

**Test Coverage:**

- Server initialization and peer info verification
- Tool registration and listing
- Server capabilities and protocol version
- Tool execution with real PubMed API (requires network)

#### MCP Inspector for Interactive Testing

The [MCP Inspector](https://github.com/modelcontextprotocol/inspector) provides an interactive UI for testing MCP servers:

```bash
# Launch MCP Inspector with the server
npx @modelcontextprotocol/inspector cargo run -p pubmed-mcp

# Inspector opens in browser, allowing:
# - Interactive tool testing
# - Parameter input with schema validation
# - Response inspection
# - Server capability exploration
```

### MCP Dependencies

- **rmcp**: Official Rust SDK for Model Context Protocol (v0.8)
  - Features: `server`, `transport-io` (runtime), `client`, `transport-child-process` (tests)
- **schemars**: JSON schema generation for tool parameters (v0.8)
- **tokio**: Async runtime with full feature set
- **tracing-subscriber**: Structured logging to stderr
- **pubmed-client**: Core library for PubMed/PMC API access

### MCP Protocol Details

**Transport**: stdio (stdin/stdout)

- Server reads JSON-RPC requests from stdin
- Server writes JSON-RPC responses to stdout
- Logging goes to stderr to avoid protocol interference

**Protocol Version**: `V_2024_11_05`

**Capabilities**:

- `tools`: Server provides searchable tools
- Tool listing with `tools/list`
- Tool execution with `tools/call`

**Message Format**: JSON-RPC 2.0

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "search_pubmed",
    "arguments": {
      "query": "COVID-19",
      "max_results": 10
    }
  }
}
```

## Test Fixtures and Data

### Downloading Test Data

The project includes scripts to download real XML responses for comprehensive integration testing:

```bash
# Download PMC XML test fixtures (already included)
./scripts/download_all_verified_xml.sh

# Download PubMed XML test fixtures
./scripts/download_pubmed_xml.sh
```

### Test Data Structure

```
tests/integration/test_data/
├── pmc_xml/         # PMC full-text XML files (18 files)
│   ├── PMC10000000.xml
│   ├── PMC10618641.xml
│   └── ...
└── pubmed_xml/      # PubMed article XML files (15 files)
    ├── 25760099.xml  # CRISPR-Cas9 research
    ├── 31978945.xml  # COVID-19 research
    ├── 33515491.xml  # Cancer treatment
    └── ...
```

### Comprehensive Integration Tests

The test suites provide extensive coverage of XML parsing and content analysis:

- **PMC Tests**: `cargo test --test comprehensive_pmc_tests`
  - XML parsing validation
  - Content structure analysis
  - Author and metadata extraction
  - Figures and tables processing

- **PubMed Tests**: `cargo test --test comprehensive_pubmed_tests`
  - Article metadata validation
  - MeSH term extraction and analysis
  - Abstract content analysis
  - Author details and affiliations
  - Chemical substances parsing
  - Article type distribution

Both test suites include statistical analysis and success rate validation to ensure robust parsing across diverse article types and content structures.

## Active Technologies

- Python 3.12+ (bindings), Rust 1.75+ (core library)\ + PyO3 0.21+ (Rust-Python bindings), maturin 1.x (build system), pubmed-client-rs (core Rust library)\ (001-query-builder-python)
- N/A (stateless query builder)\ (001-query-builder-python)

## Recent Changes

- 001-query-builder-python: Added Python 3.12+ (bindings), Rust 1.75+ (core library)\ + PyO3 0.21+ (Rust-Python bindings), maturin 1.x (build system), pubmed-client-rs (core Rust library)\
