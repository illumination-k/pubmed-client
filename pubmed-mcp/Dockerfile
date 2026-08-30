# syntax=docker/dockerfile:1

# Container image for the PubMed MCP server.
#
# Build from the WORKSPACE ROOT (the crate depends on path deps):
#   docker build -f pubmed-mcp/Dockerfile -t pubmed-mcp .
#
# Run (stdio transport, the default):
#   docker run --rm -i pubmed-mcp
# Run (streamable HTTP transport):
#   docker run --rm -p 8080:8080 pubmed-mcp --port 8080

# Must match rust-toolchain.toml — CI (ci-docker.yml) fails the build if it drifts.
ARG RUST_VERSION=1.93.1

FROM rust:${RUST_VERSION}-slim-bookworm AS builder
ARG RUST_VERSION

# The base image ships a `minimal` profile, so rust-toolchain.toml's
# `components = ["rustfmt", "clippy"]` would make rustup re-download the whole
# (already installed) toolchain. Pinning RUSTUP_TOOLCHAIN takes precedence over
# the toolchain file, so no toolchain is re-downloaded.
ENV RUSTUP_TOOLCHAIN=${RUST_VERSION}

# pubmed-client's default `native-tls` feature links against system OpenSSL.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

# The cache mounts speed up local rebuilds; the binary is copied out of the
# mounted target dir inside the same RUN, since the mount is gone afterwards.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/build/target,sharing=locked \
    cargo build --release --locked -p pubmed-mcp \
    && cp target/release/pubmed-mcp /usr/local/bin/pubmed-mcp

FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="pubmed-mcp" \
      org.opencontainers.image.description="MCP server for PubMed and PMC APIs" \
      org.opencontainers.image.source="https://github.com/illumination-k/pubmed-client" \
      org.opencontainers.image.licenses="MIT"

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 --user-group mcp

COPY --from=builder /usr/local/bin/pubmed-mcp /usr/local/bin/pubmed-mcp

USER mcp
WORKDIR /home/mcp

# Only relevant with `--port`; the default stdio transport listens on nothing.
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/pubmed-mcp"]
