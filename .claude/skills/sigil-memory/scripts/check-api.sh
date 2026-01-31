#!/bin/bash

# Check Logseq HTTP API Status
# Verifies API connectivity and authentication

set -e

API_HOST="${LOGSEQ_API_HOST:-localhost}"
API_PORT="${LOGSEQ_API_PORT:-3001}"
API_BASE="http://${API_HOST}:${API_PORT}"

show_help() {
    cat << 'EOF'
Check Logseq HTTP API Status

USAGE:
    check-api.sh [OPTIONS]

OPTIONS:
    --host HOST     API host (default: localhost)
    --port PORT     API port (default: 3001)
    --token TOKEN   API token (default: $LOGSEQ_API_TOKEN)
    --help          Show this help

DESCRIPTION:
    Verifies Logseq HTTP API server is running and accessible.
    Checks authentication and basic connectivity.

SETUP:
    1. Open Logseq desktop app
    2. Go to Settings → Features
    3. Enable "HTTP APIs server"
    4. Set LOGSEQ_API_TOKEN environment variable

EXAMPLES:
    check-api.sh
    check-api.sh --host remote-server --port 3001
    LOGSEQ_API_TOKEN=xxx check-api.sh
EOF
}

check_api_server() {
    local host="$1"
    local port="$2"
    local base_url="http://${host}:${port}"

    echo "🔍 Checking Logseq API server at ${base_url}..."

    # Test basic connectivity
    if ! curl -s --connect-timeout 5 "${base_url}/api/ping" > /dev/null 2>&1; then
        echo "❌ API server not reachable at ${base_url}"
        echo ""
        echo "Setup instructions:"
        echo "1. Open Logseq desktop app"
        echo "2. Settings → Features → Enable 'HTTP APIs server'"
        echo "3. Restart Logseq if needed"
        return 1
    fi

    echo "✅ API server is reachable"
    return 0
}

check_authentication() {
    local base_url="$1"
    local token="$2"

    echo "🔐 Checking API authentication..."

    if [[ -z "$token" ]]; then
        echo "⚠️  No API token provided"
        echo "Set LOGSEQ_API_TOKEN environment variable"
        echo "Token can be found in Logseq Settings → Features → API token"
        return 1
    fi

    # Test authenticated endpoint
    local response
    response=$(curl -s -w "%{http_code}" \
        -H "Authorization: Bearer ${token}" \
        "${base_url}/api/graphs" \
        -o /dev/null)

    case "$response" in
        200)
            echo "✅ Authentication successful"
            ;;
        401|403)
            echo "❌ Authentication failed (HTTP $response)"
            echo "Check your LOGSEQ_API_TOKEN"
            return 1
            ;;
        *)
            echo "❌ Unexpected response: HTTP $response"
            return 1
            ;;
    esac

    return 0
}

get_graph_info() {
    local base_url="$1"
    local token="$2"

    echo "📊 Retrieving graph information..."

    local response
    response=$(curl -s \
        -H "Authorization: Bearer ${token}" \
        "${base_url}/api/graphs")

    if [[ $? -eq 0 && -n "$response" ]]; then
        echo "📚 Available graphs:"
        echo "$response" | jq -r '.[] | "  • \(.name) (\(.path))"' 2>/dev/null || echo "$response"
    else
        echo "⚠️  Could not retrieve graph information"
    fi
}

test_basic_operations() {
    local base_url="$1"
    local token="$2"

    echo "🧪 Testing basic API operations..."

    # Test page creation capability
    local test_payload='{
        "page": "API Test Page",
        "blocks": [
            {"content": "# API Test"},
            {"content": "This page was created via API to test connectivity."},
            {"content": "Created at: '"$(date)"'"}
        ]
    }'

    echo "  • Testing page creation..."
    local create_response
    create_response=$(curl -s -w "%{http_code}" \
        -H "Authorization: Bearer ${token}" \
        -H "Content-Type: application/json" \
        -X POST "${base_url}/api/pages" \
        -d "$test_payload" \
        -o /dev/null)

    case "$create_response" in
        200|201)
            echo "  ✅ Page creation works"
            ;;
        *)
            echo "  ⚠️  Page creation test failed (HTTP $create_response)"
            ;;
    esac

    # Test query capability
    echo "  • Testing graph queries..."
    local query_response
    query_response=$(curl -s -w "%{http_code}" \
        -H "Authorization: Bearer ${token}" \
        "${base_url}/api/query?q=pages" \
        -o /dev/null)

    case "$query_response" in
        200)
            echo "  ✅ Graph queries work"
            ;;
        *)
            echo "  ⚠️  Query test failed (HTTP $query_response)"
            ;;
    esac
}

main() {
    local host="$API_HOST"
    local port="$API_PORT"
    local token="$LOGSEQ_API_TOKEN"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help|-h)
                show_help
                exit 0
                ;;
            --host)
                host="$2"
                shift 2
                ;;
            --port)
                port="$2"
                shift 2
                ;;
            --token)
                token="$2"
                shift 2
                ;;
            *)
                echo "Unknown option: $1" >&2
                show_help >&2
                exit 1
                ;;
        esac
    done

    local base_url="http://${host}:${port}"

    echo "🔍 Logseq API Status Check"
    echo "Server: ${base_url}"
    echo ""

    # Run checks
    if check_api_server "$host" "$port"; then
        if check_authentication "$base_url" "$token"; then
            get_graph_info "$base_url" "$token"
            test_basic_operations "$base_url" "$token"

            echo ""
            echo "🎉 Logseq API integration is ready!"
            echo ""
            echo "Available operations:"
            echo "  • Create and manage pages"
            echo "  • Query graph data"
            echo "  • Manage blocks and relationships"
            echo "  • Export and sync operations"

        else
            echo ""
            echo "🔧 Setup needed:"
            echo "  1. Get API token from Logseq Settings → Features"
            echo "  2. Set LOGSEQ_API_TOKEN environment variable"
            echo "  3. Restart this check"
        fi
    fi
}

# Only run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi