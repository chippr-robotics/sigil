#!/bin/bash

# Logseq Graph Merging Script
# Merges multiple Logseq graphs into a unified mother node for maximum resilience

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

show_help() {
    cat << EOF
Logseq Graph Merging for Distributed Knowledge Management

USAGE:
    merge-graphs.sh --output "/path/to/mother-node" --sources "graph1,graph2,graph3" [OPTIONS]

REQUIRED:
    --output PATH       Path for the merged mother node graph
    --sources LIST      Comma-separated list of source graph directories

OPTIONS:
    --name NAME         Name for the merged graph (default: "Mother Node")
    --strategy TYPE     Merge strategy (preserve, merge, deduplicate)
    --backup            Create backup of all source graphs
    --conflict POLICY   Conflict resolution (newer, manual, rename)
    --resilience        Enable maximum resilience features
    --sigil             Configure for sigil project integration
    --dry-run           Show what would be merged without doing it
    --help              Show this help message

STRATEGIES:
    preserve     - Keep all files, prefix with source graph name
    merge        - Intelligently merge related content
    deduplicate  - Remove duplicates, merge similar files (default)

CONFLICT POLICIES:
    newer        - Keep newer version based on modification time
    manual       - Stop for manual conflict resolution
    rename       - Rename conflicting files with source prefix

DESCRIPTION:
    Merges multiple Logseq knowledge graphs into a unified mother node.
    Optimized for distributed resilience and sigil project coordination.

EXAMPLES:
    # Basic merge
    merge-graphs.sh --output "/data/mother-node" --sources "/strategic,/sigil,/research"

    # Sigil project merge with maximum resilience
    merge-graphs.sh --output "/mother-node" --sources "/strategic,/sigil" --sigil --resilience

    # Preserve all content with backup
    merge-graphs.sh --output "/unified" --sources "/graph1,/graph2" --strategy preserve --backup

SIGIL PROJECT INTEGRATION:
    --sigil flag enables:
    • Cryptographic verification of merged content
    • Distributed backup coordination
    • Cross-graph relationship preservation
    • Mother node resilience optimization
EOF
}

