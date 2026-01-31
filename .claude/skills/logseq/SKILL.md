---
name: logseq
description: This skill should be used when the user wants to "use Logseq", "create knowledge graph", "analyze relationships", "set up graph database", "manage notes with Logseq", "build knowledge management system", or needs to integrate structured note-taking and graph visualization.
version: 1.0.0
allowed-tools: Read, Write, Edit, Bash, Glob, Grep
---

# Logseq Knowledge Management Skill

## Overview

Logseq is a local-first, block-based knowledge management tool that creates powerful graph databases from markdown files. Unlike traditional note-taking apps, Logseq emphasizes:

- **Bi-directional linking**: Automatic relationship discovery between concepts
- **Block-based structure**: Granular content organization and referencing
- **Graph visualization**: Visual exploration of knowledge networks
- **Local ownership**: All data stored locally, privacy-first approach
- **Plugin ecosystem**: Extensible functionality for specialized workflows

This skill provides comprehensive integration between Claude and Logseq for:
- **Knowledge graph creation** from existing document collections
- **Relationship analysis** and cross-referencing systems
- **Strategic memory management** for ongoing projects
- **Concept tracking** across multiple documents and timelines
- **Automated indexing** and graph maintenance

## Quick Start

### Installation & Setup

Install Logseq and configure for Claude integration:

```bash
# Install Logseq (if not already installed)
~/.claude/skills/logseq/scripts/install.sh

# Set up a new knowledge graph
~/.claude/skills/logseq/scripts/setup-graph.sh --directory "/path/to/documents"

# Analyze existing documents for relationships
~/.claude/skills/logseq/scripts/analyze-relationships.sh --directory "/path/to/documents"
```

### Create Your First Graph

Transform a directory of documents into a Logseq knowledge graph:

```bash
# Initialize graph structure
~/.claude/skills/logseq/scripts/init-graph.sh --name "Strategic Memory" --path "/media/dontpanic/1112-15D8"

# Generate cross-references
~/.claude/skills/logseq/scripts/cross-reference.sh --path "/media/dontpanic/1112-15D8"

# Create concept index
~/.claude/skills/logseq/scripts/index-concepts.sh --path "/media/dontpanic/1112-15D8"
```

## Core Operations

### Graph Initialization

**Set up new knowledge graphs for any document collection**

```bash
# Basic graph setup
scripts/init-graph.sh --name "Project Name" --path "/path/to/docs"

# Advanced setup with templates
scripts/init-graph.sh --name "Strategic Analysis" --path "/path/to/docs" --template strategic

# Multi-directory graph
scripts/init-graph.sh --name "Research Collection" --paths "/docs1,/docs2,/docs3"
```

**Configuration Options**

- Graph naming and metadata
- Template selection (strategic, research, project, general)
- Multi-directory support for complex projects
- Automatic config generation

### Relationship Analysis

**Automated relationship discovery and mapping**

```bash
# Full relationship analysis
scripts/analyze-relationships.sh --path "/path/to/docs" --output-format json

# Concept-based linking
scripts/cross-reference.sh --path "/path/to/docs" --strategy concepts

# Timeline-based connections
scripts/cross-reference.sh --path "/path/to/docs" --strategy timeline

# Custom relationship patterns
scripts/cross-reference.sh --path "/path/to/docs" --patterns "iteration,loop,strategy"
```

**Analysis Types**
- Direct file references and links
- Concept-based relationships (shared terminology)
- Timeline connections (chronological relationships)
- Structural similarities (file organization patterns)

### Graph Maintenance

**Keep knowledge graphs updated and optimized**

```bash
# Reindex entire graph
scripts/reindex.sh --path "/path/to/docs"

# Update relationships for new files
scripts/update-relationships.sh --path "/path/to/docs" --incremental

# Validate graph integrity
scripts/validate-graph.sh --path "/path/to/docs" --report detailed

# Optimize graph performance
scripts/optimize.sh --path "/path/to/docs" --method full
```

### Graph Merging & Distributed Management

**Merge multiple knowledge graphs into unified mother nodes for maximum resilience**

```bash
# Basic graph merging
scripts/merge-graphs.sh --output "/unified-graph" --sources "/graph1,/graph2,/graph3"

# Advanced merge with conflict resolution
scripts/merge-graphs.sh --output "/mother-node" --sources "/strategic,/technical" --strategy deduplicate --conflict newer

# Sigil project integration with maximum resilience
scripts/merge-graphs.sh --output "/sigil-mother" --sources "/strategic,/sigil" --sigil --resilience --backup

# Distributed coordination setup
scripts/setup-distributed.sh --mother-node "/central" --satellites "/graph1,/graph2" --sync-schedule daily
```

