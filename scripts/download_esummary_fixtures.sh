#!/bin/bash
# Download ESummary API responses for test fixtures

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../pubmed-client/tests/integration/test_data/api_responses/esummary"

echo "Creating ESummary fixtures directory..."
mkdir -p "$FIXTURES_DIR"

echo "Downloading ESummary API responses..."

BASE_URL="https://eutils.ncbi.nlm.nih.gov/entrez/eutils"

# Well-known PMIDs for testing
pmids=(
	"31978945" # COVID-19 research (Zhu N et al., NEJM 2020)
	"33515491" # SARS-CoV-2 cell entry (Hoffmann M et al., Cell 2020)
	"32887691" # ML in medicine
	"25760099" # CRISPR research
)

# Single article summaries
for pmid in "${pmids[@]}"; do
	echo "  - Summary for PMID $pmid..."
	curl -s "${BASE_URL}/esummary.fcgi?db=pubmed&id=${pmid}&retmode=json" \
		>"$FIXTURES_DIR/summary_${pmid}.json"
	sleep 0.4
done

# Multiple PMIDs in one request
echo "  - Multiple PMIDs summary (31978945,33515491)..."
curl -s "${BASE_URL}/esummary.fcgi?db=pubmed&id=31978945,33515491&retmode=json" \
	>"$FIXTURES_DIR/summaries_multiple.json"
sleep 0.4

echo "  - Multiple PMIDs summary (4 articles)..."
curl -s "${BASE_URL}/esummary.fcgi?db=pubmed&id=31978945,33515491,32887691,25760099&retmode=json" \
	>"$FIXTURES_DIR/summaries_four.json"
sleep 0.4

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

echo "ESummary fixtures download complete!"
echo "Files saved to: $FIXTURES_DIR"
