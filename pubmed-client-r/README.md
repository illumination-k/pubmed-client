# pubmedclient (R bindings)

R bindings for the Rust [`pubmed-client`](https://github.com/illumination-k/pubmed-client)
library, built with [extendr](https://extendr.github.io/). This is an **MVP**:
it covers the core PubMed/PMC operations (search, fetch metadata, full text,
Markdown) plus Europe PMC (cross-source search, full text, citation graphs,
database links). The richer surface available in the Python/Node bindings can
be added incrementally.

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

### Europe PMC

[Europe PMC](https://europepmc.org) is a complementary index to the NCBI
E-utilities: it covers preprints (`PPR`), patents (`PAT`), Agricola (`AGR`) and
Chinese Biological Abstracts (`CBA`) as well as PubMed (`MED`) and PMC, and
needs no API key.

Records are addressed by a source database plus an id. Every function takes
both; `source` may be `NULL`, in which case a `PMC`-prefixed id is read as a PMC
record and anything else as a PubMed record. An id given in fully-qualified
`"SOURCE/ID"` form wins over `source`.

```r
# Cross-source search, including preprints
results <- europepmc_search(client, "TITLE:CRISPR AND SRC:PPR", limit = 5)
results[[1]]$europe_pmc_id
results[[1]]$title

# Full text, as summary metadata or as raw JATS XML
info <- europepmc_fulltext(client, "PMC3258128")
xml  <- europepmc_fulltext_xml(client, "PMC3258128")

# Citation graph in both directions — broader than PubMed's own links, which
# see only PubMed-indexed articles
refs <- europepmc_references(client, "PMC3258128")
cits <- europepmc_citations(client, "33515491", source = "MED")

# Cross-references to external databases (UniProt, EMBL, PDB, ...)
links <- europepmc_database_links(client, "PMC3258128")
```

`result_type = "core"` returns far more fields than are modelled, and the set
changes over time; whatever is not named on the record is available as a JSON
object string in its `extra_json`, ready for `jsonlite::fromJSON()`.

Note that `europepmc_fulltext()` and `europepmc_fulltext_xml()` need
`pubmed-client` 0.3.2 or later: 0.3.1 built the full-text URL with a
source-qualified path, which Europe PMC answers with 404. The version
requirement in `src/rust/Cargo.toml` picks up 0.3.2 automatically once it is
published; the rest of the Europe PMC surface works against 0.3.1.

## Configuration

`pubmed_client()` accepts optional `api_key`, `email`, `tool`, `rate_limit`,
and `timeout_seconds`. An NCBI API key raises the rate limit from 3 to 10
requests/second.

## Development

The Rust source lives in `src/rust/`. This crate is intentionally **excluded
from the workspace Cargo build** (an empty `[workspace]` table in its
`Cargo.toml`) because linking requires the R toolchain (`libR`).

It depends on the **published `pubmed-client` crate** (crates.io), not a
workspace-relative path: `R CMD check` and source-tarball installs build the
package in an isolated copy, where a relative path outside the package cannot
resolve. To develop against unpublished `pubmed-client` changes, temporarily set
`pubmed-client = { path = "../../../pubmed-client" }` in `src/rust/Cargo.toml`
and install with `R CMD INSTALL pubmed-client-r` (in-place, no tarball).

After editing `src/rust/src/lib.rs`, regenerate the R wrappers and docs:

```r
rextendr::document("pubmed-client-r")
```

Keep `R/extendr-wrappers.R` in sync with the `extendr_module!` block in
`src/rust/src/lib.rs`.

Errors returned by the Rust side currently surface in R as
`"User function panicked: <fn>"` rather than carrying their own message:
extendr only converts a `Result` into an R condition under its
`result_condition` feature, which this crate does not enable. The offline tests
therefore assert that an error is raised without matching on the message.