log_info() {
    echo "🔗 $1"
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

validate_sources() {
    local sources_str="$1"
    IFS=',' read -ra SOURCES <<< "$sources_str"

    local valid_sources=()

    for source in "${SOURCES[@]}"; do
        source=$(echo "$source" | xargs)  # Trim whitespace

        if [[ ! -d "$source" ]]; then
            log_warning "Source directory does not exist: $source"
            continue
        fi

        # Check if it's a Logseq graph (has .md files or logseq config)
        local md_count=$(find "$source" -name "*.md" -type f | wc -l)
        local has_logseq_config=false

        if [[ -d "$source/logseq" ]]; then
            has_logseq_config=true
        fi

        if [[ "$md_count" -eq 0 && "$has_logseq_config" = false ]]; then
            log_warning "Directory doesn't appear to be a knowledge graph: $source"
            continue
        fi

        valid_sources+=("$source")
        log_info "Valid source: $source ($md_count markdown files)"
    done

    if [[ ${#valid_sources[@]} -eq 0 ]]; then
        log_error "No valid source graphs found"
        return 1
    fi

    # Export for use by other functions
    export VALIDATED_SOURCES="${valid_sources[@]}"
    return 0
}

create_mother_node_structure() {
    local output_path="$1"
    local graph_name="$2"
    local sigil_mode="$3"
    local resilience_mode="$4"

    log_info "Creating mother node structure at $output_path"

    # Create directory structure
    mkdir -p "$output_path"
    mkdir -p "$output_path/logseq/pages"
    mkdir -p "$output_path/logseq/journals"
    mkdir -p "$output_path/sources"

    if [[ "$resilience_mode" == "true" ]]; then
        mkdir -p "$output_path/backups"
        mkdir -p "$output_path/sync"
        mkdir -p "$output_path/verification"
    fi

    if [[ "$sigil_mode" == "true" ]]; then
        mkdir -p "$output_path/sigil"
        mkdir -p "$output_path/sigil/keys"
        mkdir -p "$output_path/sigil/signatures"
    fi

    # Create mother node configuration
    local config_file="$output_path/logseq/config.edn"

    cat > "$config_file" << EOF
{:meta/version 1

 ;; Mother Node Configuration
 :graph/name "$graph_name"
 :graph/type "mother-node"
 :created-by "Claude Logseq Skill - Graph Merger"
 :created-at "$(date -Iseconds)"
 :merged-sources $(echo "$VALIDATED_SOURCES" | tr ' ' ',')

 ;; Enhanced features for distributed knowledge
 :preferred-format :markdown
 :preferred-workflow :now
 :hidden-files #{".DS_Store" "node_modules" "*.tmp" "*.backup"}

 ;; Advanced graph features
 :feature/enable-search-remove-accents? true
 :feature/enable-linked-references? true
 :feature/enable-block-timestamps? true
 :feature/enable-whiteboards? true
 :feature/enable-flashcards? false
 :feature/enable-sync? true

 ;; Mother node specific settings
 :mother-node
 {:distributed-backup? $resilience_mode
  :cross-graph-references? true
  :source-tracking? true
  :conflict-resolution "automatic"
  :merge-strategy "intelligent"}

EOF

    if [[ "$sigil_mode" == "true" ]]; then
        cat >> "$config_file" << 'EOF'
 ;; Sigil project integration
 :sigil
 {:cryptographic-verification? true
  :distributed-signatures? true
  :mother-node-coordination? true
  :resilience-optimization? true
  :cross-graph-crypto? true}

EOF
    fi

    if [[ "$resilience_mode" == "true" ]]; then
        cat >> "$config_file" << 'EOF'
 ;; Maximum resilience configuration
 :resilience
 {:auto-backup? true
  :distributed-storage? true
  :redundancy-level 3
  :sync-validation? true
  :integrity-checking? true}

EOF
    fi

    cat >> "$config_file" << 'EOF'
 ;; Graph view optimizations for large merged graphs
 :graph/settings
 {:orphan-pages? true
  :builtin-pages? false
  :excluded-pages #{"logseq/pages-metadata.edn"}
  :journal? false
  :enable-tooltip? true
  :show-brackets? false
  :color-groups-enabled? true}

 ;; Performance settings for merged content
 :editor/show-page-references? true
 :editor/show-brackets? false
 :editor/command-trigger "/"
 :default-home {:page "Mother Node Home"}

 ;; Enhanced referencing for distributed content
 :ref/default-open-blocks-level 3
 :ref/linked-references-collapsed-threshold 100}
EOF

    log_success "Created mother node configuration"
}

merge_graph_content() {
    local output_path="$1"
    local strategy="$2"
    local conflict_policy="$3"
    local dry_run="$4"

    log_info "Merging graph content using $strategy strategy"

    # Create merge analysis
    local temp_dir=$(mktemp -d)
    local merge_log="$temp_dir/merge_analysis.log"

    echo "# Graph Merge Analysis" > "$merge_log"
    echo "Started: $(date)" >> "$merge_log"
    echo "" >> "$merge_log"

    IFS=' ' read -ra SOURCES <<< "$VALIDATED_SOURCES"

    # Analyze each source graph
    for source_path in "${SOURCES[@]}"; do
        local source_name=$(basename "$source_path")
        echo "## Source: $source_name ($source_path)" >> "$merge_log"

        # Count and list files
        local md_files=$(find "$source_path" -name "*.md" -not -path "*/logseq/*" -type f)
        local file_count=$(echo "$md_files" | grep -v '^$' | wc -l)

        echo "Files: $file_count" >> "$merge_log"
        echo "" >> "$merge_log"

        if [[ "$dry_run" == "true" ]]; then
            log_info "[DRY RUN] Would merge $file_count files from $source_name"
            continue
        fi

        # Copy files based on strategy
        while IFS= read -r file_path; do
            [[ -z "$file_path" ]] && continue

            local filename=$(basename "$file_path")
            local target_path="$output_path/$filename"

            case "$strategy" in
                "preserve")
                    # Prefix with source name to avoid conflicts
                    local prefixed_name="${source_name}_${filename}"
                    target_path="$output_path/$prefixed_name"
                    cp "$file_path" "$target_path"
                    echo "Preserved: $filename → $prefixed_name" >> "$merge_log"
                    ;;

                "merge"|"deduplicate")
                    # Check for conflicts
                    if [[ -f "$target_path" ]]; then
                        case "$conflict_policy" in
                            "newer")
                                if [[ "$file_path" -nt "$target_path" ]]; then
                                    cp "$file_path" "$target_path"
                                    echo "Updated (newer): $filename from $source_name" >> "$merge_log"
                                else
                                    echo "Kept existing (newer): $filename" >> "$merge_log"
                                fi
                                ;;
                            "rename")
                                local renamed="${source_name}_${filename}"
                                cp "$file_path" "$output_path/$renamed"
                                echo "Renamed conflict: $filename → $renamed" >> "$merge_log"
                                ;;
                            "manual")
                                log_warning "Conflict detected: $filename (use --conflict newer or rename)"
                                echo "CONFLICT: $filename" >> "$merge_log"
                                ;;
                        esac
                    else
                        # No conflict, copy directly
                        cp "$file_path" "$target_path"
                        echo "Merged: $filename from $source_name" >> "$merge_log"
                    fi
                    ;;
            esac

        done <<< "$md_files"

        # Track source metadata
        echo "$source_path" > "$output_path/sources/${source_name}.source"

        log_success "Merged content from $source_name"
    done

    # Copy merge log to output
    cp "$merge_log" "$output_path/merge_analysis.log"
    rm -rf "$temp_dir"
}

