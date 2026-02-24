#!/bin/bash
# Download ESearch API responses for test fixtures

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../pubmed-client/tests/integration/test_data/api_responses/esearch"

echo "Creating ESearch fixtures directory..."
mkdir -p "$FIXTURES_DIR"

echo "Downloading ESearch API responses..."

BASE_URL="https://eutils.ncbi.nlm.nih.gov/entrez/eutils"

# Basic search for covid-19 (5 results)
echo "  - Basic covid-19 search..."
curl -s "${BASE_URL}/esearch.fcgi?db=pubmed&term=covid-19&retmax=5&retmode=json" \
	>"$FIXTURES_DIR/search_covid19.json"
sleep 0.5

# Search with history server enabled
echo "  - Asthma search with history server..."
curl -s "${BASE_URL}/esearch.fcgi?db=pubmed&term=asthma&retmax=10&retmode=json&usehistory=y" \
	>"$FIXTURES_DIR/search_with_history.json"
sleep 0.5

# Empty results (nonexistent term)
echo "  - Empty results search..."
curl -s "${BASE_URL}/esearch.fcgi?db=pubmed&term=xyzzy_nonexistent_98765qwerty&retmax=5&retmode=json" \
	>"$FIXTURES_DIR/search_empty.json"
sleep 0.5

# Search with sort by publication date
echo "  - CRISPR search sorted by date..."
curl -s "${BASE_URL}/esearch.fcgi?db=pubmed&term=crispr+genome+editing&retmax=5&retmode=json&sort=pub+date" \
	>"$FIXTURES_DIR/search_crispr_sorted.json"
sleep 0.5

echo "Validating downloaded files..."
for file in "$FIXTURES_DIR"/*.json; do
	if [[ -s "$file" ]]; then
		if jq empty "$file" 2>/dev/null; then
			echo "  ✓ $(basename "$file") - Valid JSON ($(wc -c <"$file") bytes)"
		else
			echo "  ✗ $(basename "$file") - Invalid JSON"
		fi
	else
		echo "  ✗ $(basename "$file") - Empty file"
	fi
done

echo "ESearch fixtures download complete!"
echo "Files saved to: $FIXTURES_DIR"
