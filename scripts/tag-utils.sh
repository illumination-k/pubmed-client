#!/usr/bin/env bash
set -euo pipefail

# Script name for usage display
SCRIPT_NAME="$(basename "$0")"

# Default values
REMOTE="${REMOTE:-origin}"
TAG=""
COMMITISH="HEAD"
MESSAGE=""
DRY_RUN=false

# Help message
show_help() {
	cat <<EOF
Usage: ${SCRIPT_NAME} [OPTIONS]

Recreate git tags. This script deletes existing remote tags and GitHub releases
if they exist, then creates and pushes a new tag.

Options:
  -t, --tag TAG           Tag name (required)
                          Example: node-v0.1.0, python-v1.2.3
  -c, --commit COMMIT     Commit-ish to tag (default: HEAD)
                          Example: abc123, main, HEAD~1
  -m, --message MESSAGE   Tag message (default: "Release <tag>")
  -r, --remote REMOTE     Git remote name (default: origin)
  -d, --dry-run           Show what would be done without executing
  -h, --help              Show this help message and exit

Examples:
  ${SCRIPT_NAME} -t node-v0.1.0
      Create tag node-v0.1.0 at HEAD

  ${SCRIPT_NAME} --tag node-v0.1.0 --commit abc123
      Create tag at specific commit

  ${SCRIPT_NAME} -t v1.0.0 -r upstream
      Create tag and push to 'upstream' remote

  ${SCRIPT_NAME} -t v1.0.0 -m "Initial release"
      Create tag with custom message

  ${SCRIPT_NAME} -t v1.0.0 --dry-run
      Show what would be done without executing

Environment Variables:
  REMOTE          Alternative to --remote option

Important:
  This script DELETES existing remote tags and GitHub releases with the same
  name. New releases are NOT created (only tags are created).

EOF
}

# Parse arguments
parse_args() {
	while [[ $# -gt 0 ]]; do
		case "$1" in
		-t | --tag)
			if [[ -z "${2:-}" ]]; then
				echo "Error: --tag requires a value" >&2
				exit 2
			fi
			TAG="$2"
			shift 2
			;;
		-c | --commit)
			if [[ -z "${2:-}" ]]; then
				echo "Error: --commit requires a value" >&2
				exit 2
			fi
			COMMITISH="$2"
			shift 2
			;;
		-m | --message)
			if [[ -z "${2:-}" ]]; then
				echo "Error: --message requires a value" >&2
				exit 2
			fi
			MESSAGE="$2"
			shift 2
			;;
		-r | --remote)
			if [[ -z "${2:-}" ]]; then
				echo "Error: --remote requires a value" >&2
				exit 2
			fi
			REMOTE="$2"
			shift 2
			;;
		-d | --dry-run)
			DRY_RUN=true
			shift
			;;
		-h | --help)
			show_help
			exit 0
			;;
		-*)
			echo "Error: Unknown option: $1" >&2
			echo "Use --help for usage information" >&2
			exit 2
			;;
		*)
			# Positional argument - treat first as tag, second as commit for backwards compatibility
			if [[ -z "$TAG" ]]; then
				TAG="$1"
			elif [[ "$COMMITISH" == "HEAD" ]]; then
				COMMITISH="$1"
			else
				echo "Error: Unexpected argument: $1" >&2
				exit 2
			fi
			shift
			;;
		esac
	done
}

# Validate version pattern in tag (e.g., v1.0.0, node-v0.1.0)
validate_version_pattern() {
	local tag="$1"
	# Match vX.X.X pattern anywhere in the tag (supports prefix like node-v0.1.0)
	if [[ ! "$tag" =~ v[0-9]+\.[0-9]+\.[0-9]+ ]]; then
		echo "Error: Tag must contain version pattern 'vX.X.X'" >&2
		echo "  Examples: v1.0.0, node-v0.1.0, python-v1.2.3" >&2
		exit 2
	fi
}

# Validate required arguments
validate_args() {
	if [[ -z "$TAG" ]]; then
		echo "Error: Tag name is required" >&2
		echo "" >&2
		echo "Usage: ${SCRIPT_NAME} -t <tag> [OPTIONS]" >&2
		echo "Use --help for more information" >&2
		exit 2
	fi

	validate_version_pattern "$TAG"

	# Set default message if not provided
	if [[ -z "$MESSAGE" ]]; then
		MESSAGE="Release ${TAG}"
	fi
}

