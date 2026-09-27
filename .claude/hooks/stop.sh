#!/bin/bash

set -eu

cd "$(dirname "$0")"

CLAUDE_CODE_FEEDBACK_EXIT_CODE=2

source ./common.sh

cd ../..

# Appended, not prepended: mise env (below) prepends the pinned tool dirs to
# whatever PATH it inherits, so a leading ~/.local/bin would shadow them and the
# hook would silently lint with different tool versions than CI (e.g. a
# uv-installed zizmor instead of the version pinned in mise.root.toml).
export PATH="$PATH:$HOME/.local/bin"

if ! check_command mise; then
	echo "mise command not found. Please install mise to use this hook."
	exit 1
fi

eval "$(mise env -s bash)"

if ! mise run fmt || ! mise run lint; then
	echo "Formatting or linting failed. Please fix the issues above."
	exit $CLAUDE_CODE_FEEDBACK_EXIT_CODE
fi
