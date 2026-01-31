#!/bin/bash

# Logseq Knowledge Graph Initialization Script
# Sets up a complete knowledge graph from existing documents

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

show_help() {
    cat << EOF
Logseq Knowledge Graph Initialization

USAGE:
    init-graph.sh --name "Graph Name" --path "/path/to/documents" [OPTIONS]

REQUIRED:
    --name NAME         Name for the knowledge graph
    --path PATH         Path to directory containing documents

OPTIONS:
    --template TYPE     Template type (strategic, research, project, general)
    --auto-analyze      Automatically analyze relationships (default: true)
    --create-index      Create master index page (default: true)
    --backup            Create backup of original files
    --dry-run           Show what would be done without making changes
    --help              Show this help message

DESCRIPTION:
    Initializes a Logseq knowledge graph from existing markdown documents.
    Creates configuration, analyzes relationships, and sets up navigation structure.

EXAMPLES:
    init-graph.sh --name "Strategic Memory" --path "/media/dontpanic/1112-15D8"
    init-graph.sh --name "Research" --path "/docs" --template research
    init-graph.sh --name "Project" --path "/project" --backup --dry-run

TEMPLATES:
    strategic    - For strategic analysis, iterations, and memory management
    research     - For research projects and literature analysis
    project      - For project management and tracking
    general      - General-purpose knowledge management (default)
EOF
}

log_info() {
    echo "ℹ️  $1"
}

log_success() {
    echo "✅ $1"
}

log_warning() {
    echo "⚠️  $1"
}

log_error() {
    echo "❌ $1" >&2
}

validate_inputs() {
    local name="$1"
    local path="$2"

    if [[ -z "$name" ]]; then
        log_error "Graph name is required (--name)"
        return 1
    fi

    if [[ -z "$path" ]]; then
        log_error "Document path is required (--path)"
        return 1
    fi

    if [[ ! -d "$path" ]]; then
        log_error "Document path does not exist: $path"
        return 1
    fi

    # Check for markdown files
    local md_count=$(find "$path" -name "*.md" -type f | wc -l)
    if [[ "$md_count" -eq 0 ]]; then
        log_warning "No markdown files found in $path"
        return 1
    fi

    log_info "Found $md_count markdown files in $path"
    return 0
}

create_logseq_config() {
    local path="$1"
    local template="$2"
    local graph_name="$3"

    local logseq_dir="$path/logseq"
    mkdir -p "$logseq_dir/pages"
    mkdir -p "$logseq_dir/journals"

    log_info "Creating Logseq configuration for $template template..."

    # Base configuration
    local config_file="$logseq_dir/config.edn"
    cat > "$config_file" << EOF
{:meta/version 1

 ;; Graph identification
 :graph/name "$graph_name"
 :graph/type "$template"
 :created-by "Claude Logseq Skill"
 :created-at "$(date -Iseconds)"

 ;; File preferences
 :preferred-format :markdown
 :preferred-workflow :now
 :hidden-files #{".DS_Store" "node_modules" "*.tmp"}

 ;; Graph features
 :feature/enable-search-remove-accents? true
 :feature/enable-linked-references? true
 :feature/enable-block-timestamps? false
 :feature/enable-whiteboards? true
 :feature/enable-flashcards? false

 ;; Graph view settings
 :graph/settings
 {:orphan-pages? true
  :builtin-pages? false
  :excluded-pages #{}
  :journal? false
  :enable-tooltip? true
  :show-brackets? false}

 ;; Editor preferences
 :editor/show-page-references? true
 :editor/show-brackets? false
 :editor/command-trigger "/"

 ;; Default pages
 :default-home {:page "Graph Home"}

 ;; Block references
 :ref/default-open-blocks-level 2
 :ref/linked-references-collapsed-threshold 50

EOF

    # Template-specific configurations
    case "$template" in
        "strategic")
            cat >> "$config_file" << 'EOF'
 ;; Strategic analysis settings
 :strategic-memory
 {:auto-update-references? true
  :ralph-loop-tracking? true
  :cross-reference-validation? true
  :iteration-timeline? true}

 ;; Strategic-specific graph settings
 :graph/strategic
 {:highlight-iterations true
  :show-timeline true
  :track-karma true
  :community-analysis true}}
