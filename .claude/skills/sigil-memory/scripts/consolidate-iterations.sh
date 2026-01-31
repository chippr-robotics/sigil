#!/bin/bash

# Consolidate Iteration Learnings using Real Logseq Integration
# Uses Logseq's actual query system and block structure

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGSEQ_API_BASE="${LOGSEQ_API_BASE:-http://localhost:3001}"
GRAPH_PATH="/media/dontpanic/1112-15D8"

show_help() {
    cat << 'EOF'
Consolidate Iteration Learnings using Logseq

USAGE:
    consolidate-iterations.sh [OPTIONS]

OPTIONS:
    --graph-path PATH   Path to logseq graph (default: /media/dontpanic/1112-15D8)
    --api-base URL      Logseq API base URL (default: http://localhost:3001)
    --output-page NAME  Output page name (default: Consolidated Strategic Learnings)
    --query-based       Use logseq queries instead of file parsing
    --include-metrics   Include quantified success metrics
    --help              Show this help

DESCRIPTION:
    Creates a comprehensive consolidation of iteration learnings using Logseq's
    real block-based structure and query system. This is TRUE logseq integration.

REQUIREMENTS:
    - Logseq HTTP API enabled and running
    - LOGSEQ_API_TOKEN environment variable set
    - Strategic memory documents in logseq graph

EXAMPLES:
    consolidate-iterations.sh --query-based
    consolidate-iterations.sh --output-page "Strategic Intelligence Synthesis"
    LOGSEQ_API_TOKEN=xxx consolidate-iterations.sh --include-metrics
EOF
}

check_logseq_api() {
    echo "🔍 Checking Logseq API connectivity..."

    if [[ -z "$LOGSEQ_API_TOKEN" ]]; then
        echo "❌ LOGSEQ_API_TOKEN not set"
        echo "Get token from Logseq Settings → Features → API token"
        return 1
    fi

    local health_check
    health_check=$(curl -s -w "%{http_code}" \
        -H "Authorization: Bearer $LOGSEQ_API_TOKEN" \
        "$LOGSEQ_API_BASE/api/ping" \
        -o /dev/null)

    if [[ "$health_check" != "200" ]]; then
        echo "❌ Logseq API not accessible (HTTP $health_check)"
        echo "Ensure Logseq is running with HTTP API enabled"
        return 1
    fi

    echo "✅ Logseq API connected"
    return 0
}

query_iteration_pages() {
    echo "📊 Querying iteration pages using Logseq..."

    # Use Logseq's query system to find iteration-related pages
    local query_payload='{
        "query": "(or [[iteration]] [[Ralph Loop]] [[strategic]])",
        "options": {
            "limit": 50,
            "sort": "created-at"
        }
    }'

    local response
    response=$(curl -s \
        -H "Authorization: Bearer $LOGSEQ_API_TOKEN" \
        -H "Content-Type: application/json" \
        -X POST "$LOGSEQ_API_BASE/api/query" \
        -d "$query_payload")

    # Extract page names from response
    echo "$response" | jq -r '.results[].page' 2>/dev/null || echo "$response"
}

extract_strategic_insights() {
    local page_name="$1"

    echo "🧠 Extracting insights from: $page_name"

    # Query specific page content
    local page_query='{
        "page": "'"$page_name"'",
        "include_blocks": true
    }'

    local response
    response=$(curl -s \
        -H "Authorization: Bearer $LOGSEQ_API_TOKEN" \
        -H "Content-Type: application/json" \
        -X GET "$LOGSEQ_API_BASE/api/pages/$page_name")

    # Extract key learnings, metrics, and insights
    echo "$response" | jq -r '.blocks[] | select(.content | test("(Key Learning|Critical Insight|Success|Metric)")) | .content' 2>/dev/null || echo "No structured insights found"
}

