#!/bin/bash

# Bulk download PMC Open Access XML files for testing and benchmarking.
#
# Usage:
#   ./scripts/download_bulk_oa_xml.sh                     # Download default set
#   ./scripts/download_bulk_oa_xml.sh --search "cancer" --count 50
#   ./scripts/download_bulk_oa_xml.sh --list ids.txt      # One PMC ID per line
#   ./scripts/download_bulk_oa_xml.sh --range 11000000 11000100
#
# Options:
#   --search QUERY    Search PMC OA subset and download results
#   --count N         Number of articles to download (default: 100, max: 1000)
#   --list FILE       Read PMC IDs from file (one per line)
#   --range MIN MAX   Download PMC IDs in numeric range (checks OA availability)
#   --outdir DIR      Output directory (default: test_data/pmc_xml)
#   --api-key KEY     NCBI API key (or set NCBI_API_KEY env var)
#   --dry-run         Show what would be downloaded without downloading
#   --force           Re-download even if file already exists
#   --help            Show this help message

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================

EFETCH_BASE="https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi"
ESEARCH_BASE="https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi"
OA_API_BASE="https://www.ncbi.nlm.nih.gov/pmc/utils/oa/oa.fcgi"

DEFAULT_OUTDIR="test_data/pmc_xml"
DEFAULT_COUNT=100
MAX_COUNT=1000
MIN_FILE_SIZE=1000  # Minimum valid XML file size in bytes

# NCBI rate limit: 3 req/s without key, 10 req/s with key
DELAY_WITHOUT_KEY=0.4
DELAY_WITH_KEY=0.12

# ============================================================================
# Default PMC IDs — curated diverse set for testing
# ============================================================================

# Diverse set covering different article types, sizes, and structures
DEFAULT_PMC_IDS=(
    # Articles with rich figure content
    "PMC7906746"   # Lancet COVID-19 article, figures + tables
    "PMC10455298"  # Multiple figures with sub-panels
    "PMC11084381"  # 2024 article with supplementary materials

    # Large articles (stress test for parser)
    "PMC10000000"  # Chicago Medical Examiner
    "PMC9500000"   # Multi-section article
    "PMC9000000"   # Complex structure
    "PMC7500000"   # Older format
    "PMC6500000"   # Tables + references

    # Systematic reviews / meta-analyses (complex structure)
    "PMC10777901"  # Sleep health promotion interventions
    "PMC10653940"  # School-based interventions review
    "PMC10821037"  # Health workforce interventions

    # Open access with various licenses
    "PMC10906259"  # Open-access publishing comparison
    "PMC10618641"  # Journalists' use of OA research
    "PMC10767945"  # 2024 NAR database issue

    # Different publication years (parser compatibility)
    "PMC10500000"  # 2023
    "PMC10400000"  # 2023
    "PMC10300000"  # 2023
    "PMC10200000"  # 2023
    "PMC5000000"   # 2016
    "PMC4500000"   # 2015
    "PMC3500000"   # 2012

    # Bioinformatics / computational articles (code blocks, algorithms)
    "PMC4402560"   # Bioinformatics tool paper
    "PMC3245950"   # Database / web server paper
    "PMC6612828"   # Machine learning methods

    # Clinical trials (structured abstracts, CONSORT)
    "PMC5334499"   # Randomized clinical trial
    "PMC6830270"   # Clinical trial with CONSORT diagram

    # Case reports (short, simple structure)
    "PMC8255885"   # Case report
    "PMC7245266"   # Brief case study

    # Review articles (long, many references)
    "PMC8001632"   # Comprehensive review
    "PMC7614489"   # Narrative review

    # Articles with supplementary materials
    "PMC6097007"   # Supplementary tables/figures
    "PMC5766653"   # Additional data files

    # Non-English articles with English abstract (encoding tests)
    "PMC4789478"   # Multi-language support

    # Retracted articles (edge case handling)
    # "PMC6396722" # Known retracted — uncomment if needed

    # Very recent articles (2024-2025)
    "PMC10906259"
    "PMC11084381"
)

# ============================================================================
# Helpers
# ============================================================================

usage() {
    sed -n '3,16p' "$0" | sed 's/^# \?//'
    exit 0
}

log_info()  { echo "[INFO]  $*"; }
log_ok()    { echo "[OK]    $*"; }
log_warn()  { echo "[WARN]  $*"; }
log_error() { echo "[ERROR] $*" >&2; }

# Get file size portably (Linux + macOS)
file_size() {
    stat -c%s "$1" 2>/dev/null || stat -f%z "$1" 2>/dev/null || echo 0
}