**Distributed Architecture Support**
- Mother node creation for centralized backup and coordination
- Cross-graph relationship preservation and analysis
- Cryptographic verification integration (sigil project)
- Automated sync between distributed graphs
- Maximum resilience configurations

## Advanced Features

### Strategic Memory Integration

**Specialized support for ongoing strategic analysis and iteration**

```bash
# Set up strategic memory tracking
scripts/setup-strategic.sh --base-dir "/path/to/strategic/docs"

# Track iteration progression
scripts/track-iterations.sh --pattern "iteration_*" --output timeline

# Generate strategic insights
scripts/analyze-strategy.sh --path "/path/to/strategic" --focus "ralph_loop,community"

# Create strategic dashboards
scripts/create-dashboard.sh --type strategic --data "/path/to/strategic"
```

**Strategic Templates**
- Ralph Loop analysis patterns
- Community engagement tracking
- Technical infrastructure documentation
- Success metrics and KPI monitoring

### Concept Management

**Advanced concept tracking and relationship building**

```bash
# Extract and index concepts
scripts/extract-concepts.sh --path "/path/to/docs" --method semantic

# Build concept hierarchies
scripts/build-hierarchy.sh --concepts-file concepts.json --method similarity

# Generate concept maps
scripts/concept-map.sh --path "/path/to/docs" --format graphviz

# Track concept evolution
scripts/track-concepts.sh --path "/path/to/docs" --timeline
```

### Graph Visualization

**Create powerful visual representations of knowledge networks**

```bash
# Generate graph visualizations
scripts/visualize.sh --path "/path/to/docs" --format interactive

# Create relationship matrices
scripts/relationship-matrix.sh --path "/path/to/docs" --output svg

# Build hierarchical views
scripts/hierarchy-view.sh --path "/path/to/docs" --depth 3

# Export for external tools
scripts/export-graph.sh --path "/path/to/docs" --format cytoscape
```

## Integration Workflows

### Document Analysis Pipeline

**Complete workflow for transforming document collections into knowledge graphs**

1. **Discovery Phase**
   ```bash
   scripts/discover-structure.sh --path "/path/to/docs"
   ```
   - Analyze file organization patterns
   - Identify document types and relationships
   - Generate initial metadata

2. **Relationship Building**
   ```bash
   scripts/build-relationships.sh --path "/path/to/docs" --strategy comprehensive
   ```
   - Extract direct links and references
   - Identify concept-based connections
   - Create timeline relationships

3. **Graph Creation**
   ```bash
   scripts/create-graph.sh --path "/path/to/docs" --config auto-generated
   ```
   - Generate Logseq configuration
   - Create page templates and indexes
   - Set up navigation structures

4. **Optimization**
   ```bash
   scripts/optimize-graph.sh --path "/path/to/docs" --focus navigation
   ```
   - Improve graph performance
   - Optimize relationship networks
   - Generate usage analytics

### Ongoing Maintenance

**Automated workflows for keeping graphs current and useful**

```bash
# Daily maintenance (can be automated)
scripts/daily-maintenance.sh --path "/path/to/docs"

# Weekly analysis updates
scripts/weekly-analysis.sh --path "/path/to/docs"

# Monthly optimization
scripts/monthly-optimization.sh --path "/path/to/docs"
```

## Configuration Management

### Graph Settings

**Logseq configuration optimization for different use cases**

Create optimized configs via:

```bash
# Strategic analysis configuration
scripts/config-strategic.sh --output "/path/to/docs/logseq/config.edn"

# Research project configuration
scripts/config-research.sh --output "/path/to/docs/logseq/config.edn"

# General knowledge management configuration
scripts/config-general.sh --output "/path/to/docs/logseq/config.edn"
```

**Configuration Features**
- Graph view optimization settings
- Relationship threshold tuning
- Performance optimization for large graphs
- Template and plugin configurations

### Template System

**Reusable templates for different document and project types**

Templates available:
- `strategic` - For strategic analysis and planning documents
- `research` - For research projects and literature reviews
- `project` - For project management and tracking
- `personal` - For personal knowledge management
- `technical` - For technical documentation and code analysis

Access via:
```bash
scripts/apply-template.sh --template strategic --path "/path/to/docs"
```