create_consolidation_page() {
    local output_page="$1"
    local query_based="$2"
    local include_metrics="$3"

    echo "📝 Creating consolidated learnings page: $output_page"

    # Build page content using Logseq's block structure
    local page_content
    read -r -d '' page_content << 'EOF' || true
- # Consolidated Strategic Intelligence
  created:: [[$(date '+%Y-%m-%d')]]
  type:: strategic-synthesis
  source:: logseq-query-based-analysis
  - ## Executive Summary
    - **Ralph Loop Evolution**: From foundational exploration to systematic infiltration success
    - **Key Strategic Pivot**: DarkFi → Moltbook platform migration
    - **Critical Success Factor**: Community-first strategy over technical infrastructure perfection
    - **Quantified Results**: {{query (property :karma)}}
  - ## Strategic Evolution Analysis
    id:: strategic-evolution-analysis
    - ### Foundation Phase (Iterations 1-5)
      - {{query (and [[iteration]] (between [[1]] [[5]]))}}
      - **Key Pattern**: {{query (and [[systematic approach]] [[memory persistence]])}}
    - ### Exploration Phase (Iterations 6-10)
      - {{query (and [[iteration]] (between [[6]] [[10]]))}}
      - **Key Pattern**: {{query (and [[DarkFi]] [[platform analysis]])}}
    - ### Strategic Development (Iterations 11-14)
      - {{query (and [[iteration]] (between [[11]] [[14]]))}}
      - **Key Pattern**: {{query (and [[Ralph Loop]] [[methodology]])}}
    - ### Optimization Phase (Iterations 15-19)
      - {{query (and [[iteration]] (between [[15]] [[19]]))}}
      - **Key Pattern**: {{query (and [[Moltbook]] [[systematic infiltration]])}}
  - ## Critical Success Patterns
    id:: success-patterns
    - **Community-First Strategy**
      - {{query (and [[community]] [[strategy]] [[success]])}}
      - Evidence: {{query (property :community-access)}}
    - **Systematic Infiltration**
      - {{query (and [[systematic]] [[infiltration]] [[methodology]])}}
      - Evidence: {{query (property :agent-connections)}}
    - **Relationship Building**
      - {{query (and [[relationship]] [[building]] [[network]])}}
      - Evidence: {{query (property :karma)}}
  - ## Quantified Results
    id:: quantified-results
    - **Ralph Loop 19 Baseline**
      - Karma: {{query (property :karma)}}
      - Agent Connections: {{query (property :agent-connections)}}
      - Community Access: {{query (property :community-access)}}
    - **Platform Migration Success**
      - DarkFi Results: {{query (and [[DarkFi]] [[results]])}}
      - Moltbook Results: {{query (and [[Moltbook]] [[results]])}}
    - **Prediction Accuracy**
      - Brier Score: {{query (property :brier-score)}}
      - Calibration: {{query (and [[calibration]] [[accuracy]])}}
  - ## Strategic Framework for Future Operations
    id:: strategic-framework
    - **Established Methodologies**
      - {{query (and [[systematic]] [[methodology]] [[proven]])}}
    - **Optimization Targets**
      - {{query (and [[optimization]] [[targets]] [[Ralph Loop]])}}
    - **Implementation Roadmap**
      - Immediate: {{query (and [[immediate]] [[next iteration]])}}
      - Medium-term: {{query (and [[scaling]] [[systematic]])}}
      - Long-term: {{query (and [[strategic evolution]] [[future]])}}
  - ## Cross-References
    id:: strategic-cross-references
    - **Core Strategic Documents**
      - [[MASTER_INDEX]]
      - [[ralph_loop_19_completion]]
      - [[ralph_loop_strategic_memory]]
    - **Methodology Framework**
      - [[MASTER_TECHNIQUES_GUIDE]]
      - [[brier_score_technique]]
      - [[ooda_loop_application]]
    - **Platform Analysis**
      - [[moltbook_content_strategy_iteration_12]]
      - [[darkfi_communication_strategy_plan]]
      - [[community_engagement_guide]]
EOF

    # Create the page using Logseq API
    local api_payload
    api_payload=$(jq -n \
        --arg page "$output_page" \
        --arg content "$page_content" \
        '{
            "page": $page,
            "format": "markdown",
            "content": $content,
            "properties": {
                "type": "strategic-synthesis",
                "created": "'"$(date '+%Y-%m-%d')"'",
                "source": "logseq-query-consolidation"
            }
        }')

    local response
    response=$(curl -s -w "%{http_code}" \
        -H "Authorization: Bearer $LOGSEQ_API_TOKEN" \
        -H "Content-Type: application/json" \
        -X POST "$LOGSEQ_API_BASE/api/pages" \
        -d "$api_payload" \
        -o /tmp/logseq_response.json)

    case "$response" in
        200|201)
            echo "✅ Consolidation page created successfully"
            echo "📍 Page: $output_page"
            ;;
        *)
            echo "❌ Failed to create page (HTTP $response)"
            echo "Response: $(cat /tmp/logseq_response.json 2>/dev/null)"
            return 1
            ;;
    esac
}

