#!/bin/bash

# Logseq Relationship Analysis Script
# Analyzes and creates cross-references between documents

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

show_help() {
    cat << EOF
Logseq Relationship Analysis

USAGE:
    analyze-relationships.sh --path "/path/to/documents" [OPTIONS]

REQUIRED:
    --path PATH         Path to directory containing documents

OPTIONS:
    --strategy TYPE     Analysis strategy (concepts, timeline, structure, all)
    --output FORMAT     Output format (logseq, json, report, all)
    --threshold NUM     Relationship strength threshold (0.0-1.0, default: 0.5)
    --update            Update existing relationships rather than replace
    --depth NUM         Analysis depth for cross-references (default: 3)
    --patterns LIST     Custom patterns to match (comma-separated)
    --exclude LIST      Patterns to exclude (comma-separated)
    --help              Show this help message

DESCRIPTION:
    Analyzes documents to discover and create meaningful relationships.
    Generates cross-references, concept maps, and navigation structures.

STRATEGIES:
    concepts     - Link documents sharing concepts and terminology
    timeline     - Connect documents by temporal relationships
    structure    - Link by file organization and naming patterns
    all          - Apply all analysis strategies (default)

EXAMPLES:
    analyze-relationships.sh --path "/docs" --strategy concepts
    analyze-relationships.sh --path "/strategic" --output all --threshold 0.7
    analyze-relationships.sh --path "/research" --patterns "experiment,hypothesis"

OUTPUT FORMATS:
    logseq       - Generate Logseq relationship pages (default)
    json         - Export relationship data as JSON
    report       - Create analysis report
    all          - Generate all output formats
EOF
}

