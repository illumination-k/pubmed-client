#!/bin/bash

set -eu

HOOK_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$HOOK_DIR/../.." && pwd)}"

# shellcheck source=common.sh
source "$HOOK_DIR/common.sh"

cd "$PROJECT_ROOT"

if ! check_command mise; then
	curl https://mise.run | sh
	export PATH="$HOME/.local/bin:$PATH"
fi

# rust-toolchain.toml requests components but rustup may have installed the
# pinned toolchain without `cargo`. Ensure it is present so cargo-backed mise
# tools (cargo-nextest, cargo-llvm-cov) can be installed.
if check_command rustup; then
	channel="$(awk -F'"' '/^[[:space:]]*channel/ {print $2; exit}' rust-toolchain.toml 2>/dev/null || true)"
	if [ -n "$channel" ]; then
		rustup component add cargo rustc rust-std --toolchain "$channel" >/dev/null 2>&1 || true
	fi
fi

mise trust --all

# write all MISE_ENV into .miserc.toml

cat <<EOF > ~/.miserc.toml
env = ["root", "rust", "node", "python"]
EOF

mise settings experimental=true
mise install


DETECTED_SHELL=${CLAUDE_CODE_SHELL:-$(basename "$SHELL")}

if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
	# initialize
	echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >"$CLAUDE_ENV_FILE"
	case "$DETECTED_SHELL" in
	bash | zsh)
		mise env -s "$DETECTED_SHELL" >>"$CLAUDE_ENV_FILE"
		;;
	*)
		echo "Unsupported shell: $DETECTED_SHELL"
		exit 1
		;;
	esac
else
	echo "CLAUDE_ENV_FILE is not set. Skipping shell environment setup."
fi