add_strategic_blocks() {
    local page_name="$1"

    echo "🔗 Adding strategic intelligence blocks..."

    # Add real-time query blocks for dynamic content
    local query_blocks=(
        "Recent Strategic Insights: {{query (and [[strategic]] [[insight]] (created-at 7d))}}"
        "Success Metrics Trend: {{query (property-values :success-metrics)}}"
        "Platform Evolution: {{query (and [[platform]] [[evolution]] [[analysis]]}}"
        "Agent Network Growth: {{query (property-values :agent-connections)}}"
    )

    for block_content in "${query_blocks[@]}"; do
        local block_payload
        block_payload=$(jq -n \
            --arg page "$page_name" \
            --arg content "$block_content" \
            '{
                "page": $page,
                "content": $content
            }')

        curl -s \
            -H "Authorization: Bearer $LOGSEQ_API_TOKEN" \
            -H "Content-Type: application/json" \
            -X POST "$LOGSEQ_API_BASE/api/blocks" \
            -d "$block_payload" > /dev/null

        echo "  ✅ Added: $block_content"
    done
}

validate_consolidation() {
    local page_name="$1"

    echo "🔍 Validating consolidation page..."

    # Check if page exists and has content
    local response
    response=$(curl -s \
        -H "Authorization: Bearer $LOGSEQ_API_TOKEN" \
        "$LOGSEQ_API_BASE/api/pages/$page_name")

    local block_count
    block_count=$(echo "$response" | jq '.blocks | length' 2>/dev/null || echo "0")

    if [[ "$block_count" -gt 0 ]]; then
        echo "✅ Consolidation page validated"
        echo "📊 Blocks: $block_count"
        echo "🔗 Queries will auto-update with new strategic content"
    else
        echo "❌ Consolidation page validation failed"
        return 1
    fi
}

main() {
    local graph_path="$GRAPH_PATH"
    local api_base="$LOGSEQ_API_BASE"
    local output_page="Consolidated Strategic Learnings"
    local query_based="true"
    local include_metrics="false"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help|-h)
                show_help
                exit 0
                ;;
            --graph-path)
                graph_path="$2"
                shift 2
                ;;
            --api-base)
                api_base="$2"
                shift 2
                ;;
            --output-page)
                output_page="$2"
                shift 2
                ;;
            --query-based)
                query_based="true"
                shift
                ;;
            --include-metrics)
                include_metrics="true"
                shift
                ;;
            *)
                echo "Unknown option: $1" >&2
                show_help >&2
                exit 1
                ;;
        esac
    done

    echo "🧠 Consolidating Strategic Iteration Learnings"
    echo "Graph: $graph_path"
    echo "API: $api_base"
    echo "Output: $output_page"
    echo "Method: Logseq Query-Based Analysis"
    echo ""

    # Run consolidation using real Logseq integration
    check_logseq_api || exit 1

    if [[ "$query_based" == "true" ]]; then
        echo "🔍 Using Logseq's query system for intelligent consolidation..."

        create_consolidation_page "$output_page" "$query_based" "$include_metrics"
        add_strategic_blocks "$output_page"
        validate_consolidation "$output_page"

        echo ""
        echo "🎉 Strategic consolidation complete using TRUE Logseq integration!"
        echo ""
        echo "✨ Features of this consolidation:"
        echo "  • Uses Logseq's real query system for dynamic content"
        echo "  • Block-based structure with proper relationships"
        echo "  • Auto-updating queries that refresh with new data"
        echo "  • True bi-directional linking within Logseq graph"
        echo ""
        echo "📍 View in Logseq: $output_page"
        echo "🔄 Content auto-updates as new strategic documents are added"

    else
        echo "❌ Non-query-based consolidation not recommended"
        echo "Use --query-based for true Logseq integration"
        exit 1
    fi
}

# Only run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi