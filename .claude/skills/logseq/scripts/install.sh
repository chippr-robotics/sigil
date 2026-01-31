#!/bin/bash

# Logseq Installation Script
# Installs Logseq desktop application with proper configuration

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="$HOME/.local/share/logseq"
BIN_DIR="$HOME/.local/bin"

show_help() {
    cat << EOF
Logseq Installation Script

USAGE:
    install.sh [OPTIONS]

OPTIONS:
    --user              Install for current user only (default)
    --check             Check if Logseq is already installed
    --reinstall         Force reinstall even if already present
    --no-sandbox        Configure for headless/no-sandbox operation
    --help              Show this help message

DESCRIPTION:
    Installs Logseq desktop application optimally configured for Claude integration.
    Sets up proper permissions and configurations for knowledge graph management.

EXAMPLES:
    install.sh                    # Standard installation
    install.sh --check           # Check installation status
    install.sh --no-sandbox      # Install with sandbox disabled for headless operation
EOF
}

check_installation() {
    if command -v logseq &> /dev/null; then
        echo "✅ Logseq is installed"
        logseq --version 2>/dev/null || echo "   Version: Desktop application"
        echo "   Location: $(which logseq)"
        return 0
    else
        echo "❌ Logseq is not installed"
        return 1
    fi
}

install_logseq() {
    local force_install="$1"
    local no_sandbox="$2"

    echo "🔄 Installing Logseq..."

    # Check if already installed and not forcing reinstall
    if [[ "$force_install" != "true" ]] && command -v logseq &> /dev/null; then
        echo "✅ Logseq already installed. Use --reinstall to force reinstall."
        return 0
    fi

    # Install using the official installation script
    echo "📥 Downloading and installing Logseq..."
    curl -fsSL https://raw.githubusercontent.com/logseq/logseq/master/scripts/install-linux.sh | bash -s -- --user

    # Verify installation
    if ! command -v logseq &> /dev/null; then
        echo "❌ Installation failed - logseq command not found"
        return 1
    fi

    echo "✅ Logseq installation complete"

    # Configure for no-sandbox if requested
    if [[ "$no_sandbox" == "true" ]]; then
        echo "🔧 Configuring for no-sandbox operation..."

        # Create wrapper script for no-sandbox operation
        cat > "$BIN_DIR/logseq-no-sandbox" << 'EOF'
#!/bin/bash
exec logseq --no-sandbox "$@"
EOF
        chmod +x "$BIN_DIR/logseq-no-sandbox"

        echo "✅ No-sandbox wrapper created at $BIN_DIR/logseq-no-sandbox"
    fi
}

setup_integration() {
    echo "🔧 Setting up Claude integration configuration..."

    # Create default configuration directories
    mkdir -p "$HOME/.logseq/config"
    mkdir -p "$HOME/.logseq/templates"

    # Create basic integration config
    cat > "$HOME/.logseq/config/claude-integration.edn" << 'EOF'
;; Claude-Logseq Integration Configuration
{:integration
 {:claude-skills true
  :graph-analysis true
  :relationship-tracking true
  :concept-indexing true
  :strategic-memory true}

 :defaults
 {:graph-type :knowledge-management
  :relationship-threshold 0.7
  :auto-index true
  :cross-reference-depth 3}

 :workflows
 {:strategic-memory
  {:auto-update true
   :timeline-tracking true
   :iteration-analysis true}

  :research-projects
  {:citation-tracking true
   :literature-mapping true
   :hypothesis-linking true}}}
EOF

    echo "✅ Integration configuration created"
}

main() {
    local check_only="false"
    local force_install="false"
    local no_sandbox="false"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help|-h)
                show_help
                exit 0
                ;;
            --check)
                check_only="true"
                shift
                ;;
            --reinstall)
                force_install="true"
                shift
                ;;
            --no-sandbox)
                no_sandbox="true"
                shift
                ;;
            --user)
                # Already default, just consume the argument
                shift
                ;;
            *)
                echo "Unknown option: $1" >&2
                show_help >&2
                exit 1
                ;;
        esac
    done

    echo "🚀 Logseq Installation for Claude Integration"
    echo ""

    # Check installation if requested
    if [[ "$check_only" == "true" ]]; then
        check_installation
        exit $?
    fi

    # Install Logseq
    install_logseq "$force_install" "$no_sandbox"

    # Set up integration features
    setup_integration

    echo ""
    echo "🎉 Logseq installation and Claude integration setup complete!"
    echo ""
    echo "Next steps:"
    echo "  • Use 'logseq' command to open the application"
    if [[ "$no_sandbox" == "true" ]]; then
        echo "  • Use 'logseq-no-sandbox' for headless operation"
    fi
    echo "  • Run 'init-graph.sh' to set up your first knowledge graph"
    echo "  • See ~/.claude/skills/logseq/examples/ for usage examples"
}

# Only run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi