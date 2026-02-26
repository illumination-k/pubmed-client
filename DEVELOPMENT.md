# Development Guide

## Prerequisites

- [mise](https://mise.jdx.dev/) – tool version manager
- [Rust](https://rustup.rs/) – installed separately via rustup (not via mise)

## Local Setup (All Environments)

For full local development, create `mise.local.toml` in the project root (gitignored):

```toml
[env]
MISE_ENV = "root,rust,node,python"
```

This makes all tools and tasks available without prefixing every command with `MISE_ENV=...`.
Then run `mise run setup:all` to install everything.

## Quick Start by Area

Install only the tools you need by specifying `MISE_ENV`:

| Area            | Install                                                | Run tests                                        |
| --------------- | ------------------------------------------------------ | ------------------------------------------------ |
| Core Rust       | `MISE_ENV=rust mise install`                           | `MISE_ENV=rust mise run test`                    |
| Python bindings | `MISE_ENV=python mise install && mise run setup:py`    | `MISE_ENV=python mise run py:test`               |
| NAPI (Node.js)  | `MISE_ENV=node mise install && mise run setup:napi`    | `MISE_ENV=node mise run napi:test`               |
| WASM            | `MISE_ENV=node mise install && mise run setup:wasm`    | `MISE_ENV=node mise run wasm:test`               |
| Website         | `MISE_ENV=node mise install && mise run setup:website` | `MISE_ENV=node mise run website:start`           |
| Lint / CI tools | `MISE_ENV=root mise install`                           | `MISE_ENV=root mise run lint:root`               |
| **Everything**  | `mise run setup:all`                                   | `MISE_ENV=root,rust,node,python mise run ci:all` |

## mise Configuration Files

Tools and tasks are split by area using [mise environments](https://mise.jdx.dev/configuration/environments.html):

| File               | Loaded when       | Contains                                                            |
| ------------------ | ----------------- | ------------------------------------------------------------------- |
| `mise.toml`        | always            | hooks, `RUST_LOG`, `setup:*` entry points                           |
| `mise.root.toml`   | `MISE_ENV=root`   | dprint, actionlint, ghalint, zizmor, shfmt, pinact + fmt/lint tasks |
| `mise.rust.toml`   | `MISE_ENV=rust`   | cargo-nextest, cargo-llvm-cov + Rust workspace tasks                |
| `mise.node.toml`   | `MISE_ENV=node`   | node, pnpm + NAPI/WASM/website tasks                                |
| `mise.python.toml` | `MISE_ENV=python` | uv, PYO3 env vars + Python tasks                                    |

## mise Task ↔ CI Workflow Correspondence

| Local command                                    | CI workflow                         |
| ------------------------------------------------ | ----------------------------------- |
| `MISE_ENV=rust mise run ci:core`                 | `core.yml` – Test Suite + Doc Tests |
| `MISE_ENV=rust mise run coverage`                | `core.yml` – Code Coverage          |
| `MISE_ENV=root mise run lint:root`               | `lint.yml` – Lint and Format        |
| `MISE_ENV=python mise run ci:py`                 | `python.yml` – lint + test          |
| `MISE_ENV=python mise run py:coverage`           | `python.yml` – Python Test Coverage |
| `MISE_ENV=node mise run ci:napi`                 | `napi.yml` – lint + test            |
| `MISE_ENV=node mise run ci:wasm`                 | `wasm.yml`                          |
| `mise run ci:mcp`                                | `test-mcp-server.yml`               |
| `MISE_ENV=root,rust,node,python mise run ci:all` | all of the above                    |

## CI Trigger Map

Which CI workflows run when you change a file:

| Changed path                | core |  lint  | docs | napi | python | wasm | mcp |
| --------------------------- | :--: | :----: | :--: | :--: | :----: | :--: | :-: |
| `pubmed-client/src/**`      |  ✅  | always |  ✅  |  ✅  |   ✅   |  ✅  | ✅  |
| `pubmed-client/tests/**`    |  ✅  | always |  —   |  —   |   —    |  —   | ✅  |
| `pubmed-client-napi/**`     |  —   | always |  ✅  |  ✅  |   —    |  —   |  —  |
| `pubmed-client-py/**`       |  —   | always |  ✅  |  —   |   ✅   |  —   |  —  |
| `pubmed-client-wasm/**`     |  —   | always |  —   |  —   |   —    |  ✅  |  —  |
| `pubmed-mcp/**`             |  —   | always |  —   |  —   |   —    |  —   | ✅  |
| `website/**`                |  —   | always |  ✅  |  —   |   —    |  —   |  —  |
| `Cargo.toml` / `Cargo.lock` |  ✅  | always |  ✅  |  ✅  |   ✅   |  ✅  | ✅  |

> **Note**: `lint.yml` always runs on every push/PR (no path filter).

## Release Tag Naming

| Tag pattern | Publishes to                                            |
| ----------- | ------------------------------------------------------- |
| `v*`        | crates.io (`pubmed-client`, `pubmed-cli`, `pubmed-mcp`) |
| `py-v*`     | PyPI (`pubmed-client-py`)                               |
| `node-v*`   | npm (`pubmed-client` NAPI package)                      |
