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


if [ ! -f ~/.bashrc ] || ! grep -q 'mise activate bash' ~/.bashrc; then
	echo 'eval "$(~/.local/bin/mise activate bash)"' >> ~/.bashrc
fi

if [ ! -f ~/.zshrc ] || ! grep -q 'mise activate zsh' ~/.zshrc; then
	echo 'eval "$(~/.local/bin/mise activate zsh)"' >> ~/.zshrc
fi

source ~/.bashrc
