#!/bin/bash

set -eu

cd "$(dirname "$0")"

source ./common.sh

cd ../..

if ! check_command mise; then
	curl https://mise.run | sh
	export PATH="$HOME/.local/bin:$PATH"
fi

mise trust --all

# write all MISE_ENV into .miserc.toml

cat <<EOF > ~/.miserc.toml
env = ["root", "rust", "node", "python"]
EOF

mise settings experimental=true
# Tolerate individual tool install failures (e.g. cargo-built tools that
# require a rustup toolchain not present in this environment); the rest of
# the env should still come up so subsequent hooks can run.
mise install || echo "mise install reported failures; continuing with available tools"

# Repair the pinned rust toolchain if its dir exists but cargo/rustc are
# missing (rustup sees the dir and skips reinstall, so any cargo invocation
# proxied through it fails; this also breaks maturin builds via uv).
if check_command rustup && [ -f rust-toolchain.toml ]; then
	if ! cargo --version >/dev/null 2>&1; then
		channel=$(awk -F'"' '/^channel/ {print $2}' rust-toolchain.toml)
		if [ -n "$channel" ]; then
			echo "Repairing rust toolchain $channel (cargo missing)..."
			rustup toolchain uninstall "$channel" || true
			rustup toolchain install "$channel" --profile default \
				--component rustfmt --component clippy --no-self-update
		fi
	fi
fi


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