EOF
            ;;
        "research")
            cat >> "$config_file" << 'EOF'
 ;; Research project settings
 :research
 {:citation-tracking? true
  :literature-mapping? true
  :hypothesis-linking? true
  :methodology-tracking? true}

 ;; Research-specific graph settings
 :graph/research
 {:highlight-citations true
  :show-methodology true
  :track-hypotheses true}}
EOF
            ;;
        "project")
            cat >> "$config_file" << 'EOF'
 ;; Project management settings
 :project
 {:task-tracking? true
  :milestone-linking? true
  :resource-mapping? true
  :timeline-visualization? true}

 ;; Project-specific graph settings
 :graph/project
 {:highlight-milestones true
  :show-timeline true
  :track-progress true}}
EOF
            ;;
    esac

    log_success "Created Logseq configuration: $config_file"
}

create_graph_home() {
    local path="$1"
    local template="$2"
    local graph_name="$3"

    local home_file="$path/logseq/pages/Graph Home.md"

    log_info "Creating graph home page..."

    cat > "$home_file" << EOF
# $graph_name Knowledge Graph

Welcome to the **$graph_name** knowledge graph, powered by Logseq and Claude integration.

## Overview

- **Graph Type**: $template
- **Created**: $(date '+%Y-%m-%d')
- **Documents**: Analyzing markdown files in this directory
- **Integration**: Claude Logseq Skill

## Quick Navigation

### Core Documents
EOF

    # Find and list key documents
    find "$path" -name "*.md" -not -path "*/logseq/*" | head -10 | while read -r file; do
        local basename=$(basename "$file" .md)
        echo "- [[$basename]]" >> "$home_file"
    done

    # Template-specific navigation
    case "$template" in
        "strategic")
            cat >> "$home_file" << 'EOF'

### Strategic Analysis
- [[Master Index]] - Central strategic overview
- [[Relationship Map]] - Complete file network
- [[Strategic Memory]] - Key insights and patterns
- [[Iteration Timeline]] - Historical progression

### Quick Actions
- Use graph view to explore relationships
- Search for specific concepts using semantic search
- Follow [[cross-references]] to related documents
- Check [[Recent Changes]] for latest updates
EOF
            ;;
        "research")
            cat >> "$home_file" << 'EOF'

### Research Navigation
- [[Literature Review]] - Source analysis and citations
- [[Methodology]] - Research methods and approaches
- [[Findings]] - Key discoveries and insights
- [[Bibliography]] - Complete reference list

### Research Tools
- Use graph view to trace citation networks
- Search for methodology patterns
- Follow hypothesis development chains
- Track experimental results
EOF
            ;;
        "project")
            cat >> "$home_file" << 'EOF'

### Project Management
- [[Project Overview]] - Goals and objectives
- [[Milestones]] - Key deliverables and deadlines
- [[Resources]] - Tools, people, and materials
- [[Progress]] - Current status and next steps

### Project Tools
- Use graph view to see task dependencies
- Search for resource allocation patterns
- Follow milestone achievement chains
- Track project evolution over time
EOF
            ;;
        *)
            cat >> "$home_file" << 'EOF'

### General Navigation
- Use the graph view to explore document relationships
- Search for concepts using the search functionality
- Follow links to discover related content
- Check recent pages for latest activity
EOF
            ;;
    esac

    cat >> "$home_file" << EOF

## Graph Features

$(find "$path" -name "*.md" -not -path "*/logseq/*" | wc -l) documents indexed and cross-referenced for intelligent navigation.

## Getting Started

1. **Explore**: Use the graph view (button in top right) to visualize relationships
2. **Search**: Use Ctrl/Cmd+K to search across all documents
3. **Navigate**: Click on [[wiki-style links]] to move between documents
4. **Analyze**: Use the relationship maps to understand document connections

---

*Knowledge graph initialized with Claude Logseq Skill*
EOF

    log_success "Created graph home page: $home_file"
}

