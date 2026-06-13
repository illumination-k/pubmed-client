#!/usr/bin/env bash
set -euo pipefail

# sync-versions.sh — keep every publishable package on a single unified version.
#
# Source of truth: [workspace.package] version in the root Cargo.toml.
# Targets kept in lock-step:
#   - root Cargo.toml [workspace.dependencies] internal crate versions
#   - pubmed-client-napi/package.json  (version + optionalDependencies.*)
#   - pubmed-client-wasm/package.json  (version)
#   - pubmed-client-py/pyproject.toml  ([project] version)
#
# Usage:
#   sync-versions.sh <version>   Bump the workspace version, then propagate everywhere.
#   sync-versions.sh             Propagate the current workspace version everywhere.
#   sync-versions.sh --check     Verify every target matches the workspace version (exit 1 on drift).
#
# Requires: jq (for package.json edits). Available on GitHub-hosted runners by default.

SCRIPT_NAME="$(basename "$0")"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

ROOT_CARGO="${ROOT}/Cargo.toml"
NAPI_PKG="${ROOT}/pubmed-client-napi/package.json"
WASM_PKG="${ROOT}/pubmed-client-wasm/package.json"
PY_PYPROJECT="${ROOT}/pubmed-client-py/pyproject.toml"

# Note: the napi pnpm-lock.yaml does NOT pin the per-platform binary versions —
# they are listed under pnpm.ignoredOptionalDependencies in package.json so a
# version bump never invalidates the lockfile. So there is nothing to sync there.

# Internal crates that carry an explicit version in [workspace.dependencies].
INTERNAL_CRATES=(pubmed-client pubmed-formatter pubmed-parser)

show_help() {
	cat <<EOF
Usage: ${SCRIPT_NAME} [<version> | --check | --help]

  <version>   Set the workspace version (e.g. 0.2.0) and propagate to all packages.
  (no args)   Propagate the current workspace version to all packages.
  --check     Verify all packages match the workspace version; exit 1 on mismatch.
  --help      Show this help.
EOF
}

die() {
	echo "error: $*" >&2
	exit 1
}

# Read the version from [workspace.package] in the root Cargo.toml.
read_workspace_version() {
	awk '
		/^\[workspace\.package\]/ { in_section = 1; next }
		/^\[/ { in_section = 0 }
		in_section && /^version[[:space:]]*=/ {
			match($0, /"[^"]+"/)
			print substr($0, RSTART + 1, RLENGTH - 2)
			exit
		}
	' "$ROOT_CARGO"
}

# Set the version inside a named TOML table (only the first version key in that table).
set_toml_table_version() {
	local file="$1" table="$2" version="$3"
	local tmp
	tmp="$(mktemp)"
	awk -v table="$table" -v version="$version" '
		$0 == "[" table "]" { in_section = 1; print; next }
		/^\[/ { in_section = 0 }
		in_section && !done && /^version[[:space:]]*=/ {
			sub(/"[^"]+"/, "\"" version "\"")
			done = 1
		}
		{ print }
	' "$file" >"$tmp"
	mv "$tmp" "$file"
}

# Set the version on each internal crate line in [workspace.dependencies].
set_internal_crate_versions() {
	local version="$1" crate
	for crate in "${INTERNAL_CRATES[@]}"; do
		# Match lines like: pubmed-client = { path = "pubmed-client", version = "X" }
		perl -i -pe "s/^(\\Q${crate}\\E\\s*=\\s*\\{[^}]*version\\s*=\\s*)\"[^\"]+\"/\${1}\"${version}\"/" "$ROOT_CARGO"
	done
}

set_json_version() {
	local file="$1" version="$2" tmp
	tmp="$(mktemp)"
	jq --arg v "$version" '.version = $v' "$file" >"$tmp"
	mv "$tmp" "$file"
}

set_napi_optional_deps() {
	local version="$1" tmp
	tmp="$(mktemp)"
	# Bump every optionalDependencies entry (platform-specific binary packages).
	jq --arg v "$version" \
		'.optionalDependencies |= with_entries(.value = $v)' \
		"$NAPI_PKG" >"$tmp"
	mv "$tmp" "$NAPI_PKG"
}

propagate() {
	local version="$1"
	set_toml_table_version "$ROOT_CARGO" "workspace.package" "$version"
	set_internal_crate_versions "$version"
	set_json_version "$NAPI_PKG" "$version"
	set_napi_optional_deps "$version"
	set_json_version "$WASM_PKG" "$version"
	set_toml_table_version "$PY_PYPROJECT" "project" "$version"
	echo "Synced all packages to ${version}"
}

# Collect "label\tversion" rows for every target, for --check reporting.
collect_versions() {
	local crate
	printf 'workspace.package\t%s\n' "$(read_workspace_version)"
	for crate in "${INTERNAL_CRATES[@]}"; do
		local v
		v="$(grep -E "^${crate}\s*=\s*\{" "$ROOT_CARGO" | grep -oE 'version\s*=\s*"[^"]+"' | grep -oE '"[^"]+"' | tr -d '"')"
		printf 'workspace.deps.%s\t%s\n' "$crate" "$v"
	done
	printf 'napi.version\t%s\n' "$(jq -r '.version' "$NAPI_PKG")"
	jq -r '.optionalDependencies | to_entries[] | "napi.opt." + .key + "\t" + .value' "$NAPI_PKG"
	printf 'wasm.version\t%s\n' "$(jq -r '.version' "$WASM_PKG")"
	printf 'py.version\t%s\n' "$(awk '
		/^\[project\]/ { in_section = 1; next }
		/^\[/ { in_section = 0 }
		in_section && /^version[[:space:]]*=/ {
			match($0, /"[^"]+"/); print substr($0, RSTART + 1, RLENGTH - 2); exit
		}
	' "$PY_PYPROJECT")"
}

check() {
	local expected mismatch=0
	expected="$(read_workspace_version)"
	[[ -n "$expected" ]] || die "could not read workspace version from ${ROOT_CARGO}"

	while IFS=$'\t' read -r label actual; do
		if [[ "$actual" != "$expected" ]]; then
			printf '  MISMATCH  %-28s %s (expected %s)\n' "$label" "${actual:-<empty>}" "$expected"
			mismatch=1
		fi
	done < <(collect_versions)

	if [[ "$mismatch" -ne 0 ]]; then
		echo >&2
		echo "error: package versions are out of sync with workspace version ${expected}." >&2
		echo "       run 'scripts/sync-versions.sh' (or 'mise run sync-versions') to fix." >&2
		exit 1
	fi
	echo "All package versions are in sync at ${expected}"
}

main() {
	command -v jq >/dev/null || die "jq is required but not found in PATH"

	case "${1:-}" in
	-h | --help)
		show_help
		;;
	--check)
		check
		;;
	"")
		propagate "$(read_workspace_version)"
		;;
	-*)
		die "unknown option: $1"
		;;
	*)
		[[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$ ]] ||
			die "invalid version: '$1' (expected semver like 0.2.0 or 1.0.0-rc.1)"
		propagate "$1"
		;;
	esac
}

main "$@"