# ============================================================================
# Parse arguments
# ============================================================================

MODE="default"       # default | search | list | range
SEARCH_QUERY=""
COUNT=$DEFAULT_COUNT
LIST_FILE=""
RANGE_MIN=0
RANGE_MAX=0
OUTDIR="$DEFAULT_OUTDIR"
API_KEY="${NCBI_API_KEY:-}"
DRY_RUN=false
FORCE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --search)  MODE="search"; SEARCH_QUERY="$2"; shift 2 ;;
        --count)   COUNT="$2"; shift 2 ;;
        --list)    MODE="list"; LIST_FILE="$2"; shift 2 ;;
        --range)   MODE="range"; RANGE_MIN="$2"; RANGE_MAX="$3"; shift 3 ;;
        --outdir)  OUTDIR="$2"; shift 2 ;;
        --api-key) API_KEY="$2"; shift 2 ;;
        --dry-run) DRY_RUN=true; shift ;;
        --force)   FORCE=true; shift ;;
        --help|-h) usage ;;
        *)         log_error "Unknown option: $1"; usage ;;
    esac
done

# Clamp count
if (( COUNT > MAX_COUNT )); then
    log_warn "Count capped to $MAX_COUNT"
    COUNT=$MAX_COUNT
fi

# Set rate limit delay
if [[ -n "$API_KEY" ]]; then
    DELAY=$DELAY_WITH_KEY
    API_PARAMS="&api_key=${API_KEY}"
    log_info "Using API key (10 req/s rate limit)"
else
    DELAY=$DELAY_WITHOUT_KEY
    API_PARAMS=""
    log_info "No API key (3 req/s rate limit). Set NCBI_API_KEY for faster downloads."
fi

mkdir -p "$OUTDIR"

# ============================================================================
# Collect PMC IDs based on mode
# ============================================================================

PMC_IDS=()

