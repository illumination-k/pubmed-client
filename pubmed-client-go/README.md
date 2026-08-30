# pubmed-client-go

Go bindings for [pubmed-client](https://github.com/illumination-k/pubmed-client) — an async
Rust client for the PubMed and PMC (PubMed Central) APIs.

The bindings wrap the Rust crate through cgo. Calls are synchronous but cancellable: a shared
Tokio runtime drives the async client and blocks until the request completes or the caller's
`context.Context` is done.

The surface covers PubMed search and metadata (ESearch, EFetch, ESummary), the discovery APIs
(ELink, EInfo, EGQuery, ECitMatch, ESpell), PMC full text, XML, Markdown and Open Access
downloads, a query builder, and citation export.

## Requirements

- Go 1.23+ with cgo enabled (a C toolchain must be on `PATH`)
- Rust 1.93+ and `cargo`, to build the static library

TLS is pure-Rust (rustls + ring), so **no OpenSSL/libssl is needed** at build or run time.

## Building

The package links against a static archive produced by the Rust crate in [`rust/`](rust/). It is
about 75 MB, so it is built from source rather than committed:

```bash
MISE_ENV=go mise run go:build   # cargo build --release -p pubmed-client-go → lib/$GOOS_$GOARCH/
cd pubmed-client-go && go build ./...
```

`go:build` must be re-run after any change to the Rust crates. Every other Go task depends on it,
so `go:test` and `lint:go` build the archive for you.

Because a `go get` of this module will not build the archive for you, depend on it from a checkout
with a `replace` directive:

```
require github.com/illumination-k/pubmed-client/pubmed-client-go v0.0.0
replace github.com/illumination-k/pubmed-client/pubmed-client-go => ../path/to/pubmed-client/pubmed-client-go
```

To link an archive kept elsewhere, override the link line:

```bash
CGO_LDFLAGS="-L/opt/pubmed/lib -lpubmed_client_go" go build ./...
```

### Platform support

Linux and macOS (amd64 and arm64) are built and tested in CI. Windows link flags are present but
**untested**: cgo uses the mingw toolchain there, so the archive has to be built for
`x86_64-pc-windows-gnu` rather than Cargo's default msvc target, which produces an incompatible
`.lib`.

## Usage

```go
package main

import (
	"context"
	"fmt"
	"log"

	pubmedclient "github.com/illumination-k/pubmed-client/pubmed-client-go"
)

func main() {
	client, err := pubmedclient.New(&pubmedclient.Config{
		Email: "you@example.com",
		Tool:  "my-app",
	})
	if err != nil {
		log.Fatal(err)
	}
	defer client.Close()

	articles, err := client.SearchAndFetch(context.Background(), "CRISPR gene editing", 5)
	if err != nil {
		log.Fatal(err)
	}
	for _, article := range articles {
		fmt.Printf("%s: %s (%s)\n", article.PMID, article.Title, article.Journal)
	}
}
```

Fuller examples live in [`examples/basic`](examples/basic/main.go) (search, export, full text)
and [`examples/discovery`](examples/discovery/main.go) (query builder, ESummary, ELink, EInfo,
ESpell).

### API

Every call takes a `context.Context` first. `Config` accepts `APIKey`, `Email`, `Tool`,
`RateLimit`, `Timeout`, `UserAgent`, `BaseURL`, `EuropePMCBaseURL` and `Cache`; the zero value is
valid. `BaseURL` overrides the NCBI E-utilities endpoint only — Europe PMC is hosted by EBI
elsewhere and has its own override.

#### Lifecycle

| Method                          | Description                                        |
| ------------------------------- | -------------------------------------------------- |
| `New(*Config) (*Client, error)` | Create a client; nil config means library defaults |
| `(*Client).Close() error`       | Release the Rust client; idempotent                |
| `Version() string`              | Version of the underlying Rust crate               |

#### PubMed search and metadata

| Method                                   | Description                                     |
| ---------------------------------------- | ----------------------------------------------- |
| `SearchArticles(ctx, query, limit)`      | Search PubMed, return PMIDs                     |
| `SearchArticlesWithOptions(…, options)`  | As above, with a result ordering                |
| `Search(ctx, *SearchQuery)`              | Run a built query, honouring its limit and sort |
| `FetchArticle(ctx, pmid)`                | Metadata for one PMID                           |
| `FetchArticles(ctx, pmids)`              | Batched metadata                                |
| `FetchAllByPMIDs(ctx, pmids)`            | Batched metadata for very large PMID lists      |
| `SearchAndFetch(ctx, query, limit)`      | Search, then fetch each hit                     |
| `SearchAndFetchWithOptions(…, options)`  | As above, with a result ordering                |
| `SearchAndFetchQuery(ctx, *SearchQuery)` | Run a built query and fetch each hit            |
| `SearchWithFullText(ctx, query, limit)`  | Search and attach PMC full text where available |

#### ESummary

| Method                                       | Description                     |
| -------------------------------------------- | ------------------------------- |
| `FetchSummary(ctx, pmid)`                    | Lightweight record for one PMID |
| `FetchSummaries(ctx, pmids)`                 | Lightweight records, batched    |
| `SearchAndFetchSummaries(ctx, query, limit)` | Search, then summarise each hit |

#### ELink, EInfo, EGQuery, ECitMatch, ESpell

| Method                              | Description                                    |
| ----------------------------------- | ---------------------------------------------- |
| `GetRelatedArticles(ctx, pmids)`    | Articles PubMed considers related              |
| `GetPMCLinks(ctx, pmids)`           | PMC identifiers with full text available       |
| `GetCitations(ctx, pmids)`          | Articles citing the given PMIDs                |
| `GetDatabaseList(ctx)`              | Every Entrez database name                     |
| `GetDatabaseInfo(ctx, database)`    | Record count, searchable fields, links         |
| `GlobalQuery(ctx, term)`            | Match counts across every Entrez database      |
| `MatchCitations(ctx, citations)`    | Resolve bibliographic citations to PMIDs       |
| `SpellCheck(ctx, term)`             | Spelling suggestions for a search term         |
| `SpellCheckDB(ctx, term, database)` | As above, against a database other than PubMed |

The ELink calls take `[]uint32` rather than PMID strings, matching NCBI's own UID parameter.

#### PMC

| Method                                          | Description                                 |
| ----------------------------------------------- | ------------------------------------------- |
| `FetchFullText(ctx, pmcid)`                     | Structured full text                        |
| `FetchXML(ctx, pmcid)`                          | Raw JATS XML                                |
| `FetchMarkdown(ctx, pmcid)`                     | Full text rendered to Markdown              |
| `FetchMarkdownWithOptions(ctx, pmcid, options)` | As above, with the rendering tuned          |
| `CheckPMCAvailability(ctx, pmid)`               | Is PMC full text available?                 |
| `IsOASubset(ctx, pmcid)`                        | Open Access status, licence, retraction     |
| `DownloadFiles(ctx, pmcid, dir)`                | Download an OA article's files              |
| `ExtractFigures(ctx, pmcid, dir)`               | Download figures paired with their captions |
| `ClearPMCCache(ctx)`                            | Drop cached PMC responses                   |

#### Europe PMC

[Europe PMC](https://europepmc.org) is a complementary index to the NCBI E-utilities: it covers
preprints (`PPR`), patents (`PAT`), Agricola (`AGR`) and Chinese Biological Abstracts (`CBA`) as
well as PubMed (`MED`) and PMC, and needs no API key.

Records are addressed by a source database plus an id. Every method takes both; `source` may be
empty, in which case a `PMC`-prefixed id is read as a PMC record and anything else as a PubMed
record. An id in fully-qualified `"SOURCE/ID"` form wins over `source`.

| Method                                                       | Description                                    |
| ------------------------------------------------------------ | ---------------------------------------------- |
| `EuropePMCSearch(ctx, query, limit)`                         | Search every Europe PMC source                 |
| `EuropePMCSearchWithOptions(ctx, query, limit, options)`     | As above, with detail level, page size, sort   |
| `EuropePMCSearchPage(ctx, query, options)`                   | One page, with the total count and next cursor |
| `EuropePMCFetchFullText(ctx, id, source)`                    | Structured full text (PMC-sourced records)     |
| `EuropePMCFetchXML(ctx, id, source)`                         | Raw JATS XML (any source with full text)       |
| `EuropePMCReferences(ctx, id, source)`                       | Works cited by the record                      |
| `EuropePMCCitations(ctx, id, source)`                        | Articles citing the record                     |
| `EuropePMCDatabaseLinks(ctx, id, source)`                    | Cross-references to external databases         |
| `EuropePMCDownloadSupplementaryFiles(ctx, id, source, path)` | Download the supplementary-files ZIP           |

```go
// Cross-source search, including preprints
results, err := client.EuropePMCSearch(ctx, "TITLE:CRISPR AND SRC:PPR", 10)
for _, result := range results {
	fmt.Println(result.EuropePMCID, result.Title)
}

// Citation graph in both directions — broader than GetCitations, which sees
// only PubMed-indexed articles
references, err := client.EuropePMCReferences(ctx, "PMC3258128", "")
citations, err := client.EuropePMCCitations(ctx, "33515491", "MED")
```

`EuropePMCCore` results carry far more fields than are modelled, and the set changes over time;
whatever is not named on the struct lands in its `Extra map[string]any` rather than being dropped.

#### Query builder and export

`SearchQuery` records the builder calls you make and replays them against the Rust `SearchQuery`,
so field tags have one implementation across every language binding:

```go
query := pubmedclient.NewSearchQuery().
	TitleOrAbstract("CRISPR").
	MeshTerm("Gene Editing").
	PublishedAfter(pubmedclient.Year(2020)).
	ArticleType("Review").
	HumanStudiesOnly().
	Limit(20).
	Sort(pubmedclient.SortPublicationDate)

if err := query.Validate(); err != nil {
	log.Fatal(err)
}
articles, err := client.SearchAndFetchQuery(ctx, query)
```

`Build()` returns the assembled query string, and `String()` is the same thing for logging.
Raw PubMed syntax still works everywhere a query string is accepted:

```go
pmids, err := client.SearchArticles(ctx, "cancer[ti] AND 2023[pdat] AND review[pt]", 20)
```

Citation export renders in Rust, so the output matches the CLI and the other bindings:

```go
bibtex, err := pubmedclient.ExportArticles(articles, pubmedclient.FormatBibTeX)
ris, err := articles[0].ToRIS()
```

Formats: `FormatBibTeX`, `FormatRIS`, `FormatCSLJSON`, `FormatNBIB`.

### Contexts and cancellation

Cancelling a context aborts the in-flight HTTP request rather than merely reporting the
cancellation once it finishes: the call is handed a cancellation token that a watchdog goroutine
fires, and the Rust side drops the request future. The call then returns the context's own error,
so `errors.Is(err, context.Canceled)` and `context.DeadlineExceeded` work as expected.

`Config.Timeout` still bounds each individual HTTP request, which is the right tool for a
per-request ceiling; a context deadline bounds the whole call.

### Rate limits

NCBI allows 3 requests/second without an API key and 10 with one. The Rust client enforces this
with a shared token bucket, so a `*Client` shared across goroutines stays within the limit.

### Concurrency and lifetime

A `*Client` is safe for concurrent use. It owns memory outside the Go heap, so call `Close` when
done — a finalizer is registered as a safety net, but relying on it is not recommended. `Close`
waits for in-flight calls to finish, so cancelling a context and closing immediately is safe.
Calls made after `Close` return `ErrClosed`.

### Errors

Failures arrive as `*Error`, carrying the failing operation, a `Kind`, and the message from
`pubmed-client`. The common causes also match package sentinels:

```go
if errors.Is(err, pubmedclient.ErrPMCNotAvailable) {
	// expected: most PubMed articles are not in the PMC Open Access subset
}

var ffiErr *pubmedclient.Error
if errors.As(err, &ffiErr) {
	log.Printf("%s failed (%s): %s", ffiErr.Op, ffiErr.Kind, ffiErr.Message)
}
```

Sentinels: `ErrClosed`, `ErrInvalidArgument`, `ErrNotFound`, `ErrPMCNotAvailable`,
`ErrRateLimited`, `ErrInvalidQuery`. `Error.Status` carries the HTTP status when `Kind` is
`KindAPI`.

### Known limitations

- `PMCArticle` is a flattened projection of the JATS tree, not the full DTD model. Table cell
  contents in particular are omitted — use `FetchXML` or `FetchMarkdown` when the rendered table
  is needed.
- The history-server API (`EPost`, WebEnv sessions) is not exposed directly; `FetchAllByPMIDs`
  uses it internally for large PMID lists.

## Testing

All tasks live in `mise.go.toml` at the workspace root and need `MISE_ENV=go`:

```bash
MISE_ENV=go mise run go:test              # offline: unit tests plus end-to-end tests against a stub server
MISE_ENV=go mise run go:test-integration  # live NCBI API, opt-in
MISE_ENV=go mise run lint:go              # gofmt + go vet
MISE_ENV=go mise run fmt:go               # gofmt -w
```

The offline suite points `Config.BaseURL` at an `httptest` server covering every E-utilities
endpoint the bindings call, so it exercises the whole chain (Go → cgo → Rust → HTTP → parsing →
JSON → Go structs) without network access.

Integration tests additionally require `PUBMED_REAL_API_TESTS=1` (set by `go:test-integration`)
and honour `NCBI_API_KEY` and `PUBMED_EMAIL`.

The Rust FFI layer has its own tests covering the boundary conventions, cancellation, the query
replay and the export merge:

```bash
cargo test -p pubmed-client-go
```

## Layout

```
pubmed-client-go/
├── rust/            # C-ABI shim crate (staticlib), a Cargo workspace member
│   └── src/
│       ├── error.rs   # the JSON error envelope and its kind taxonomy
│       ├── ffi.rs     # argument borrowing, result ownership, panic containment
│       ├── cancel.rs  # the Tokio runtime and the cancellation token
│       ├── client.rs  # handle lifecycle and configuration decoding
│       ├── dto.rs     # projections of Rust models onto the JSON Go decodes
│       ├── pubmed.rs  # E-utilities calls
│       ├── pmc.rs     # PMC full text, XML, Markdown, OA downloads
│       ├── europe_pmc.rs # Europe PMC search, full text, citation graphs
│       ├── query.rs   # replay of the Go query builder onto SearchQuery
│       └── export.rs  # citation export
├── include/         # C header consumed by cgo
├── ffi.go           # the only file that talks to C
├── client.go        # Client, Config, context plumbing
├── pubmed.go        # PubMed and E-utilities methods
├── pmc.go           # PMC methods and Markdown options
├── europe_pmc.go    # Europe PMC methods and search options
├── query.go         # SearchQuery builder
├── export.go        # citation export
├── models.go        # Go structs mirroring the JSON boundary
├── errors.go        # Error, Kind, sentinels
└── examples/
```

Values cross the FFI boundary as JSON: each call returns one owned C string that Go re-parses into
the typed structs in `models.go`. That keeps the C surface small while both sides stay fully
typed, and lets the Rust models gain fields without breaking Go. Errors cross as a JSON envelope
(`{"kind": …, "message": …}`), which is what makes the sentinels above possible without matching
on message text.

## License

MIT, same as the rest of the workspace.
