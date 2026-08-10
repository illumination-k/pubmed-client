# Build the Rust static library the cgo package links against, then run the Go
# toolchain over it.
#
#   make build   compile the Rust archive into lib/$(GOOS)_$(GOARCH)/
#   make test    build, then run the offline Go tests
#   make check   gofmt + go vet
#   make clean   remove the staged archives

GO      ?= go
CARGO   ?= cargo
GOOS    ?= $(shell $(GO) env GOOS)
GOARCH  ?= $(shell $(GO) env GOARCH)

CRATE       := pubmed-client-go
TARGET_DIR  := $(shell $(CARGO) metadata --format-version 1 --no-deps --manifest-path rust/Cargo.toml | \
                 sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
ARCHIVE     := libpubmed_client_go.a
LIB_DIR     := lib/$(GOOS)_$(GOARCH)

.PHONY: all build test test-integration check fmt vet clean

all: build

# `-p $(CRATE)` matters: under `cargo build --workspace` Cargo unifies features
# across the whole workspace and pulls native-tls back in, which would make the
# archive depend on system OpenSSL.
build:
	$(CARGO) build --release -p $(CRATE)
	mkdir -p $(LIB_DIR)
	cp $(TARGET_DIR)/release/$(ARCHIVE) $(LIB_DIR)/$(ARCHIVE)

test: build
	$(GO) test ./...

# Live NCBI calls, opt-in and rate limited — same convention as the Rust crate.
test-integration: build
	PUBMED_REAL_API_TESTS=1 $(GO) test -tags=integration -timeout 10m ./...

check: fmt vet

fmt:
	@output="$$(gofmt -l . 2>&1)"; \
	if [ -n "$$output" ]; then echo "gofmt needed:"; echo "$$output"; exit 1; fi

vet: build
	$(GO) vet ./...

clean:
	rm -rf lib
