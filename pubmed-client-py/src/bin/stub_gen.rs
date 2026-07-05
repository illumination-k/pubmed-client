//! Type-stub generator for the `pubmed_client` Python module.
//!
//! Run with `cargo run --bin stub_gen` (or `mise run py:stubgen`). It regenerates
//! `pubmed_client.pyi` from the `#[gen_stub_*]` annotations in the crate, then
//! splices in the few module members that `pyo3-stub-gen` cannot introspect:
//! the `__version__` attribute and the `create_exception!` hierarchy (both are
//! registered via `m.add(...)` in `src/lib.rs`, which the macro machinery never
//! sees). CI regenerates and runs `git diff --exit-code` plus `stubtest`, so the
//! checked-in stub always matches the compiled module.

use std::fs;
use std::path::Path;

use pyo3_stub_gen::Result;

/// Names that exist at runtime but are invisible to `pyo3-stub-gen` because they
/// are added with `m.add(...)` / `create_exception!` rather than
/// `#[gen_stub_pyclass]`. Keep in sync with `src/lib.rs` and `src/utils.rs`.
const EXTRA_ALL: &[&str] = &[
    "__version__",
    "PubMedException",
    "ParseException",
    "RequestException",
    "InvalidQueryException",
    "RateLimitException",
    "ApiException",
    "SearchLimitException",
    "HistorySessionException",
];

/// Definitions spliced onto the end of the generated stub. These mirror the
/// module attributes and exception hierarchy declared in `src/lib.rs` (the
/// `#[pymodule]` body) and `src/utils.rs` (`create_exception!`). Keep them in
/// sync; `stubtest` in CI fails if they drift from the compiled module.
const EXTRA_DEFS: &str = r#"
# --- Manually maintained (spliced in by `stub_gen.rs`) ------------------------
# `pyo3-stub-gen` cannot introspect `m.add(...)` / `create_exception!`, so the
# module version and the exception hierarchy are declared here. Keep in sync
# with `src/lib.rs` and `src/utils.rs`; CI verifies via `stubtest`.

__version__: builtins.str

class PubMedException(builtins.Exception):
    r"""Base exception for all PubMed client errors."""

class ParseException(PubMedException):
    r"""XML or JSON parsing failed."""

class RequestException(PubMedException):
    r"""HTTP request failed (network, timeout, DNS)."""

class InvalidQueryException(PubMedException):
    r"""Invalid query structure or parameters."""

class RateLimitException(PubMedException):
    r"""API rate limit exceeded (HTTP 429)."""

class ApiException(PubMedException):
    r"""API returned an error HTTP status code."""

class SearchLimitException(PubMedException):
    r"""Requested result count exceeds the maximum retrievable limit."""

class HistorySessionException(PubMedException):
    r"""History server session expired or WebEnv unavailable."""
"#;

fn main() -> Result<()> {
    let stub = pubmed_client_py::stub_info()?;
    stub.generate()?;
    postprocess()?;
    Ok(())
}

/// Splice the hand-maintained members into the freshly generated stub.
fn postprocess() -> Result<()> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("pubmed_client.pyi");
    let content = fs::read_to_string(&path)?;
    let content = merge_all(&content);
    let content = format!("{}\n{}", content.trim_end(), EXTRA_DEFS);
    fs::write(&path, strip_trailing_whitespace(&content))?;
    Ok(())
}

/// Strip trailing whitespace from every line and end with a single newline.
///
/// `pyo3-stub-gen` emits indented blank lines inside docstrings; the repo's
/// `trailing-whitespace` pre-commit hook would otherwise rewrite the generated
/// file, making the CI `git diff --exit-code` check fail on a clean checkout.
fn strip_trailing_whitespace(content: &str) -> String {
    let mut out: String = content
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    out.push('\n');
    out
}

/// Merge [`EXTRA_ALL`] into the generated `__all__` list, keeping the entries
/// sorted so the output is deterministic across regenerations.
fn merge_all(content: &str) -> String {
    let Some(start) = content.find("__all__ = [") else {
        return content.to_string();
    };
    let Some(rel_end) = content[start..].find(']') else {
        return content.to_string();
    };
    let end = start + rel_end + 1;

    let mut names: Vec<String> = content[start..end]
        .lines()
        .filter_map(|line| {
            let token = line.trim().trim_end_matches(',');
            token
                .strip_prefix('"')
                .and_then(|t| t.strip_suffix('"'))
                .map(str::to_string)
        })
        .collect();
    names.extend(EXTRA_ALL.iter().map(|s| (*s).to_string()));
    names.sort();
    names.dedup();

    let mut rebuilt = String::from("__all__ = [\n");
    for name in &names {
        rebuilt.push_str(&format!("    \"{name}\",\n"));
    }
    rebuilt.push(']');

    format!("{}{}{}", &content[..start], rebuilt, &content[end..])
}
