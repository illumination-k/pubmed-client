#!/bin/bash
# Download EGQuery API responses for test fixtures
#
# EGQuery returns XML with record counts for each NCBI database.
#
# NOTE: As of 2026, the NCBI EGQuery API (egquery.fcgi) returns HTTP 301 redirects
# to an internal Linkerd service mesh endpoint (ext-http-eutils.linkerd.ncbi.nlm.nih.gov)
# that is not publicly accessible. This means fixture download is currently not possible
# via curl from external networks.
#
# The EGQuery API parsing is covered by unit tests in:
#   pubmed-client/src/pubmed/client/egquery.rs
#
# If NCBI restores external access to EGQuery, re-run this script.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURES_DIR="$SCRIPT_DIR/../pubmed-client/tests/integration/test_data/api_responses/egquery"

echo "EGQuery fixtures:"
echo "  NOTE: NCBI EGQuery API redirects to an internal endpoint (Linkerd service mesh)"
echo "  that is not publicly accessible. Real fixture download is not currently possible."
echo ""
echo "  Verifying redirect behavior..."

BASE_URL="https://eutils.ncbi.nlm.nih.gov/entrez/eutils"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/egquery.fcgi?term=asthma")
REDIRECT_LOC=$(curl -s -D - -o /dev/null "${BASE_URL}/egquery.fcgi?term=asthma" 2>/dev/null | grep -i "^location:" | tr -d '\r')

echo "  HTTP status: $HTTP_CODE"
echo "  Redirect: $REDIRECT_LOC"

if [[ "$HTTP_CODE" == "301" ]]; then
	echo ""
	echo "  ✗ EGQuery API returns 301 redirect to internal endpoint - cannot download fixtures"
	echo "    EGQuery parsing tests use inline XML (see src/pubmed/client/egquery.rs unit tests)"
else
	echo ""
	echo "  EGQuery API may be accessible again. Attempting download..."
	mkdir -p "$FIXTURES_DIR"

	terms=("asthma" "covid-19" "crispr")
	for term in "${terms[@]}"; do
		filename=$(echo "$term" | tr '+' '_' | tr '-' '_')
		echo "  - EGQuery for '$term'..."
		curl -s "${BASE_URL}/egquery.fcgi?term=${term}" \
			>"$FIXTURES_DIR/egquery_${filename}.xml"
		sleep 0.4
	done
fi
