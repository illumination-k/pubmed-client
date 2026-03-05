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
mise install


DETECTED_SHELL=${CLAUDE_CODE_SHELL:-$(basename "$SHELL")}

if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
	# initialize
	echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >"$CLAUDE_ENV_FILE"
	case "$DETECTED_SHELL" in
	bash)
		mise activate bash >>"$CLAUDE_ENV_FILE"
		;;
	zsh)
		mise activate zsh >>"$CLAUDE_ENV_FILE"
		;;
	*)
		echo "Unsupported shell: $DETECTED_SHELL"
		exit 1
		;;
	esac
else
	echo "CLAUDE_ENV_FILE is not set. Skipping shell environment setup."
fi

source ~/.bashrc