analyze_relationships() {
    local path="$1"
    local dry_run="$2"

    log_info "Analyzing document relationships..."

    if [[ "$dry_run" == "true" ]]; then
        log_info "[DRY RUN] Would analyze relationships for files in $path"
        return 0
    fi

    # Use the relationship analysis script from strategic memory setup
    if [[ -f "$path/reindex_relationships.py" ]]; then
        log_info "Using existing relationship analysis script..."
        cd "$path" && python3 reindex_relationships.py
    else
        # Create a basic relationship analyzer
        cat > "$path/analyze_relationships.py" << 'EOF'
#!/usr/bin/env python3
import os
import re
from pathlib import Path
from collections import defaultdict

def analyze_files(base_dir):
    base_path = Path(base_dir)
    md_files = list(base_path.glob("*.md"))

    relationships = defaultdict(set)

    print(f"📄 Analyzing {len(md_files)} markdown files...")

    for file_path in md_files:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        file_name = file_path.stem

        # Find markdown links
        md_links = re.findall(r'\[([^\]]+)\]\(([^)]+\.md)\)', content)
        for _, linked_file in md_links:
            linked_name = Path(linked_file).stem
            relationships[file_name].add(linked_name)

        # Find wiki-style links
        wiki_links = re.findall(r'\[\[([^\]]+)\]\]', content)
        for linked_name in wiki_links:
            relationships[file_name].add(linked_name)

    print(f"🔗 Found {sum(len(rels) for rels in relationships.values())} relationships")

    # Create basic relationship map
    with open(base_path / "logseq/pages/Relationship_Map.md", 'w') as f:
        f.write("# Document Relationship Map\n\n")
        f.write(f"Generated from {len(md_files)} documents\n\n")

        for file_name in sorted(relationships.keys()):
            related = relationships[file_name]
            if related:
                f.write(f"## [[{file_name}]]\n")
                f.write("**Connected to:**\n")
                for rel in sorted(related):
                    f.write(f"- [[{rel}]]\n")
                f.write("\n")

if __name__ == "__main__":
    analyze_files(".")
EOF

        cd "$path" && python3 analyze_relationships.py
        log_success "Created basic relationship analysis"
    fi
}

create_backup() {
    local path="$1"
    local dry_run="$2"

    if [[ "$dry_run" == "true" ]]; then
        log_info "[DRY RUN] Would create backup of $path"
        return 0
    fi

    local backup_dir="${path}_backup_$(date +%Y%m%d_%H%M%S)"
    log_info "Creating backup at $backup_dir..."

    # Copy only markdown files for backup
    mkdir -p "$backup_dir"
    find "$path" -name "*.md" -not -path "*/logseq/*" -exec cp {} "$backup_dir/" \;

    log_success "Backup created: $backup_dir"
}

main() {
    local name=""
    local path=""
    local template="general"
    local auto_analyze="true"
    local create_index="true"
    local backup="false"
    local dry_run="false"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help|-h)
                show_help
                exit 0
                ;;
            --name)
                name="$2"
                shift 2
                ;;
            --path)
                path="$2"
                shift 2
                ;;
            --template)
                template="$2"
                shift 2
                ;;
            --no-auto-analyze)
                auto_analyze="false"
                shift
                ;;
            --no-create-index)
                create_index="false"
                shift
                ;;
            --backup)
                backup="true"
                shift
                ;;
            --dry-run)
                dry_run="true"
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_help >&2
                exit 1
                ;;
        esac
    done

    echo "🧠 Initializing Logseq Knowledge Graph"
    echo "   Name: $name"
    echo "   Path: $path"
    echo "   Template: $template"
    echo ""

    # Validate inputs
    validate_inputs "$name" "$path" || exit 1

    # Create backup if requested
    if [[ "$backup" == "true" ]]; then
        create_backup "$path" "$dry_run"
    fi

    # Create Logseq configuration
    if [[ "$dry_run" != "true" ]]; then
        create_logseq_config "$path" "$template" "$name"
    else
        log_info "[DRY RUN] Would create Logseq configuration"
    fi

    # Create graph home page
    if [[ "$create_index" == "true" ]]; then
        if [[ "$dry_run" != "true" ]]; then
            create_graph_home "$path" "$template" "$name"
        else
            log_info "[DRY RUN] Would create graph home page"
        fi
    fi

    # Analyze relationships
    if [[ "$auto_analyze" == "true" ]]; then
        analyze_relationships "$path" "$dry_run"
    fi

    echo ""
    log_success "Knowledge graph initialization complete!"
    echo ""
    echo "Next steps:"
    echo "  • Open Logseq and select '$path' as your graph directory"
    echo "  • Start with the 'Graph Home' page for navigation"
    echo "  • Use graph view to explore document relationships"
    echo "  • Run relationship analysis periodically to update connections"

    if [[ "$template" == "strategic" ]]; then
        echo ""
        echo "Strategic template features:"
        echo "  • Ralph Loop iteration tracking"
        echo "  • Strategic memory cross-referencing"
        echo "  • Community analysis integration"
        echo "  • Success metrics visualization"
    fi
}

# Only run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi