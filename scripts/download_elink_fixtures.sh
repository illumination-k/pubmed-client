#!/bin/bash
# Download ELink API responses for test fixtures

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../tests/integration/test_data/api_responses/elink"

echo "Creating ELink fixtures directory..."
mkdir -p "$FIXTURES_DIR"

echo "Downloading ELink API responses..."

# Well-known PMIDs for testing
pmids=(
	"31978945" # COVID-19 research
	"33515491" # Cancer treatment
	"32887691" # Machine learning in medicine
	"25760099" # CRISPR research
	"28495875" # Alzheimer's research
	"29540945" # Diabetes treatment
	"27350240" # Immunotherapy
	"26846451" # Genomics
	"34567890" # Bioinformatics
	"35123456" # Microbiome
)

# Related articles (pubmed_pubmed)
for pmid in "${pmids[@]}"; do
	echo "  - Related articles for PMID $pmid..."
	curl -s "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/elink.fcgi?dbfrom=pubmed&db=pubmed&id=$pmid&retmode=json" \
		>"$FIXTURES_DIR/related_${pmid}.json"
	sleep 0.4
done

# PMC links (pubmed_pmc)
for pmid in "${pmids[@]}"; do
	echo "  - PMC links for PMID $pmid..."
	curl -s "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/elink.fcgi?dbfrom=pubmed&db=pmc&id=$pmid&retmode=json" \
		>"$FIXTURES_DIR/pmc_links_${pmid}.json"
	sleep 0.4
done

# Citations (pubmed_pubmed_citedin)
for pmid in "${pmids[@]}"; do
	echo "  - Citations for PMID $pmid..."
	curl -s "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/elink.fcgi?dbfrom=pubmed&db=pubmed&id=$pmid&linkname=pubmed_pubmed_citedin&retmode=json" \
		>"$FIXTURES_DIR/citations_${pmid}.json"
	sleep 0.4
done

# Multiple PMIDs tests
echo "  - Multiple PMIDs related articles..."
multi_pmids="31978945,33515491,32887691"
curl -s "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/elink.fcgi?dbfrom=pubmed&db=pubmed&id=$multi_pmids&retmode=json" \
	>"$FIXTURES_DIR/related_multiple.json"
sleep 0.4

echo "  - Multiple PMIDs PMC links..."
curl -s "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/elink.fcgi?dbfrom=pubmed&db=pmc&id=$multi_pmids&retmode=json" \
	>"$FIXTURES_DIR/pmc_links_multiple.json"
sleep 0.4

echo "  - Multiple PMIDs citations..."
curl -s "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/elink.fcgi?dbfrom=pubmed&db=pubmed&id=$multi_pmids&linkname=pubmed_pubmed_citedin&retmode=json" \
	>"$FIXTURES_DIR/citations_multiple.json"

echo "Validating downloaded files..."
for file in "$FIXTURES_DIR"/*.json; do
	if [[ -s "$file" ]]; then
		# Check if it's valid JSON
		if jq empty "$file" 2>/dev/null; then
			echo "  ✓ $(basename "$file") - Valid JSON ($(wc -c <"$file") bytes)"
		else
			echo "  ✗ $(basename "$file") - Invalid JSON"
		fi
	else
		echo "  ✗ $(basename "$file") - Empty file"
	fi
done

echo "ELink fixtures download complete!"
echo "Files saved to: $FIXTURES_DIR"
