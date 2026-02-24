#!/bin/bash
# Download ECitMatch API responses for test fixtures
#
# ECitMatch returns pipe-delimited plain text.
# Format: journal|year|volume|first_page|author|key|pmid

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../pubmed-client/tests/integration/test_data/api_responses/ecitmatch"

echo "Creating ECitMatch fixtures directory..."
mkdir -p "$FIXTURES_DIR"

echo "Downloading ECitMatch API responses..."

BASE_URL="https://eutils.ncbi.nlm.nih.gov/entrez/eutils"

# All-found citations
# Two classic microbiology papers with known PMIDs
echo "  - All-found citations (2 classic papers)..."
BDATA="proc+natl+acad+sci+u+s+a|1991|88|3248|mann+bj|Art1|%0Dscience|1987|235|182|palmenberg+ac|Art2|"
curl -s "${BASE_URL}/ecitmatch.cgi?db=pubmed&retmode=xml&bdata=${BDATA}" \
	>"$FIXTURES_DIR/citmatch_found.txt"
sleep 0.5

# Mixed citations: one found, one not found, one ambiguous, one found
echo "  - Mixed citations (found/not-found/ambiguous)..."
BDATA_MIXED="proc+natl+acad+sci+u+s+a|1991|88|3248|mann+bj|Art1|%0Dfake+journal|2000|1|1|nobody|ref2|%0Dn+engl+j+med|2020|382|727|zhu+n|Art4|"
curl -s "${BASE_URL}/ecitmatch.cgi?db=pubmed&retmode=xml&bdata=${BDATA_MIXED}" \
	>"$FIXTURES_DIR/citmatch_mixed.txt"
sleep 0.5

# Single citation lookup (well-known COVID-19 paper)
echo "  - Single citation lookup (COVID-19 paper)..."
BDATA_SINGLE="n+engl+j+med|2020|382|727|zhu+n|covid1|"
curl -s "${BASE_URL}/ecitmatch.cgi?db=pubmed&retmode=xml&bdata=${BDATA_SINGLE}" \
	>"$FIXTURES_DIR/citmatch_single.txt"
sleep 0.5

echo "Validating downloaded files..."
for file in "$FIXTURES_DIR"/*.txt; do
	if [[ -s "$file" ]]; then
		echo "  ✓ $(basename "$file") - $(wc -l <"$file") lines, $(wc -c <"$file") bytes"
		echo "    Content: $(cat "$file")"
	else
		echo "  ✗ $(basename "$file") - Empty file"
	fi
done

echo "ECitMatch fixtures download complete!"
echo "Files saved to: $FIXTURES_DIR"
