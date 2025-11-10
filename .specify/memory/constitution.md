<!--
Sync Impact Report
==================
Version: 1.0.0 (initial constitution)
Modified Principles: N/A (initial version)
Added Sections: All sections (initial version)
Removed Sections: None

Templates Requiring Updates:
- ✅ plan-template.md: Constitution Check section aligned
- ✅ spec-template.md: Requirements sections aligned
- ✅ tasks-template.md: Testing and multi-language task patterns aligned

Follow-up TODOs: None
-->

# PubMed Client Constitution

## Core Principles

### I. Multi-Language API Client Design

The core Rust library serves as the single source of truth for all functionality. Every feature MUST be accessible through language bindings while maintaining language-idiomatic patterns.

**Non-negotiable rules:**

- Core library (pubmed-client) contains all business logic and API implementations
- Language bindings (WASM, Python) provide idiomatic wrappers without reimplementing logic
- Each binding MUST maintain language conventions (async/Promise for WASM, blocking with GIL release for Python)
- Public API stability: internal response types separate from public models
- Builder patterns for complex operations (SearchQuery, ClientConfig)

**Rationale**: A single Rust core with multiple bindings ensures consistency, reduces duplication, and leverages Rust's safety guarantees while serving multiple language ecosystems.

### II. Testing Excellence (NON-NEGOTIABLE)

Multi-layer testing is mandatory for all features. Tests MUST exist at all appropriate levels before code is considered complete.

**Non-negotiable rules:**

- **Unit tests**: Alongside source code in module files
- **Integration tests**: Real XML fixtures from NCBI APIs in `tests/integration/test_data/`
- **Binding tests**: Language-specific tests (TypeScript/Vitest for WASM, pytest for Python)
- **Data-driven testing**: Use real API responses, not mocked data where possible
- **Test runners**: cargo-nextest (Rust), pytest (Python), vitest (TypeScript)
- **Property testing**: Use rstest for parameterized tests
- **Coverage tracking**: Maintain test coverage with llvm-cov

**Rationale**: NCBI APIs are complex and evolving. Comprehensive testing with real data ensures parsing robustness and prevents regressions across multiple language interfaces.

### III. Type Safety & API Design

Strong typing and explicit error handling are fundamental. APIs MUST use Rust's type system to prevent misuse and provide clear contracts.

**Non-negotiable rules:**

- All public functions return `Result<T, Error>` for fallible operations
- Use newtype patterns where appropriate (e.g., PMID, PMCID wrappers)
- Builder pattern for complex constructions (SearchQuery with method chaining)
- Separation of concerns: internal response types vs. public API models
- Type stubs for Python (`.pyi` files), TypeScript definitions for WASM
- No empty structs with static methods - use module functions

**Rationale**: Type safety prevents entire classes of bugs at compile time. Explicit error handling forces consideration of failure modes. Clean API design reduces cognitive load and improves maintainability.

### IV. NCBI API Compliance

All interactions with NCBI E-utilities MUST comply with NCBI guidelines and rate limits.

**Non-negotiable rules:**

- **Rate limiting**: 3 requests/second without API key, 10 requests/second with API key
- **Token bucket algorithm**: Implemented with tokio-util for accurate rate limiting
- **Retry logic**: Exponential backoff for transient failures (network, 5xx, 429)
- **Identification**: API key, email, and tool name configuration
- **Field tag validation**: All search field tags MUST be verified against official NCBI documentation before implementation
- **Documentation reference**: Always cite NCBI E-utilities docs when implementing API features

**Rationale**: NCBI provides critical public infrastructure. Compliance ensures continued access and prevents service degradation for the entire research community.

### V. Structured Logging & Observability

Production code MUST use structured logging. Debug information must be traceable without polluting standard output.

**Non-negotiable rules:**

- **Use `tracing` crate**: All logging via tracing macros (info!, debug!, error!, trace!)
- **NEVER use println!/eprintln!** in production code (exceptions: documentation examples, demo apps only)
- **Structured context**: Include relevant fields (pmid, query, operation) in log events
- **Log levels**: RUST_LOG environment variable controls output verbosity
- **Test tracing**: Use `#[traced_test]` for integration tests requiring log output

**Rationale**: Structured logging enables debugging in production without modifying code. println! bypasses the logging infrastructure and makes debugging harder. Tracing provides context-aware observability.

### VI. Parser Design Philosophy

Parsers MUST use idiomatic Rust patterns: module functions over empty structs.

**Non-negotiable rules:**

- Use module functions (e.g., `parse_article_from_xml()`) not empty structs with static methods
- Clear separation: parsers in `parser.rs` or `parser/` module directories
- XML utilities isolated in dedicated modules
- Extraction functions clearly named (e.g., `extract_authors()`, `extract_references()`)
- Test with real XML fixtures, not synthetic examples

