#!/bin/bash

set -euo pipefail

function has_command() {
    command -v "$1" >/dev/null 2>&1
}

function has_cargo_subcommand() {
    cargo "$1" --version >/dev/null 2>&1
}

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
