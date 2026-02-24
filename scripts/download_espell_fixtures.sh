#!/bin/bash
# Download ESpell API responses for test fixtures
#
# ESpell returns XML with spelling suggestions for search terms.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../pubmed-client/tests/integration/test_data/api_responses/espell"

echo "Creating ESpell fixtures directory..."
mkdir -p "$FIXTURES_DIR"

echo "Downloading ESpell API responses..."

BASE_URL="https://eutils.ncbi.nlm.nih.gov/entrez/eutils"

# Misspelled terms expected to have corrections
echo "  - Misspelled 'asthmaa' (expected correction: asthma)..."
curl -s "${BASE_URL}/espell.fcgi?db=pubmed&term=asthmaa" \
	>"$FIXTURES_DIR/espell_asthmaa.xml"
sleep 0.4

echo "  - Misspelled 'alergies' (expected correction: allergies)..."
curl -s "${BASE_URL}/espell.fcgi?db=pubmed&term=alergies" \
	>"$FIXTURES_DIR/espell_alergies.xml"
sleep 0.4

echo "  - Multiple misspellings 'asthmaa+OR+alergies'..."
curl -s "${BASE_URL}/espell.fcgi?db=pubmed&term=asthmaa+OR+alergies" \
	>"$FIXTURES_DIR/espell_multiple_corrections.xml"
sleep 0.4

echo "  - Misspelled 'fiberblast' (expected correction: fibroblast)..."
curl -s "${BASE_URL}/espell.fcgi?db=pubmed&term=fiberblast" \
	>"$FIXTURES_DIR/espell_fiberblast.xml"
sleep 0.4

# Correctly spelled term (no corrections expected)
echo "  - Correctly spelled 'asthma' (no corrections expected)..."
curl -s "${BASE_URL}/espell.fcgi?db=pubmed&term=asthma" \
	>"$FIXTURES_DIR/espell_correct.xml"
sleep 0.4

# Term with no results (unknown word)
echo "  - Unknown term 'xyzzyqwerty' (no corrections expected)..."
curl -s "${BASE_URL}/espell.fcgi?db=pubmed&term=xyzzyqwerty" \
	>"$FIXTURES_DIR/espell_unknown.xml"
sleep 0.4

echo "Validating downloaded files..."
for file in "$FIXTURES_DIR"/*.xml; do
	if [[ -s "$file" ]]; then
		if grep -q "<eSpellResult>" "$file"; then
			corrected=$(grep -o '<CorrectedQuery>[^<]*</CorrectedQuery>' "$file" | sed 's/<[^>]*>//g')
			echo "  ✓ $(basename "$file") - Valid XML, CorrectedQuery: '${corrected}'"
		else
			echo "  ✗ $(basename "$file") - Missing <eSpellResult> element"
			echo "    Content: $(head -5 "$file")"
		fi
	else
		echo "  ✗ $(basename "$file") - Empty file"
	fi
done

echo "ESpell fixtures download complete!"
echo "Files saved to: $FIXTURES_DIR"