# Parse command line arguments
parse_args "$@"
validate_args

# Helpers
log() { printf '[%s] %s\n' "$(date '+%Y-%m-%dT%H:%M:%S')" "$*"; }
err() { log "ERROR: $*" >&2; }

# Execute or print command based on DRY_RUN
run_cmd() {
	if $DRY_RUN; then
		log "[DRY-RUN] Would execute: $*"
	else
		"$@"
	fi
}

# Check commands
command -v git >/dev/null 2>&1 || {
	err "git is required"
	exit 3
}
GH_AVAILABLE=false
if command -v gh >/dev/null 2>&1; then
	GH_AVAILABLE=true
fi

# Detect local tag
local_tag_exists() {
	git show-ref --tags --quiet --verify "refs/tags/${TAG}"
}

# Detect remote tag (on REMOTE)
remote_tag_exists() {
	# git ls-remote returns lines like: <sha>\trefs/tags/<tag>
	git ls-remote --tags "$REMOTE" "refs/tags/${TAG}" | grep -q "refs/tags/${TAG}" || return 1
}

# Detect gh release
gh_release_exists() {
	if ! $GH_AVAILABLE; then
		return 1
	fi
	# gh release view returns non-zero if not found
	gh release view "$TAG" >/dev/null 2>&1
}

# Create annotated tag locally
create_local_tag() {
	log "Creating annotated local tag ${TAG} -> ${COMMITISH}"
	run_cmd git tag -a "$TAG" "$COMMITISH" -m "$MESSAGE"
	if ! $DRY_RUN; then
		log "Local tag ${TAG} created"
	fi
}

# Delete local tag if exists (ignore if not present)
delete_local_tag() {
	if local_tag_exists; then
		log "Deleting local tag ${TAG}"
		run_cmd git tag -d "$TAG"
		if ! $DRY_RUN; then
			log "Deleted local tag ${TAG}"
		fi
	else
		log "Local tag ${TAG} does not exist; skipping local delete"
	fi
}

# Delete remote tag if exists (ignore if not present)
delete_remote_tag() {
	if remote_tag_exists; then
		log "Deleting remote tag ${TAG} on ${REMOTE}"
		# push :refs/tags/<tag> deletes the remote tag
		run_cmd git push "$REMOTE" ":refs/tags/${TAG}"
		if ! $DRY_RUN; then
			log "Deleted remote tag ${TAG} on ${REMOTE}"
		fi
	else
		log "Remote tag ${TAG} does not exist on ${REMOTE}; skipping remote delete"
	fi
}

# Push tag to remote
push_tag_to_remote() {
	log "Pushing tag ${TAG} to ${REMOTE}"
	run_cmd git push "$REMOTE" "refs/tags/${TAG}"
	if ! $DRY_RUN; then
		log "Pushed tag ${TAG} to ${REMOTE}"
	fi
}

# Delete GitHub Release if exists (does not create new release)
delete_gh_release_if_exists() {
	if ! $GH_AVAILABLE; then
		log "gh CLI not available → skipping release handling"
		return 0
	fi

	if gh_release_exists; then
		log "GitHub Release for ${TAG} exists → deleting"
		run_cmd gh release delete "$TAG" -y
		if ! $DRY_RUN; then
			log "Deleted GitHub Release ${TAG}"
		fi
	else
		log "No existing GitHub Release for ${TAG} → nothing to delete"
	fi
}

# Main flow
if $DRY_RUN; then
	log "=== DRY-RUN MODE: No changes will be made ==="
fi
log "Start: handle tag=${TAG}, commitish=${COMMITISH}, remote=${REMOTE}"

# 1) Local: if exists -> delete, then create; if not exists -> create
if local_tag_exists; then
	log "Local tag ${TAG} exists"
	delete_local_tag
else
	log "Local tag ${TAG} does not exist"
fi

# Create local tag
create_local_tag

# 2) Remote: if exists -> delete, then push; if not exists -> push
if remote_tag_exists; then
	log "Remote tag ${TAG} exists on ${REMOTE}"
	delete_remote_tag
else
	log "Remote tag ${TAG} does not exist on ${REMOTE}"
fi

# Push newly created tag
push_tag_to_remote

# 3) GH release: delete if exists, do nothing if not
delete_gh_release_if_exists

log "Done."
