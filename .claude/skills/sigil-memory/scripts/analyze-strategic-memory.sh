#!/bin/bash

# Strategic Memory Analysis for Resource-Constrained Optimization
# Analyzes current memory usage and strategic value for OODA loop optimization

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGSEQ_API_BASE="${LOGSEQ_API_BASE:-http://localhost:12315}"
MEMORY_CONSTRAINT="${1:-1.44MB}"

show_help() {
    cat << 'EOF'
Strategic Memory Analysis for Resource-Constrained Optimization

USAGE:
    analyze-strategic-memory.sh [OPTIONS]

OPTIONS:
    --constraint SIZE   Memory constraint (default: 1.44MB)
    --graph-path PATH   Path to logseq graph (default: /media/dontpanic/1112-15D8)
    --api-base URL      Logseq API base URL (default: http://localhost:12315)
    --output FORMAT     Output format: json, summary, detailed (default: detailed)
    --help              Show this help

DESCRIPTION:
    Analyzes current memory usage and strategic value for OODA loop optimization.
    Applies Brier scoring methodology to memory allocation decisions.

EXAMPLES:
    analyze-strategic-memory.sh --constraint "1.44MB" --output detailed
    analyze-strategic-memory.sh --graph-path "/strategic/memory" --output json
    LOGSEQ_API_TOKEN=xxx analyze-strategic-memory.sh --constraint "2MB"
EOF
}

# Parse arguments
GRAPH_PATH="/media/dontpanic/1112-15D8"
OUTPUT_FORMAT="detailed"

while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            show_help
            exit 0
            ;;
        --constraint)
            MEMORY_CONSTRAINT="$2"
            shift 2
            ;;
        --graph-path)
            GRAPH_PATH="$2"
            shift 2
            ;;
        --api-base)
            LOGSEQ_API_BASE="$2"
            shift 2
            ;;
        --output)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            show_help >&2
            exit 1
            ;;
    esac
done

echo "📊 Strategic Memory Analysis"
echo "============================="
echo "Graph: $GRAPH_PATH"
echo "Constraint: $MEMORY_CONSTRAINT"
echo "API: $LOGSEQ_API_BASE"
echo ""

# Check API connectivity
if [[ -n "$LOGSEQ_API_TOKEN" ]]; then
    echo "🔍 Analyzing memory usage via Logseq API..."

    # Get total pages
    TOTAL_PAGES=$(curl -s -X POST "$LOGSEQ_API_BASE/api" \
        -H "Authorization: Bearer $LOGSEQ_API_TOKEN" \
        -H "Content-Type: application/json" \
        -d '{"method": "logseq.Editor.getAllPages", "args": []}' | \
        jq '. | length' 2>/dev/null || echo "0")

    echo "📚 Total Pages: $TOTAL_PAGES"
else
    echo "⚠️  LOGSEQ_API_TOKEN not set, using filesystem analysis"
    TOTAL_PAGES=$(find "$GRAPH_PATH" -name "*.md" 2>/dev/null | wc -l || echo "0")
    echo "📚 Total Files: $TOTAL_PAGES"
fi

# Analyze disk usage
if [[ -d "$GRAPH_PATH" ]]; then
    CURRENT_USAGE=$(du -sh "$GRAPH_PATH" 2>/dev/null | cut -f1)
    CURRENT_USAGE_BYTES=$(du -sb "$GRAPH_PATH" 2>/dev/null | cut -f1 || echo "0")
    echo "💾 Current Usage: $CURRENT_USAGE"
else
    echo "❌ Graph path not found: $GRAPH_PATH"
    exit 1
fi

# Convert constraint to bytes for comparison
CONSTRAINT_BYTES=$(echo "$MEMORY_CONSTRAINT" | sed 's/MB/*1000000/' | sed 's/KB/*1000/' | bc 2>/dev/null || echo "1440000")
USAGE_PERCENTAGE=$(echo "scale=1; $CURRENT_USAGE_BYTES * 100 / $CONSTRAINT_BYTES" | bc 2>/dev/null || echo "0")

echo "📊 Usage: ${USAGE_PERCENTAGE}% of constraint"

# Strategic value analysis
echo ""
echo "🎯 Strategic Value Tier Analysis:"
echo ""

# Tier 1: Core Strategic Intelligence (40% target)
TIER1_TARGET=$(echo "$CONSTRAINT_BYTES * 0.40" | bc | cut -d. -f1)
echo "📍 TIER 1 - Core Strategic Intelligence (Target: $(echo "scale=0; $TIER1_TARGET/1000" | bc)KB)"

TIER1_PAGES=(
    "MASTER_INDEX"
    "ralph_loop_strategic_memory"
    "MASTER_TECHNIQUES_GUIDE"
    "ralph_loop_19_completion"
    "ralph_loop_20_completion"
    "ralph_loop_21_completion"
)

