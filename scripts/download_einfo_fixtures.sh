#!/bin/bash
# Download EInfo API responses for test fixtures

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../tests/integration/test_data/api_responses/einfo"

echo "Creating EInfo fixtures directory..."
mkdir -p "$FIXTURES_DIR"

echo "Downloading EInfo API responses..."

# Database list
echo "  - Database list..."
curl -s "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/einfo.fcgi?retmode=json" \
    > "$FIXTURES_DIR/database_list.json"

# Common databases
databases=("pubmed" "pmc" "protein" "nucleotide" "genome" "structure" "taxonomy" "snp" "assembly" "bioproject")

for db in "${databases[@]}"; do
    echo "  - $db database info..."
    curl -s "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/einfo.fcgi?db=$db&retmode=json" \
        > "$FIXTURES_DIR/${db}_info.json"
    sleep 0.5  # Be respectful to NCBI servers
done

echo "Validating downloaded files..."
for file in "$FIXTURES_DIR"/*.json; do
    if [[ -s "$file" ]]; then
        # Check if it's valid JSON
        if jq empty "$file" 2>/dev/null; then
            echo "  ✓ $(basename "$file") - Valid JSON ($(wc -c < "$file") bytes)"
        else
            echo "  ✗ $(basename "$file") - Invalid JSON"
        fi
    else
        echo "  ✗ $(basename "$file") - Empty file"
    fi
done

echo "EInfo fixtures download complete!"
echo "Files saved to: $FIXTURES_DIR"