## Integration with Claude Workflows

### Memory Management

**Structured approach to maintaining strategic memory and context**

```bash
# Create memory indexes for Claude context
scripts/create-memory-index.sh --path "/strategic/docs" --format claude

# Generate relationship summaries
scripts/summarize-relationships.sh --path "/strategic/docs" --depth 2

# Build context maps for complex projects
scripts/build-context.sh --path "/strategic/docs" --focus recent
```

### Analysis Support

**Tools for supporting Claude's analysis and decision-making**

```bash
# Generate analysis dashboards
scripts/analysis-dashboard.sh --path "/project/docs" --metrics full

# Create decision support indexes
scripts/decision-support.sh --path "/project/docs" --format structured

# Build knowledge synthesis views
scripts/synthesis-view.sh --path "/project/docs" --strategy comprehensive
```

## Best Practices

### Graph Design

**Principles for creating effective knowledge graphs**

- **Start simple**: Begin with basic linking, expand gradually
- **Use consistent naming**: Establish conventions for files and concepts
- **Regular maintenance**: Schedule periodic reindexing and optimization
- **Meaningful relationships**: Focus on value-adding connections
- **Progressive enhancement**: Build complexity over time

### Document Organization

**Structuring documents for optimal graph creation**

- **Clear hierarchies**: Organize files logically for automatic relationship detection
- **Consistent metadata**: Use standardized headers and front matter
- **Link intentionally**: Create explicit connections between related concepts
- **Document evolution**: Track changes and iterations systematically

### Performance Optimization

**Keeping graphs responsive and useful**

- **Regular cleanup**: Remove orphaned links and unused concepts
- **Relationship pruning**: Focus on high-value connections
- **Index optimization**: Maintain efficient search and navigation
- **Size management**: Monitor and manage graph complexity

## Troubleshooting

### Common Issues

**Installation Problems**
- Logseq sandbox configuration (run with --no-sandbox if needed)
- Permission issues with local directories
- Missing dependencies for graph generation

**Graph Performance**
- Large file collections causing slow loading
- Too many relationships creating visual clutter
- Configuration optimization for better performance

**Relationship Quality**
- False positive connections from automated analysis
- Missing important relationships between concepts
- Inconsistent linking patterns across documents

### Solutions

See `references/troubleshooting.md` for detailed solutions and `examples/` directory for working implementations.

## References

**Complete Documentation**

- `references/api-reference.md` - Complete script documentation
- `references/configuration-guide.md` - Detailed configuration options
- `references/graph-theory.md` - Knowledge graph principles and best practices
- `references/integration-patterns.md` - Claude integration workflows

**Working Examples**

- `examples/strategic-memory.md` - Complete strategic memory setup
- `examples/research-project.md` - Research project knowledge graph
- `examples/technical-docs.md` - Technical documentation analysis
- `examples/maintenance-workflows.md` - Ongoing maintenance examples

All scripts include detailed help via `--help` flag and are designed for both interactive and automated use.

## Helper Scripts Reference

Located in `~/.claude/skills/logseq/scripts/`:

### Core Operations
- `install.sh` - Install and configure Logseq
- `init-graph.sh` - Initialize new knowledge graphs
- `analyze-relationships.sh` - Comprehensive relationship analysis
- `cross-reference.sh` - Build cross-references between documents
- `reindex.sh` - Rebuild graph indexes and relationships

### Graph Management
- `setup-graph.sh` - Complete graph setup workflow
- `validate-graph.sh` - Check graph integrity and quality
- `optimize.sh` - Performance optimization and cleanup
- `export-graph.sh` - Export for external tools and analysis
- `visualize.sh` - Generate visual representations

### Specialized Workflows
- `setup-strategic.sh` - Strategic memory configuration
- `track-iterations.sh` - Timeline and iteration analysis
- `concept-map.sh` - Advanced concept relationship mapping
- `create-dashboard.sh` - Generate analysis dashboards
- `daily-maintenance.sh` - Automated maintenance workflows

### Graph Merging & Distributed Management
- `merge-graphs.sh` - Merge multiple graphs into unified mother nodes
- `setup-distributed.sh` - Configure distributed graph architectures
- `sync-graphs.sh` - Synchronize between mother node and satellite graphs
- `verify-integrity.sh` - Cryptographic verification of merged content
- `restore-from-mother.sh` - Emergency recovery from mother node backups

Each script provides comprehensive help and examples via the `--help` option.