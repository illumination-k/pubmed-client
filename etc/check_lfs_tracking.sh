#!/usr/bin/env bash
#
# Check if files configured in .gitattributes are properly tracked as Git LFS pointers
#

set -euo pipefail

# Check git-lfs installation
if ! command -v git-lfs &> /dev/null; then
    echo "Warning: git-lfs is not installed, skipping LFS tracking check"
    echo "Note: This check will run in CI/CD environments where git-lfs is available"
    exit 0
fi

# Get repository root
cd "$(git rev-parse --show-toplevel)"

# Check .gitattributes exists
if [[ ! -f .gitattributes ]]; then
    echo "Error: .gitattributes not found"
    exit 1
fi

echo "=== Git LFS Tracking Check ==="
echo

# Read LFS patterns from .gitattributes
echo "LFS patterns:"
grep "filter=lfs" .gitattributes | awk '{print "  " $1}'
echo

# Counters
total=0
ok=0
ng=0

# Check each pattern
while IFS= read -r line; do
    # Skip comments and empty lines
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "$line" ]] && continue

    # Get pattern if line contains filter=lfs
    if [[ "$line" =~ filter=lfs ]]; then
        pattern=$(echo "$line" | awk '{print $1}')

        # Extract directory and extension from pattern like "dir/**/*.ext"
        if [[ "$pattern" == *"/**/"* ]]; then
            dir="${pattern%%/**/*}"
            ext="${pattern##*.}"

            # Find files
            if [[ -d "$dir" ]]; then
                while IFS= read -r file; do
                    total=$((total + 1))

                    # Check if file is LFS pointer in Git repository (not working directory)
                    if git cat-file -p HEAD:"$file" 2>/dev/null | head -n 1 | grep -q "^version https://git-lfs.github.com/spec/v1"; then
                        echo "✓ $file"
                        ok=$((ok + 1))
                    else
                        echo "✗ $file"
                        ng=$((ng + 1))
                    fi
                done < <(find "$dir" -type f -name "*.$ext")
            fi
        fi
    fi
done < .gitattributes

# Summary
echo
echo "=== Summary ==="
echo "Total: $total"
echo "LFS pointers: $ok"
echo "NOT LFS pointers: $ng"

# Exit with error if any files are not LFS pointers
[[ $ng -gt 0 ]] && exit 1
exit 0