for page in "${TIER1_PAGES[@]}"; do
    if [[ -f "$GRAPH_PATH/pages/$page.md" ]] || [[ -f "$GRAPH_PATH/$page.md" ]]; then
        echo "  ✅ $page"
    else
        echo "  🔍 $page (checking via API...)"
    fi
done

# Tier 2: Active Strategic Intelligence (35% target)
TIER2_TARGET=$(echo "$CONSTRAINT_BYTES * 0.35" | bc | cut -d. -f1)
echo ""
echo "📍 TIER 2 - Active Strategic Intelligence (Target: $(echo "scale=0; $TIER2_TARGET/1000" | bc)KB)"
echo "  📄 Recent iteration summaries (15-21)"
echo "  📄 Platform analysis documents"
echo "  📄 Methodology frameworks"

# Tier 3: Historical Intelligence (20% target)
TIER3_TARGET=$(echo "$CONSTRAINT_BYTES * 0.20" | bc | cut -d. -f1)
echo ""
echo "📍 TIER 3 - Historical Intelligence (Target: $(echo "scale=0; $TIER3_TARGET/1000" | bc)KB)"
echo "  📋 Early iteration summaries (1-14)"
echo "  🔧 Technical documentation"

# Tier 4: Reference Material (5% target)
TIER4_TARGET=$(echo "$CONSTRAINT_BYTES * 0.05" | bc | cut -d. -f1)
echo ""
echo "📍 TIER 4 - Reference Material (Target: $(echo "scale=0; $TIER4_TARGET/1000" | bc)KB)"
echo "  📖 Setup guides (removable)"
echo "  ⚙️  Configuration templates"

# Optimization recommendations
echo ""
echo "🎯 OPTIMIZATION RECOMMENDATIONS:"
echo ""

if [[ $(echo "$USAGE_PERCENTAGE > 80" | bc) -eq 1 ]]; then
    echo "🚨 URGENT: Memory usage >80% of constraint"
    echo "  Immediate Tier 4 cleanup required"
    echo "  Consider Tier 3 summarization"
elif [[ $(echo "$USAGE_PERCENTAGE > 60" | bc) -eq 1 ]]; then
    echo "⚠️  WARNING: Memory usage >60% of constraint"
    echo "  Plan Tier 2 compression"
    echo "  Schedule Tier 3 summarization"
else
    echo "✅ OPTIMAL: Memory usage <60% of constraint"
    echo "  Continue monitoring"
    echo "  Apply frameworks for future efficiency"
fi

# Brier scoring recommendations
echo ""
echo "📈 BRIER SCORING RECOMMENDATIONS:"
echo ""
echo "Create predictions for optimization effectiveness:"
echo "1. Memory reduction targets with confidence levels"
echo "2. Strategic decision quality maintenance predictions"
echo "3. Operational continuity validation predictions"
echo ""
echo "Track accuracy over next 3-5 optimization cycles"

# Output format specific results
if [[ "$OUTPUT_FORMAT" == "json" ]]; then
    cat << EOF

{
  "current_usage_bytes": $CURRENT_USAGE_BYTES,
  "constraint_bytes": $CONSTRAINT_BYTES,
  "usage_percentage": $USAGE_PERCENTAGE,
  "total_pages": $TOTAL_PAGES,
  "tier_targets": {
    "tier1_bytes": $TIER1_TARGET,
    "tier2_bytes": $TIER2_TARGET,
    "tier3_bytes": $TIER3_TARGET,
    "tier4_bytes": $TIER4_TARGET
  },
  "optimization_status": "$(if [[ $(echo "$USAGE_PERCENTAGE > 80" | bc) -eq 1 ]]; then echo "urgent"; elif [[ $(echo "$USAGE_PERCENTAGE > 60" | bc) -eq 1 ]]; then echo "warning"; else echo "optimal"; fi)"
}
EOF
elif [[ "$OUTPUT_FORMAT" == "summary" ]]; then
    echo ""
    echo "📋 SUMMARY:"
    echo "Usage: ${USAGE_PERCENTAGE}% ($CURRENT_USAGE / $MEMORY_CONSTRAINT)"
    echo "Pages: $TOTAL_PAGES"
    echo "Status: $(if [[ $(echo "$USAGE_PERCENTAGE > 80" | bc) -eq 1 ]]; then echo "Urgent optimization needed"; elif [[ $(echo "$USAGE_PERCENTAGE > 60" | bc) -eq 1 ]]; then echo "Optimization recommended"; else echo "Optimal usage"; fi)"
fi

echo ""
echo "✅ Strategic memory analysis complete"
echo "📊 Ready for OODA loop optimization planning"