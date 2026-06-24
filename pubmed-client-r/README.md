# pubmedclient (R bindings)

R bindings for the Rust [`pubmed-client`](https://github.com/illumination-k/pubmed-client)
library, built with [extendr](https://extendr.github.io/). This is an **MVP**:
it covers the core PubMed/PMC operations (search, fetch metadata, full text,
Markdown). The richer surface available in the Python/Node bindings can be
added incrementally.

## Requirements

- R (>= 4.2)
- A Rust toolchain (`cargo`, `rustc`) — see <https://rustup.rs/>

The package compiles a Rust static library at install time, so the toolchain is
required to build it (it is listed in `SystemRequirements`).

## Installation

From the repository root:

```r
# install.packages("remotes")
remotes::install_local("pubmed-client-r")
```

Or with `R CMD INSTALL` from a shell:

```bash
R CMD INSTALL pubmed-client-r
```

## Usage

```r
library(pubmedclient)

client <- pubmed_client(email = "you@example.com")

# Search -> character vector of PMIDs
pmids <- pubmed_search(client, "crispr gene editing", limit = 5)

# Fetch metadata (single PMID -> named list; multiple -> list of lists)
article <- pubmed_fetch(client, pmids[1])
article$title
article$authors

# Search and fetch in one call
articles <- pubmed_search_and_fetch(client, "covid-19", limit = 3)

# PMC full text
info <- pmc_fulltext(client, "PMC7906746")
md   <- pmc_to_markdown(client, "PMC7906746")
cat(md)
```

## Configuration

`pubmed_client()` accepts optional `api_key`, `email`, `tool`, `rate_limit`,
and `timeout_seconds`. An NCBI API key raises the rate limit from 3 to 10
requests/second.

## Development

The Rust source lives in `src/rust/`. This crate is intentionally **excluded
from the workspace Cargo build** (an empty `[workspace]` table in its
`Cargo.toml`) because linking requires the R toolchain (`libR`).

After editing `src/rust/src/lib.rs`, regenerate the R wrappers and docs:

```r
rextendr::document("pubmed-client-r")
```

Keep `R/extendr-wrappers.R` in sync with the `extendr_module!` block in
`src/rust/src/lib.rs`.