case "$MODE" in
    default)
        # Remove duplicates from default list
        declare -A seen
        for id in "${DEFAULT_PMC_IDS[@]}"; do
            if [[ -z "${seen[$id]:-}" ]]; then
                PMC_IDS+=("$id")
                seen[$id]=1
            fi
        done
        log_info "Using curated set of ${#PMC_IDS[@]} PMC IDs"
        ;;

    search)
        log_info "Searching PMC OA subset for: '$SEARCH_QUERY' (retmax=$COUNT)"
        SEARCH_URL="${ESEARCH_BASE}?db=pmc&term=${SEARCH_QUERY}+AND+open+access[filter]&retmax=${COUNT}&retmode=json${API_PARAMS}"
        SEARCH_RESULT=$(curl -s "$SEARCH_URL")

        # Extract PMC IDs from JSON response
        mapfile -t RAW_IDS < <(echo "$SEARCH_RESULT" | grep -oP '"(\d+)"' | tr -d '"' | head -n "$COUNT")

        if [[ ${#RAW_IDS[@]} -eq 0 ]]; then
            log_error "No results found for query: '$SEARCH_QUERY'"
            exit 1
        fi

        for id in "${RAW_IDS[@]}"; do
            PMC_IDS+=("PMC${id}")
        done
        log_info "Found ${#PMC_IDS[@]} articles"
        ;;

    list)
        if [[ ! -f "$LIST_FILE" ]]; then
            log_error "File not found: $LIST_FILE"
            exit 1
        fi
        while IFS= read -r line || [[ -n "$line" ]]; do
            line=$(echo "$line" | tr -d '[:space:]')
            [[ -z "$line" || "$line" == \#* ]] && continue
            # Normalize: add PMC prefix if missing
            if [[ "$line" =~ ^[0-9]+$ ]]; then
                PMC_IDS+=("PMC${line}")
            else
                PMC_IDS+=("$line")
            fi
        done < "$LIST_FILE"
        log_info "Loaded ${#PMC_IDS[@]} PMC IDs from $LIST_FILE"
        ;;

    range)
        if (( RANGE_MIN >= RANGE_MAX )); then
            log_error "Invalid range: $RANGE_MIN >= $RANGE_MAX"
            exit 1
        fi
        RANGE_SIZE=$((RANGE_MAX - RANGE_MIN))
        if (( RANGE_SIZE > MAX_COUNT )); then
            log_warn "Range too large ($RANGE_SIZE). Capping to $MAX_COUNT"
            RANGE_MAX=$((RANGE_MIN + MAX_COUNT))
        fi
        for (( i=RANGE_MIN; i<RANGE_MAX; i++ )); do
            PMC_IDS+=("PMC${i}")
        done
        log_info "Generated ${#PMC_IDS[@]} PMC IDs from range $RANGE_MIN-$RANGE_MAX"
        ;;
esac

TOTAL=${#PMC_IDS[@]}
if (( TOTAL == 0 )); then
    log_error "No PMC IDs to download"
    exit 1
fi

# ============================================================================
# Download
# ============================================================================

SUCCESS_COUNT=0
SKIP_COUNT=0
FAIL_COUNT=0
TOTAL_BYTES=0

log_info "Output directory: $OUTDIR"
log_info "Downloading $TOTAL PMC XML files..."
echo "---"

for (( idx=0; idx<TOTAL; idx++ )); do
    PMC_ID="${PMC_IDS[$idx]}"
    NUMERIC_PART="${PMC_ID#PMC}"
    OUTPUT_FILE="${OUTDIR}/${PMC_ID}.xml"
    PROGRESS="[$((idx+1))/${TOTAL}]"

    # Skip existing files unless --force
    if [[ -f "$OUTPUT_FILE" ]] && ! $FORCE; then
        SIZE=$(file_size "$OUTPUT_FILE")
        if (( SIZE > MIN_FILE_SIZE )); then
            echo "${PROGRESS} SKIP  ${PMC_ID} (already exists, $(numfmt --to=iec "$SIZE" 2>/dev/null || echo "${SIZE}B"))"
            ((SKIP_COUNT++))
            continue
        fi
    fi

    if $DRY_RUN; then
        echo "${PROGRESS} DRY   ${PMC_ID}"
        continue
    fi

    # Download using EFetch API (same endpoint as pubmed-client uses)
    URL="${EFETCH_BASE}?db=pmc&id=PMC${NUMERIC_PART}&retmode=xml${API_PARAMS}"

    HTTP_CODE=$(curl -s -w "%{http_code}" -o "$OUTPUT_FILE" -L "$URL")

    if [[ "$HTTP_CODE" != "200" ]]; then
        echo "${PROGRESS} FAIL  ${PMC_ID} (HTTP ${HTTP_CODE})"
        rm -f "$OUTPUT_FILE"
        ((FAIL_COUNT++))
        sleep "$DELAY"
        continue
    fi

    SIZE=$(file_size "$OUTPUT_FILE")

    # Validate: must be > minimum size and contain PMC XML markers
    if (( SIZE < MIN_FILE_SIZE )); then
        echo "${PROGRESS} FAIL  ${PMC_ID} (too small: ${SIZE} bytes)"
        rm -f "$OUTPUT_FILE"
        ((FAIL_COUNT++))
    elif ! grep -q "<article\b\|<pmc-articleset\|<body>" "$OUTPUT_FILE" 2>/dev/null; then
        # Check if it's an error response
        if grep -q "<ERROR>" "$OUTPUT_FILE" 2>/dev/null; then
            ERROR_MSG=$(grep -oP '(?<=<ERROR>).*?(?=</ERROR>)' "$OUTPUT_FILE" 2>/dev/null || echo "unknown")
            echo "${PROGRESS} FAIL  ${PMC_ID} (API error: ${ERROR_MSG})"
        else
            echo "${PROGRESS} FAIL  ${PMC_ID} (invalid XML content)"
        fi
        rm -f "$OUTPUT_FILE"
        ((FAIL_COUNT++))
    else
        SIZE_HUMAN=$(numfmt --to=iec "$SIZE" 2>/dev/null || echo "${SIZE}B")
        echo "${PROGRESS} OK    ${PMC_ID} (${SIZE_HUMAN})"
        ((SUCCESS_COUNT++))
        ((TOTAL_BYTES += SIZE))
    fi

    # Rate limiting
    sleep "$DELAY"
done

# ============================================================================
# Summary
# ============================================================================

echo "---"
echo ""
echo "Download Summary"
echo "  Downloaded:  $SUCCESS_COUNT"
echo "  Skipped:     $SKIP_COUNT"
echo "  Failed:      $FAIL_COUNT"
echo "  Total size:  $(numfmt --to=iec "$TOTAL_BYTES" 2>/dev/null || echo "${TOTAL_BYTES} bytes")"
echo ""

FILE_COUNT=$(find "$OUTDIR" -name "*.xml" -type f 2>/dev/null | wc -l)
echo "  Files in ${OUTDIR}: ${FILE_COUNT}"
echo ""

if (( SUCCESS_COUNT > 0 || SKIP_COUNT > 0 )); then
    echo "Run parser benchmarks with:"
    echo "  cargo bench -p pubmed-parser"
    echo ""
    echo "Run integration tests with:"
    echo "  cargo test --test comprehensive_pmc_tests -p pubmed-client"
fi
