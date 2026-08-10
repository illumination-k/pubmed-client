# pubmed-client-go

Go bindings for [pubmed-client](https://github.com/illumination-k/pubmed-client) — an async
Rust client for the PubMed and PMC (PubMed Central) APIs.

The bindings wrap the Rust crate through cgo. Calls are synchronous: a shared Tokio runtime
drives the async client and blocks until the request completes.

> **Status: MVP.** The surface covers PubMed search and metadata plus PMC full text and Markdown.
> Citation export, figure extraction, ELink/EInfo/ESpell and the query builder are available in
> the [Rust](../pubmed-client), [Python](../pubmed-client-py) and [Node](../pubmed-client-napi)
> bindings but not yet here.

## Requirements

- Go 1.23+ with cgo enabled (a C toolchain must be on `PATH`)
- Rust 1.93+ and `cargo`, to build the static library

TLS is pure-Rust (rustls + ring), so **no OpenSSL/libssl is needed** at build or run time.

## Building

The package links against a static archive produced by the Rust crate in [`rust/`](rust/). It is
about 75 MB, so it is built from source rather than committed:

```bash
cd pubmed-client-go
make build          # cargo build --release -p pubmed-client-go, staged into lib/$GOOS_$GOARCH/
go build ./...
```

`make build` must be re-run after any change to the Rust crates.

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

	articles, err := client.SearchAndFetch("CRISPR gene editing", 5)
	if err != nil {
		log.Fatal(err)
	}
	for _, article := range articles {
		fmt.Printf("%s: %s (%s)\n", article.PMID, article.Title, article.Journal)
	}
}
```

A fuller example lives in [`examples/basic`](examples/basic/main.go).

### API

| Method                                                                 | Description                                        |
| ---------------------------------------------------------------------- | -------------------------------------------------- |
| `New(*Config) (*Client, error)`                                        | Create a client; nil config means library defaults |
| `(*Client).Close() error`                                              | Release the Rust client; idempotent                |
| `(*Client).SearchArticles(query string, limit int) ([]string, error)`  | Search PubMed, return PMIDs                        |
| `(*Client).FetchArticle(pmid string) (*Article, error)`                | Metadata for one PMID                              |
| `(*Client).FetchArticles(pmids []string) ([]Article, error)`           | Batched metadata                                   |
| `(*Client).SearchAndFetch(query string, limit int) ([]Article, error)` | Search, then fetch each hit                        |
| `(*Client).FetchFullText(pmcid string) (*PMCArticle, error)`           | PMC full text                                      |
| `(*Client).FetchMarkdown(pmcid string) (string, error)`                | PMC full text rendered to Markdown                 |
| `(*Client).CheckPMCAvailability(pmid string) (string, bool, error)`    | Is PMC full text available?                        |
| `Version() string`                                                     | Version of the underlying Rust crate               |

`Config` accepts `APIKey`, `Email`, `Tool`, `RateLimit`, `Timeout`, `UserAgent`, `BaseURL` and
`Cache`. The zero value is valid.

Queries accept PubMed's full syntax, including field tags:

```go
pmids, err := client.SearchArticles("cancer[ti] AND 2023[pdat] AND review[pt]", 20)
```

### Rate limits

NCBI allows 3 requests/second without an API key and 10 with one. The Rust client enforces this
with a shared token bucket, so a `*Client` shared across goroutines stays within the limit.

### Concurrency and lifetime

A `*Client` is safe for concurrent use. It owns memory outside the Go heap, so call `Close` when
done — a finalizer is registered as a safety net, but relying on it is not recommended. Calls made
after `Close` return `ErrClosed`.

### Errors

Failures from the Rust side arrive as `*Error`, carrying the failing operation and the message:

```go
var ffiErr *pubmedclient.Error
if errors.As(err, &ffiErr) {
	log.Printf("%s failed: %s", ffiErr.Op, ffiErr.Message)
}
```

### Known limitations

- No `context.Context` support. Calls block for the duration of the request and cannot be
  cancelled; bound them with `Config.Timeout` instead.
- `PMCArticle` is a flattened projection of the JATS tree, not the full DTD model. Table cell
  contents in particular are omitted — use `FetchMarkdown` when the rendered table is needed.

## Testing

```bash
make test              # offline: unit tests plus end-to-end tests against a local stub server
make test-integration  # live NCBI API, opt-in
make check             # gofmt + go vet
```

The offline suite points `Config.BaseURL` at an `httptest` server, so it exercises the whole chain
(Go → cgo → Rust → HTTP → XML parsing → JSON → Go structs) without network access.

Integration tests additionally require `PUBMED_REAL_API_TESTS=1` (set by `make test-integration`)
and honour `NCBI_API_KEY` and `PUBMED_EMAIL`.

The Rust FFI layer has its own tests covering the boundary conventions:

```bash
cargo test -p pubmed-client-go
```

## Layout

```
pubmed-client-go/
├── rust/            # C-ABI shim crate (staticlib), a Cargo workspace member
├── include/         # C header consumed by cgo
├── ffi.go           # the only file that talks to C
├── client.go        # public Go API
├── models.go        # Go structs mirroring the JSON boundary
├── errors.go
└── examples/basic/
```

Values cross the FFI boundary as JSON: each call returns one owned C string that Go re-parses into
the typed structs in `models.go`. That keeps the C surface at a handful of functions while both
sides stay fully typed, and lets the Rust models gain fields without breaking Go.

## License

MIT, same as the rest of the workspace.
