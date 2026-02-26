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

# Setup Python virtual environment for PyO3
# uv is only available when MISE_ENV includes 'python'
echo "==> Setting up Python virtual environment..."
if has_command "uv"; then
	cd "$PROJECT_ROOT/pubmed-client-py"
	uv venv
else
	echo "Warning: uv is not installed, skipping Python venv setup"
	echo "         Run with MISE_ENV=python to install uv"
fi

# Install pnpm dependencies for TS packages
# pnpm is only available when MISE_ENV includes 'node'
echo "==> Installing pnpm dependencies for TS packages..."
if has_command "pnpm"; then
	cd "$PROJECT_ROOT/pubmed-client-napi"
	pnpm install --frozen-lockfile
	cd "$PROJECT_ROOT/pubmed-client-wasm"
	pnpm install --frozen-lockfile
	cd "$PROJECT_ROOT/website"
	pnpm install --frozen-lockfile
else
	echo "Warning: pnpm is not installed, skipping TS package setup"
	echo "         Run with MISE_ENV=node to install pnpm"
fi

echo "==> Postinstall setup complete!"