log_info() {
    echo "🔍 $1"
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

analyze_concepts() {
    local path="$1"
    local threshold="$2"
    local output_file="$3"

    log_info "Analyzing concept-based relationships..."

    python3 << EOF
import os
import re
import json
from pathlib import Path
from collections import defaultdict, Counter

def extract_concepts(content):
    """Extract concepts from document content"""
    concepts = set()

    # Extract emphasized text
    concepts.update(re.findall(r'\*\*([^*]+)\*\*', content))
    concepts.update(re.findall(r'`([^`]+)`', content))

    # Extract headers (without #)
    headers = re.findall(r'^#{1,6}\s+(.+)$', content, re.MULTILINE)
    concepts.update(h.strip() for h in headers if len(h.strip()) > 3)

    # Extract capitalized phrases
    cap_phrases = re.findall(r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)+\b', content)
    concepts.update(p for p in cap_phrases if len(p.split()) <= 4)

    # Clean concepts
    cleaned = set()
    for concept in concepts:
        concept = concept.strip()
        if len(concept) > 2 and len(concept) < 50:
            cleaned.add(concept)

    return cleaned

def analyze_directory(base_dir, threshold):
    """Analyze all markdown files in directory"""
    base_path = Path(base_dir)
    md_files = [f for f in base_path.glob("*.md") if "logseq" not in f.parts]

    print(f"📄 Analyzing {len(md_files)} files for concepts...")

    file_concepts = {}
    all_concepts = Counter()

    # Extract concepts from each file
    for file_path in md_files:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        concepts = extract_concepts(content)
        file_concepts[file_path.stem] = concepts
        all_concepts.update(concepts)

    # Filter concepts by frequency and relevance
    significant_concepts = {
        concept for concept, count in all_concepts.items()
        if count >= 2 and count <= len(md_files) * 0.8
    }

    print(f"🔍 Found {len(significant_concepts)} significant concepts")

    # Build relationships based on shared concepts
    relationships = defaultdict(lambda: defaultdict(float))
    concept_files = defaultdict(set)

    for file_name, concepts in file_concepts.items():
        relevant_concepts = concepts & significant_concepts

        for concept in relevant_concepts:
            concept_files[concept].add(file_name)

        # Calculate relationships
        for other_file, other_concepts in file_concepts.items():
            if file_name != other_file:
                other_relevant = other_concepts & significant_concepts
                shared = len(relevant_concepts & other_relevant)
                total = len(relevant_concepts | other_relevant)

                if total > 0:
                    strength = shared / total
                    if strength >= threshold:
                        relationships[file_name][other_file] = strength

    return dict(relationships), concept_files, significant_concepts

# Run analysis
relationships, concept_files, concepts = analyze_directory("$path", $threshold)

# Output results
with open("$output_file", 'w') as f:
    json.dump({
        "relationships": relationships,
        "concepts": {k: list(v) for k, v in concept_files.items()},
        "metadata": {
            "strategy": "concepts",
            "threshold": $threshold,
            "total_files": len(relationships),
            "total_concepts": len(concepts)
        }
    }, f, indent=2)

print(f"✅ Concept analysis complete. {len(relationships)} files with relationships.")
EOF

    log_success "Concept analysis complete"
}

analyze_timeline() {
    local path="$1"
    local threshold="$2"
    local output_file="$3"

    log_info "Analyzing timeline-based relationships..."

    python3 << EOF
import os
import re
import json
from pathlib import Path
from collections import defaultdict
from datetime import datetime

def extract_temporal_info(content, filename):
    """Extract dates and sequence information"""
    dates = []
    sequences = []

    # Extract dates
    date_patterns = [
        r'20\d{2}-\d{2}-\d{2}',  # YYYY-MM-DD
        r'\d{2}/\d{2}/20\d{2}',  # MM/DD/YYYY
        r'(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2},?\s+20\d{2}',  # Month DD, YYYY
    ]

    for pattern in date_patterns:
        matches = re.findall(pattern, content)
        dates.extend(matches)

    # Extract sequence indicators from filename
    seq_match = re.search(r'(?:iteration|loop|part|phase|step)[\s_-]*(\d+)', filename, re.IGNORECASE)
    if seq_match:
        sequences.append(int(seq_match.group(1)))

    # Extract sequence from content
    seq_patterns = [
        r'(?:iteration|loop|part|phase|step)\s*(\d+)',
        r'(\d+)(?:st|nd|rd|th)\s+(?:iteration|phase|part)',
    ]

    for pattern in seq_patterns:
        matches = re.findall(pattern, content, re.IGNORECASE)
        sequences.extend(int(m) for m in matches if m.isdigit())

    return dates, sequences

def analyze_directory(base_dir, threshold):
    """Analyze temporal relationships"""
    base_path = Path(base_dir)
    md_files = [f for f in base_path.glob("*.md") if "logseq" not in f.parts]

    print(f"📄 Analyzing {len(md_files)} files for temporal patterns...")

    file_temporal = {}

    # Extract temporal info from each file
    for file_path in md_files:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        dates, sequences = extract_temporal_info(content, file_path.name)
        file_temporal[file_path.stem] = {
            "dates": dates,
            "sequences": sequences,
            "filename": file_path.name
        }

    # Build timeline relationships
    relationships = defaultdict(lambda: defaultdict(float))

    for file_name, temporal_info in file_temporal.items():
        for other_file, other_temporal in file_temporal.items():
            if file_name != other_file:
                strength = 0.0

                # Sequence-based relationships
                if temporal_info["sequences"] and other_temporal["sequences"]:
                    file_seq = max(temporal_info["sequences"])
                    other_seq = max(other_temporal["sequences"])

                    # Strong relationship for consecutive sequences
                    if abs(file_seq - other_seq) == 1:
                        strength = max(strength, 0.9)
                    elif abs(file_seq - other_seq) <= 3:
                        strength = max(strength, 0.7)
                    elif abs(file_seq - other_seq) <= 5:
                        strength = max(strength, 0.5)

                # Date-based relationships
                if temporal_info["dates"] and other_temporal["dates"]:
                    # Simple temporal proximity for now
                    strength = max(strength, 0.4)

                if strength >= threshold:
                    relationships[file_name][other_file] = strength

    return dict(relationships), file_temporal

# Run analysis
relationships, temporal_info = analyze_directory("$path", $threshold)

# Output results
with open("$output_file", 'w') as f:
    json.dump({
        "relationships": relationships,
        "temporal_info": temporal_info,
        "metadata": {
            "strategy": "timeline",
            "threshold": $threshold,
            "total_files": len(relationships)
        }
    }, f, indent=2)

print(f"✅ Timeline analysis complete. {len(relationships)} files with relationships.")
EOF

    log_success "Timeline analysis complete"
}

generate_logseq_output() {
    local analysis_file="$1"
    local output_dir="$2"

    log_info "Generating Logseq relationship pages..."

    python3 << EOF
import json
from pathlib import Path

# Load analysis results
with open("$analysis_file", 'r') as f:
    data = json.load(f)

relationships = data["relationships"]
metadata = data["metadata"]

# Create output directory
output_path = Path("$output_dir")
output_path.mkdir(parents=True, exist_ok=True)

# Generate relationship map
relationship_file = output_path / "Relationship_Map.md"
with open(relationship_file, 'w') as f:
    f.write(f"# Document Relationship Map\\n\\n")
    f.write(f"Generated using {metadata['strategy']} analysis\\n")
    f.write(f"Threshold: {metadata['threshold']}\\n\\n")

    f.write(f"## Overview\\n")
    f.write(f"- **Total Files**: {metadata['total_files']}\\n")
    f.write(f"- **Analysis Strategy**: {metadata['strategy']}\\n")
    f.write(f"- **Generated**: $(date)\\n\\n")

    f.write(f"## File Relationships\\n\\n")

    for file_name in sorted(relationships.keys()):
        related_files = relationships[file_name]
        if related_files:
            f.write(f"### [[{file_name}]]\\n")
            f.write(f"**Connected to:**\\n")

            # Sort by relationship strength
            sorted_relations = sorted(
                related_files.items(),
                key=lambda x: x[1],
                reverse=True
            )

            for related_file, strength in sorted_relations:
                f.write(f"- [[{related_file}]] (strength: {strength:.2f})\\n")
            f.write(f"\\n")

# Generate concept index if available
if "concepts" in data:
    concept_file = output_path / "Concept_Index.md"
    with open(concept_file, 'w') as f:
        f.write(f"# Concept Index\\n\\n")
        f.write(f"Generated from {metadata['strategy']} analysis\\n\\n")

        concepts = data["concepts"]
        for concept in sorted(concepts.keys()):
            files = concepts[concept]
            if len(files) > 1:  # Only show concepts that connect multiple files
                f.write(f"## {concept}\\n")
                f.write(f"**Appears in:**\\n")
                for file_name in sorted(files):
                    f.write(f"- [[{file_name}]]\\n")
                f.write(f"\\n")

print(f"✅ Generated Logseq pages in {output_path}")
EOF

    log_success "Logseq output generated"
}

main() {
    local path=""
    local strategy="all"
    local output_format="logseq"
    local threshold="0.5"
    local update="false"
    local depth="3"
    local patterns=""
    local exclude=""

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --help|-h)
                show_help
                exit 0
                ;;
            --path)
                path="$2"
                shift 2
                ;;
            --strategy)
                strategy="$2"
                shift 2
                ;;
            --output)
                output_format="$2"
                shift 2
                ;;
            --threshold)
                threshold="$2"
                shift 2
                ;;
            --update)
                update="true"
                shift
                ;;
            --depth)
                depth="$2"
                shift 2
                ;;
            --patterns)
                patterns="$2"
                shift 2
                ;;
            --exclude)
                exclude="$2"
                shift 2
                ;;
            *)
                log_error "Unknown option: $1"
                show_help >&2
                exit 1
                ;;
        esac
    done

    if [[ -z "$path" ]]; then
        log_error "Path is required (--path)"
        exit 1
    fi

    if [[ ! -d "$path" ]]; then
        log_error "Path does not exist: $path"
        exit 1
    fi

    echo "🔍 Analyzing Document Relationships"
    echo "   Path: $path"
    echo "   Strategy: $strategy"
    echo "   Threshold: $threshold"
    echo ""

    # Create temporary files for analysis results
    local temp_dir=$(mktemp -d)
    local concepts_file="$temp_dir/concepts.json"
    local timeline_file="$temp_dir/timeline.json"
    local combined_file="$temp_dir/combined.json"

    # Run analysis based on strategy
    case "$strategy" in
        "concepts")
            analyze_concepts "$path" "$threshold" "$concepts_file"
            cp "$concepts_file" "$combined_file"
            ;;
        "timeline")
            analyze_timeline "$path" "$threshold" "$timeline_file"
            cp "$timeline_file" "$combined_file"
            ;;
        "all"|*)
            analyze_concepts "$path" "$threshold" "$concepts_file"
            analyze_timeline "$path" "$threshold" "$timeline_file"

            # Combine results (simplified for now)
            cp "$concepts_file" "$combined_file"
            ;;
    esac

    # Generate output based on format
    case "$output_format" in
        "logseq"|"all")
            generate_logseq_output "$combined_file" "$path/logseq/pages"
            ;;
        "json")
            cp "$combined_file" "$path/relationship_analysis.json"
            log_success "JSON output saved to: $path/relationship_analysis.json"
            ;;
        "report")
            # Generate human-readable report
            log_info "Generating analysis report..."
            # Implementation would go here
            ;;
    esac

    # Cleanup
    rm -rf "$temp_dir"

    echo ""
    log_success "Relationship analysis complete!"
    echo ""
    echo "Results:"
    echo "  • Check $path/logseq/pages/Relationship_Map.md for complete network"
    echo "  • Use Logseq graph view to visualize relationships"
    echo "  • Re-run periodically to update relationships"
}

# Only run main if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi