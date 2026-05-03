#!/bin/bash

# Post-install setup script for mise
# This script is run after mise installs tools
#
# To skip this script, set SKIP_POSTINSTALL=1:
#   SKIP_POSTINSTALL=1 mise install

set -euo pipefail

# Allow skipping postinstall via environment variable
if [[ "${SKIP_POSTINSTALL:-}" == "1" ]]; then
	echo "==> Skipping postinstall setup (SKIP_POSTINSTALL=1)"
	exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

function has_command() {
	command -v "$1" >/dev/null 2>&1
}

echo "==> Running postinstall setup..."

echo "==> Installing prek hooks..."
if has_command "prek"; then
	prek install -f
else
	echo "Warning: prek is not installed, skipping hooks installation"
	echo "         Run with MISE_ENV=root to install prek"
fi

# Setup Python virtual environment for PyO3 (skip if already created;
# `uv venv` against an existing dir prints a warning we don't need every run).
# uv is only available when MISE_ENV includes 'python'.
echo "==> Setting up Python virtual environment..."
if has_command "uv"; then
	if [[ -f "$PROJECT_ROOT/pubmed-client-py/.venv/pyvenv.cfg" ]]; then
		echo "    .venv already present; skipping uv venv"
	else
		(cd "$PROJECT_ROOT/pubmed-client-py" && uv venv)
	fi
else
	echo "Warning: uv is not installed, skipping Python venv setup"
	echo "         Run with MISE_ENV=python to install uv"
fi

# Install pnpm dependencies for TS packages in parallel (the three workspaces
# are independent; pnpm's content-addressed store is concurrency-safe).
# pnpm is only available when MISE_ENV includes 'node'.
echo "==> Installing pnpm dependencies for TS packages..."
if has_command "pnpm"; then
	pids=()
	for dir in pubmed-client-napi pubmed-client-wasm website; do
		(cd "$PROJECT_ROOT/$dir" && pnpm install --frozen-lockfile) &
		pids+=("$!")
	done
	pnpm_status=0
	for pid in "${pids[@]}"; do
		wait "$pid" || pnpm_status=$?
	done
	if [[ $pnpm_status -ne 0 ]]; then
		echo "Error: one or more pnpm installs failed" >&2
		exit "$pnpm_status"
	fi
else
	echo "Warning: pnpm is not installed, skipping TS package setup"
	echo "         Run with MISE_ENV=node to install pnpm"
fi

echo "==> Postinstall setup complete!"
