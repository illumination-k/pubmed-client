#!/bin/bash

set -euo pipefail

# 例：リモート環境でのみ実行
if [ "$CLAUDE_CODE_REMOTE" != "true" ]; then
	exit 0
fi

## Create symlink from AGENTS.md to CLAUDE.md
cd "$(dirname "$0")/.."

# install mise
curl https://mise.run | sh

export PATH="$HOME/.local/bin:$PATH"
eval "$(~/.local/bin/mise activate bash)"

mise trust
mise install
