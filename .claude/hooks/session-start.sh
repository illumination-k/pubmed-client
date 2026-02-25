#!/bin/bash

set -eu

cd "$(dirname "$0")"

source ./common.sh

cd ../..

if ! check_command mise; then
	curl https://mise.run | sh
	export PATH="$HOME/.local/bin:$PATH"
fi

mise trust

mise settings experimental=true
mise install

# スクリプトはbashで実行されるため、常にbash用のactivateを使用
eval "$(~/.local/bin/mise activate bash)"

pnpm install
