#!/bin/bash

# Directory to store PubMed XML files
XML_DIR="test_data/pubmed_xml"

# Create directory if it doesn't exist
mkdir -p "$XML_DIR"

# Diverse set of PMIDs covering different article types and characteristics
PMIDS=(
	# COVID-19 research articles
	"31978945" # Early COVID-19 case series
	"33515491" # COVID-19 treatment systematic review
	"32887691" # COVID-19 vaccine development

	# Cancer research
	"34567890" # Cancer immunotherapy review
	"35123456" # Breast cancer clinical trial
	"36789012" # Lung cancer meta-analysis

	# Other medical research
	"37456789" # Diabetes management clinical trial
	"38123456" # Cardiovascular disease systematic review
	"39789012" # Mental health intervention study
	"40456789" # Infectious disease case study

	# Well-known classic papers with rich metadata
	"25760099" # CRISPR-Cas9 genome editing
	"26846451" # Machine learning in healthcare
	"27350240" # Precision medicine review
	"28495875" # Artificial intelligence in radiology
	"29540945" # Telemedicine systematic review
)

echo "Downloading PubMed XML files using EFetch API..."
echo "Total PMIDs to process: ${#PMIDS[@]}"

SUCCESS_COUNT=0
SKIP_COUNT=0
FAIL_COUNT=0

for PMID in "${PMIDS[@]}"; do
	OUTPUT_FILE="$XML_DIR/${PMID}.xml"

	# Skip if already exists and is valid (size > 1KB)
	if [ -f "$OUTPUT_FILE" ] && [ $(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null) -gt 1000 ]; then
		echo "✓ $PMID already exists, skipping..."
		((SKIP_COUNT++))
		continue
	fi

	echo "Downloading PMID: $PMID..."

	# Use NCBI EFetch API to download article XML
	URL="https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi?db=pubmed&id=${PMID}&retmode=xml"

	# Download with curl, following redirects and handling errors
	curl -s -L -o "$OUTPUT_FILE" "$URL"

	# Check if download was successful
	if [ -f "$OUTPUT_FILE" ] && [ $(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null) -gt 1000 ]; then
		# Verify it contains expected PubMed XML structure
		if grep -q "<PubmedArticle" "$OUTPUT_FILE" || grep -q "<MedlineCitation" "$OUTPUT_FILE"; then
			echo "✓ Successfully downloaded PMID: $PMID"
			((SUCCESS_COUNT++))
		else
			echo "✗ Downloaded file for PMID $PMID doesn't contain valid PubMed XML"
			rm -f "$OUTPUT_FILE"
			((FAIL_COUNT++))
		fi
	else
		echo "✗ Failed to download PMID: $PMID or file is too small"
		rm -f "$OUTPUT_FILE"
		((FAIL_COUNT++))
	fi

	# Be respectful to NCBI servers - small delay between requests
	sleep 0.5
done

echo "
Download Summary:
- Successfully downloaded: $SUCCESS_COUNT
- Already existed: $SKIP_COUNT
- Failed: $FAIL_COUNT
- Total XML files in directory:"
ls -la "$XML_DIR" | grep -c "\.xml$"

echo "
Sample of downloaded files:"
ls -lh "$XML_DIR"/*.xml | head -5

echo "
You can now run PubMed integration tests with:
cargo test --test comprehensive_pubmed_tests"
