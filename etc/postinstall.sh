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

function has_cargo_subcommand() {
	cargo "$1" --version >/dev/null 2>&1
}

echo "==> Running postinstall setup..."

# Setup pre-commit hooks
echo "==> Installing pre-commit hooks..."
if has_command "pre-commit"; then
	pre-commit install --install-hooks
else
	echo "Warning: pre-commit is not installed, skipping hooks installation"
fi

# Setup Python virtual environment for PyO3
echo "==> Setting up Python virtual environment..."
cd "$PROJECT_ROOT/pubmed-client-py"
uv venv

# Setup test environment (cargo tools)
echo "==> Setting up test environment..."
cd "$PROJECT_ROOT"

if ! has_command "cargo"; then
	echo "Error: cargo is not installed. Please install Rust from https://www.rust-lang.org/tools/install"
	exit 1
fi

# Check and install cargo-nextest if not present
if ! has_cargo_subcommand "nextest"; then
	echo "cargo-nextest is not installed. Installing..."
	cargo install cargo-nextest --locked
	if [ $? -ne 0 ]; then
		echo "Error: Failed to install cargo-nextest"
		exit 1
	fi
	echo "cargo-nextest installed successfully"
else
	echo "cargo-nextest is already installed"
fi

# Check and install cargo-llvm-cov if not present (for coverage)
if ! has_cargo_subcommand "llvm-cov"; then
	echo "cargo-llvm-cov is not installed. Installing..."
	cargo install cargo-llvm-cov --locked
	if [ $? -ne 0 ]; then
		echo "Error: Failed to install cargo-llvm-cov"
		exit 1
	fi
	echo "cargo-llvm-cov installed successfully"
else
	echo "cargo-llvm-cov is already installed"
fi

echo "==> Postinstall setup complete!"