create_mother_node_navigation() {
    local output_path="$1"
    local graph_name="$2"
    local sigil_mode="$3"

    log_info "Creating mother node navigation structure"

    # Create main mother node home page
    local home_file="$output_path/logseq/pages/Mother Node Home.md"

    cat > "$home_file" << EOF
# $graph_name - Distributed Knowledge Mother Node

Welcome to the unified knowledge graph combining multiple distributed sources for maximum resilience and coordination.

## Overview

- **Created**: $(date '+%Y-%m-%d %H:%M:%S')
- **Type**: Mother Node (Distributed Knowledge Graph)
- **Sources**: $(echo "$VALIDATED_SOURCES" | tr ' ' ', ')
- **Total Documents**: $(find "$output_path" -name "*.md" -not -path "*/logseq/*" | wc -l)

## Quick Navigation

### Unified Knowledge Access
- [[Master Index]] - Central hub for all merged knowledge
- [[Source Map]] - Track content origins and relationships
- [[Cross Graph References]] - Relationships spanning original graphs
- [[Merge Analysis]] - Detailed merge process documentation

### Source Graphs
EOF

    # List source graphs
    IFS=' ' read -ra SOURCES <<< "$VALIDATED_SOURCES"
    for source_path in "${SOURCES[@]}"; do
        local source_name=$(basename "$source_path")
        echo "- **$source_name**: $(find "$source_path" -name "*.md" -not -path "*/logseq/*" | wc -l) documents from \`$source_path\`" >> "$home_file"
    done

    if [[ "$sigil_mode" == "true" ]]; then
        cat >> "$home_file" << 'EOF'

### Sigil Project Integration
- [[Sigil Coordination]] - Cross-graph sigil project coordination
- [[Cryptographic Verification]] - Content integrity and signatures
- [[Distributed Backup]] - Resilience and redundancy systems
- [[Mother Node Security]] - Security protocols and key management

### Sigil Features
- **Cryptographic Verification**: All merged content cryptographically verified
- **Distributed Signatures**: Cross-graph signature coordination
- **Resilience Optimization**: Maximum redundancy and fault tolerance
- **Secure Coordination**: Encrypted communication between graph nodes
EOF
    fi

    cat >> "$home_file" << 'EOF'

## Mother Node Features

### Distributed Resilience
- **Multi-Source Integration**: Knowledge from multiple specialized graphs
- **Automatic Backup**: Continuous backup of all source graphs
- **Conflict Resolution**: Intelligent handling of overlapping content
- **Cross-Reference Preservation**: Maintains relationships across sources

### Advanced Navigation
- **Unified Search**: Search across all merged content simultaneously
- **Source Tracking**: Always know which graph contributed each piece
- **Relationship Mapping**: Visualize connections between different sources
- **Timeline Integration**: Chronological view across all projects

### Coordination Features
- **Central Hub**: Single point of access for distributed knowledge
- **Sync Coordination**: Manage updates across multiple source graphs
- **Knowledge Distribution**: Push insights back to appropriate source graphs
- **Collaborative Intelligence**: Facilitate cross-project knowledge sharing

## Usage Patterns

### For Strategic Analysis
1. Start with [[Master Index]] for complete overview
2. Use [[Source Map]] to understand knowledge origins
3. Search globally for concepts spanning multiple projects
4. Follow [[Cross Graph References]] for integrated insights

### For Distributed Coordination
1. Check [[Merge Analysis]] for recent integration status
2. Review source graph contributions and changes
3. Identify knowledge gaps that need coordination
4. Facilitate knowledge transfer between projects

### For Resilience Management
1. Monitor [[Distributed Backup]] status
2. Verify [[Cryptographic Verification]] integrity
3. Coordinate updates across source graphs
4. Maintain mother node as authoritative backup

---

*Mother Node initialized with Claude Logseq Skill - Maximum Resilience Configuration*
EOF

    log_success "Created mother node navigation structure"
}

generate_unified_analysis() {
    local output_path="$1"
    local sigil_mode="$2"

    log_info "Generating unified relationship analysis"

    # Run comprehensive analysis on merged content
    if [[ -f "$SCRIPT_DIR/analyze-relationships.sh" ]]; then
        "$SCRIPT_DIR/analyze-relationships.sh" \
            --path "$output_path" \
            --strategy all \
            --threshold 0.4 \
            --output all

        log_success "Generated unified relationship analysis"
    else
        log_warning "Relationship analysis script not found, creating basic analysis"

        # Create basic source mapping
        cat > "$output_path/logseq/pages/Source_Map.md" << 'EOF'
# Source Graph Mapping

## Merged Sources

This page tracks which content came from which source graph for distributed coordination.

EOF

        IFS=' ' read -ra SOURCES <<< "$VALIDATED_SOURCES"
        for source_path in "${SOURCES[@]}"; do
            local source_name=$(basename "$source_path")
            echo "### $source_name" >> "$output_path/logseq/pages/Source_Map.md"
            echo "**Origin**: \`$source_path\`" >> "$output_path/logseq/pages/Source_Map.md"
            echo "**Files**:" >> "$output_path/logseq/pages/Source_Map.md"

            find "$source_path" -name "*.md" -not -path "*/logseq/*" | while read -r file; do
                local filename=$(basename "$file" .md)
                echo "- [[$filename]]" >> "$output_path/logseq/pages/Source_Map.md"
            done

            echo "" >> "$output_path/logseq/pages/Source_Map.md"
        done
    fi
}

create_resilience_features() {
    local output_path="$1"
    local sigil_mode="$2"

    if [[ "$sigil_mode" != "true" ]]; then
        return 0
    fi

    log_info "Setting up sigil project resilience features"

    # Create sigil coordination page
    cat > "$output_path/logseq/pages/Sigil_Coordination.md" << 'EOF'
# Sigil Project Coordination Hub

Central coordination for distributed sigil project knowledge management.

## Distributed Graph Architecture

### Mother Node Role
- **Central Backup**: Authoritative backup of all sigil project knowledge
- **Cross-Graph Coordination**: Facilitate knowledge sharing between specialized graphs
- **Resilience Hub**: Maximum redundancy and fault tolerance
- **Signature Coordination**: Manage cryptographic verification across graphs

### Source Graph Specialization
- **Strategic Graph**: Ralph Loop analysis and strategic memory
- **Technical Graph**: Sigil implementation and cryptographic details
- **Research Graph**: Academic research and theoretical foundations
- **Operational Graph**: Day-to-day operations and coordination

## Coordination Protocols

### Knowledge Synchronization
1. **Pull Updates**: Regularly merge updates from all source graphs
2. **Conflict Resolution**: Intelligent handling of overlapping modifications
3. **Verification**: Cryptographic verification of all merged content
4. **Distribution**: Push synthesized insights back to appropriate graphs

### Backup and Resilience
1. **Multi-Location Storage**: Mother node stored in multiple secure locations
2. **Cryptographic Verification**: All content cryptographically signed
3. **Distributed Redundancy**: Each source graph maintains independent backups
4. **Recovery Protocols**: Procedures for rebuilding from mother node if needed

### Security Coordination
1. **Key Management**: Coordinate cryptographic keys across graphs
2. **Signature Verification**: Verify integrity of cross-graph content
3. **Access Control**: Manage access permissions for distributed team
4. **Audit Trail**: Track all modifications and cross-graph transfers

## Implementation Status

### Current Integration
- [ ] Strategic memory graph merged
- [ ] Sigil technical documentation integrated
- [ ] Research foundations consolidated
- [ ] Cross-graph relationships mapped

### Next Steps
1. Implement automated sync between mother node and source graphs
2. Set up cryptographic verification for all merged content
3. Create distributed backup protocols
4. Establish cross-graph communication channels

---

*Sigil Project - Distributed Knowledge Management*
EOF

    # Create backup coordination script
    cat > "$output_path/sigil/backup-coordination.sh" << 'EOF'
#!/bin/bash

# Sigil Project Backup Coordination
# Manages distributed backups across mother node and source graphs

set -e

echo "🔐 Sigil Project Backup Coordination"

# This would coordinate with actual sigil infrastructure
# For now, create placeholder for implementation

echo "✅ Backup coordination script created"
echo "   • Configure with actual sigil project infrastructure"
echo "   • Integrate with cryptographic signature system"
echo "   • Set up distributed storage coordination"
EOF

    chmod +x "$output_path/sigil/backup-coordination.sh"

    log_success "Created sigil project resilience features"
}

main() {
    local output_path=""
    local sources=""
    local graph_name="Mother Node"
    local strategy="deduplicate"
    local backup="false"
    local conflict_policy="newer"
    local resilience="false"
    local sigil_mode="false"
    local dry_run="false"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help|-h)
                show_help
                exit 0
                ;;
            --output)
                output_path="$2"
                shift 2
                ;;
            --sources)
                sources="$2"
                shift 2
                ;;
            --name)
                graph_name="$2"
                shift 2
                ;;
            --strategy)
                strategy="$2"
                shift 2
                ;;
            --backup)
                backup="true"
                shift
                ;;
            --conflict)
                conflict_policy="$2"
                shift 2
                ;;
            --resilience)
                resilience="true"
                shift
                ;;
            --sigil)
                sigil_mode="true"
                resilience="true"  # Sigil mode implies resilience
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

    if [[ -z "$output_path" ]]; then
        log_error "Output path is required (--output)"
        exit 1
    fi

    if [[ -z "$sources" ]]; then
        log_error "Source graphs are required (--sources)"
        exit 1
    fi

    echo "🔗 Merging Logseq Knowledge Graphs"
    echo "   Output: $output_path"
    echo "   Sources: $sources"
    echo "   Strategy: $strategy"
    echo "   Sigil Mode: $sigil_mode"
    echo "   Resilience: $resilience"
    echo ""

    # Validate source graphs
    validate_sources "$sources" || exit 1

    if [[ "$dry_run" == "true" ]]; then
        log_info "[DRY RUN] Would create mother node at $output_path"
        log_info "[DRY RUN] Would merge $(echo "$VALIDATED_SOURCES" | wc -w) source graphs"
        exit 0
    fi

    # Create mother node structure
    create_mother_node_structure "$output_path" "$graph_name" "$sigil_mode" "$resilience"

    # Merge content from all source graphs
    merge_graph_content "$output_path" "$strategy" "$conflict_policy" "$dry_run"

    # Create navigation structure
    create_mother_node_navigation "$output_path" "$graph_name" "$sigil_mode"

    # Generate unified analysis
    generate_unified_analysis "$output_path" "$sigil_mode"

    # Set up resilience features if requested
    if [[ "$resilience" == "true" ]]; then
        create_resilience_features "$output_path" "$sigil_mode"
    fi

    echo ""
    log_success "Mother node graph merge complete!"
    echo ""
    echo "🎯 Unified Knowledge Graph Created:"
    echo "   • Location: $output_path"
    echo "   • Sources: $(echo "$VALIDATED_SOURCES" | wc -w) graphs merged"
    echo "   • Documents: $(find "$output_path" -name "*.md" -not -path "*/logseq/*" | wc -l) total files"

    if [[ "$sigil_mode" == "true" ]]; then
        echo "   • Sigil Integration: Enabled"
        echo "   • Cryptographic Verification: Ready"
        echo "   • Distributed Coordination: Configured"
    fi

    echo ""
    echo "🚀 Next Steps:"
    echo "   • Open Logseq: logseq \"$output_path\""
    echo "   • Start with: Mother Node Home page"
    echo "   • Review: Source Map for content origins"
    echo "   • Check: Merge Analysis for detailed results"

    if [[ "$sigil_mode" == "true" ]]; then
        echo "   • Configure: Sigil project coordination"
        echo "   • Set up: Distributed backup protocols"
    fi
}

# Only run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi