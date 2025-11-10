#!/bin/bash

# Directory to store XML files
XML_DIR="tests/test_data/pmc_xml"

# All verified PMC IDs from our searches
PMC_IDS=(
	# Already downloaded successfully
	"PMC10000000" # Chicago Medical Examiner article
	"PMC9500000"
	"PMC9000000"
	"PMC7500000"
	"PMC6500000"
	"PMC6000000"

	# Verified from 2024 research
	"PMC10777901" # Sleep health promotion interventions
	"PMC10653940" # School-based interventions for adolescent mental health
	"PMC10821037" # Health workforce interventions for immunisation

	# Open access articles from 2024
	"PMC10906259" # Does it pay to pay? Open-access publishing comparison
	"PMC10618641" # Making science public: journalists' use of Open Access research
	"PMC10618038" # Open access: evolution not revolution
	"PMC10767945" # 2024 Nucleic Acids Research database issue
	"PMC10876690" # Effects of open access publishing in Neuropsychopharmacology

	# Additional verified PMC IDs from 2023
	"PMC10500000" # Digital care program for chronic knee pain
	"PMC10400000" # CT-Scan diagnostic accuracy for renal stones
	"PMC10300000" # Exercise program for hemodialysis patients
	"PMC10200000" # Time-restricted eating and circadian rhythms

	# More PMC IDs to try
	"PMC5000000"
	"PMC4500000"
	"PMC4000000"
	"PMC3500000"
)

echo "Downloading verified PMC XML files..."
echo "Total IDs to process: ${#PMC_IDS[@]}"

SUCCESS_COUNT=0
SKIP_COUNT=0
FAIL_COUNT=0

for PMC_ID in "${PMC_IDS[@]}"; do
	OUTPUT_FILE="$XML_DIR/${PMC_ID}.xml"

	# Skip if already exists and is valid
	if [ -f "$OUTPUT_FILE" ] && [ $(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null) -gt 1000 ]; then
		echo "✓ $PMC_ID already exists, skipping..."
		((SKIP_COUNT++))
		continue
	fi

	echo "Downloading $PMC_ID..."
	URL="https://www.ncbi.nlm.nih.gov/pmc/oai/oai.cgi?verb=GetRecord&identifier=oai:pubmedcentral.nih.gov:${PMC_ID#PMC}&metadataPrefix=pmc"

	curl -s -o "$OUTPUT_FILE" "$URL"

	# Check if download was successful (file size > 1KB)
	if [ -f "$OUTPUT_FILE" ] && [ $(stat -f%z "$OUTPUT_FILE" 2>/dev/null || stat -c%s "$OUTPUT_FILE" 2>/dev/null) -gt 1000 ]; then
		echo "✓ Successfully downloaded $PMC_ID"
		((SUCCESS_COUNT++))
	else
		echo "✗ Failed to download $PMC_ID or file is too small"
		rm -f "$OUTPUT_FILE"
		((FAIL_COUNT++))
	fi

	# Small delay to be respectful to the server
	sleep 1
done

echo "
Download Summary:
- Successfully downloaded: $SUCCESS_COUNT
- Already existed: $SKIP_COUNT
- Failed: $FAIL_COUNT
- Total XML files:"
ls -la "$XML_DIR" | grep -c "\.xml$"
