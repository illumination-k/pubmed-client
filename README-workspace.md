# PubMed Client Workspace

A Rust workspace for PubMed and PMC API clients with multiple language bindings.

## Workspace Structure

```
pubmed-client-rs/
├── pubmed-client/          # Core Rust library
├── pubmed-client-wasm/     # WebAssembly bindings for Node.js/Browser
└── tests/                  # Integration tests
```

## Packages

### `pubmed-client` - Core Rust Library

The main Rust library providing async interfaces for PubMed and PMC APIs.

**Features:**

- PubMed article search and metadata retrieval
- PMC full-text article access
- Rate limiting compliance with NCBI guidelines
- Markdown export for PMC articles
- Advanced query building with filters

**Usage:**

```bash
cd pubmed-client
cargo build
cargo test
```

### `pubmed-client-wasm` - WebAssembly Bindings

WebAssembly bindings for use in Node.js and browsers.

**Features:**

- JavaScript/TypeScript compatible API
- Promise-based async interface
- Type definitions included
- NPM package ready

**Usage:**

```bash
cd pubmed-client-wasm
pnpm install
pnpm run build
pnpm run publish
```

## Development

### Building All Packages

```bash
# Build Rust packages
cargo build

# Build WASM package
cd pubmed-client-wasm && pnpm run build
```

### Testing

```bash
# Run Rust tests
cargo test

# Run WASM tests
cd pubmed-client-wasm && pnpm run test
```

### Publishing

#### Rust Crate to crates.io

```bash
cd pubmed-client
cargo publish
```

#### WASM Package to npm

```bash
cd pubmed-client-wasm
pnpm run publish
```

## Future Expansions

This workspace is designed to support additional language bindings:

- **Python bindings** (using Maturin)
- **Go bindings** (using cgo)
- **C/C++ bindings** (using cbindgen)

Each binding will be a separate workspace member sharing the core `pubmed-client` library.
