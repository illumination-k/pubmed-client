#!/bin/bash
set -e

# Automated Stub Generation Script for pubmed-client-py
# =====================================================
#
# This script provides a simple MVP for semi-automated stub generation.
# It uses runtime introspection to extract class/method signatures,
# which can then be manually enhanced with type annotations.
#
# Usage:
#   ./scripts/generate_stubs.sh
#
# Output:
#   pubmed_client_auto.pyi - Auto-generated stub file (no type annotations)
#
# Workflow:
#   1. Build the Python package with maturin
#   2. Generate baseline stubs using runtime introspection
#   3. Compare with existing manual stub file
#   4. Manually merge type annotations if needed
#

cd "$(dirname "$0")/.."

echo "🔨 Building Python package with maturin..."
uv run --with maturin maturin develop --quiet

echo "📝 Generating stubs using runtime introspection..."
uv run python scripts/inspect_module.py pubmed_client pubmed_client_auto.pyi

echo "✅ Generated: pubmed_client_auto.pyi"
echo ""
echo "📊 Comparison:"
echo "  Manual stub:      $(wc -l < pubmed_client.pyi) lines"
echo "  Auto-generated:   $(wc -l < pubmed_client_auto.pyi) lines"
echo ""
echo "💡 Next steps:"
echo "  1. Review: diff pubmed_client.pyi pubmed_client_auto.pyi"
echo "  2. Check for missing classes/methods in manual stub"
echo "  3. Add type annotations to auto-generated stubs if needed"
echo ""
echo "📝 Note: Auto-generated stubs lack type annotations and property fields."
echo "   Use this as a baseline to verify completeness of manual stubs."