**Rationale**: Empty structs with static methods are Java-isms that add no value in Rust. Module functions are cleaner, more idiomatic, and reduce unnecessary type definitions.

## Development Standards

### Code Quality Requirements

**Rust**:

- **Formatting**: `cargo fmt` + `dprint` (enforced)
- **Linting**: `cargo clippy` with strict lints
- **Type checking**: All code MUST pass `cargo check` without warnings

**Python (pubmed-client-py/)**:

- **Formatting**: `ruff format` (enforced)
- **Linting**: `ruff check` (enforced)
- **Type checking**: `mypy --strict` on tests (enforced)

**TypeScript (pubmed-client-wasm/)**:

- **Formatting**: Biome formatter (enforced)
- **Linting**: Biome linter (enforced)
- **Type checking**: TypeScript strict mode enabled

### Documentation Requirements

- **API documentation**: All public functions, types, and modules MUST have doc comments
- **Examples**: Public APIs MUST include usage examples in doc comments
- **CLAUDE.md**: Runtime guidance for AI assistants (architecture, commands, guidelines)
- **README.md**: User-facing documentation with quickstart examples
- **Type stubs**: Python `.pyi` files with complete type information

### Performance & Resource Management

- **Async runtime**: Use tokio for all async operations
- **Resource cleanup**: Implement Drop for resources requiring cleanup
- **Memory efficiency**: Avoid unnecessary clones; use references and Arc<T> appropriately
- **Rate limiter efficiency**: Share rate limiter instances via Arc<RateLimiter>

## Development Workflow

### Git Operations

**File and Directory Renaming**:

- **ALWAYS use `git mv`** for renaming files/directories
- NEVER use shell `mv` command (breaks git history tracking)
- Verify renames with `git status` (should show "renamed:")

**Commit Practices**:

- **Test before committing**: All tests MUST pass
- **Conventional commits**: Use conventional commit format
- **Atomic commits**: One logical change per commit
- **Branch naming**: `claude/feature-name-<session-id>` for AI-assisted development

### Build & Test Workflow

**Workspace Commands** (from repository root):

```bash
cargo build              # Build all workspace members
cargo test               # Test all workspace members
mise run test            # Use nextest (preferred)
mise run coverage:open   # Generate and view coverage
```

**Package-Specific Commands**:

```bash
# Core library (pubmed-client/)
cargo test --lib                                    # Unit tests
cargo test --test comprehensive_pubmed_tests        # Integration tests

# Python bindings (pubmed-client-py/)
uv run pytest                                       # All tests
uv run pytest -m "not integration"                  # Unit tests only
uv run mypy tests/ --strict                         # Type checking

# WASM bindings (pubmed-client-wasm/)
pnpm run test                                       # TypeScript tests
pnpm run build                                      # Build WASM package

# CLI (pubmed-cli/)
cargo run -p pubmed-cli -- search "query"           # Run CLI command

# MCP Server (pubmed-mcp/)
cargo test -p pubmed-mcp                            # Test MCP server
cargo run -p pubmed-mcp                             # Run MCP server
```

### Multi-Language Coordination

When adding features:

1. **Core first**: Implement in pubmed-client/ with Rust tests
2. **Bindings second**: Add to WASM and Python with language-specific tests
3. **CLI/MCP third**: Expose via command-line tools if appropriate
4. **Documentation last**: Update all relevant docs (README, CLAUDE.md, API docs)

## Governance

### Amendment Process

This constitution supersedes all other development practices and guidelines. Amendments require:

1. **Documentation**: Proposed changes documented with rationale
2. **Version bump**: Semantic versioning (MAJOR: breaking governance, MINOR: new principle/section, PATCH: clarifications)
3. **Template sync**: All dependent templates updated (plan, spec, tasks, commands)
4. **Sync impact report**: Changes documented in HTML comment at top of this file

### Compliance & Review

**All code reviews MUST verify**:

- Tests exist and pass at all appropriate levels
- Tracing used instead of println! in production code
- Type safety maintained (no unsafe without explicit justification)
- NCBI API guidelines respected (rate limiting, field tag validation)
- Multi-language compatibility preserved
- Documentation updated

**Complexity Justification**:

- Any deviation from constitutional principles MUST be explicitly justified in implementation plans
- Alternative simpler approaches MUST be documented and reasons for rejection explained

**Runtime Guidance**:

- Developers and AI assistants MUST consult `CLAUDE.md` for implementation patterns, commands, architecture details, and critical guidelines
- `CLAUDE.md` provides operational guidance; this constitution provides governing principles

**Version**: 1.0.0 | **Ratified**: 2025-11-05 | **Last Amended**: 2025-11-05
