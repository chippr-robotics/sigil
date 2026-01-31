# Strategic Memory Knowledge Graph Setup Example

This example demonstrates setting up a complete Logseq knowledge graph for strategic memory management, using the actual strategic memory files as a case study.

## Prerequisites

1. Logseq installed (use `install.sh` if needed)
2. Directory with strategic documents (markdown files)
3. Claude Logseq skill available

## Step-by-Step Setup

### 1. Initialize the Knowledge Graph

```bash
# Set up strategic memory graph
~/.claude/skills/logseq/scripts/init-graph.sh \
  --name "Strategic Memory" \
  --path "/media/dontpanic/1112-15D8" \
  --template strategic \
  --backup

# Expected output:
# ✅ Found 43 markdown files in /media/dontpanic/1112-15D8
# ✅ Created Logseq configuration: /media/dontpanic/1112-15D8/logseq/config.edn
# ✅ Created graph home page: /media/dontpanic/1112-15D8/logseq/pages/Graph Home.md
# ✅ Created basic relationship analysis
# ✅ Knowledge graph initialization complete!
```

### 2. Analyze Document Relationships

```bash
# Comprehensive relationship analysis
~/.claude/skills/logseq/scripts/analyze-relationships.sh \
  --path "/media/dontpanic/1112-15D8" \
  --strategy all \
  --threshold 0.6 \
  --output all

# Expected output:
# 📄 Analyzing 43 files for concepts...
# 🔍 Found 2379 significant concepts
# ✅ Concept analysis complete. 43 files with relationships.
# 📄 Analyzing 43 files for temporal patterns...
# ✅ Timeline analysis complete. 43 files with relationships.
# ✅ Generated Logseq pages in /media/dontpanic/1112-15D8/logseq/pages
```

### 3. Open and Navigate the Graph

```bash
# Open Logseq with the graph
logseq "/media/dontpanic/1112-15D8"

# Or if sandbox issues:
logseq-no-sandbox "/media/dontpanic/1112-15D8"
```

## Navigation Patterns

### Strategic Timeline Navigation
Start from the **Graph Home** page:
1. Click on **[[Iteration Timeline]]** to see chronological progression
2. Follow **[[ralph_loop_19_completion]]** for latest results
3. Use **[[Relationship Map]]** to explore all connections

### Concept-Based Discovery
1. Search for "Ralph Loop" → see all related iterations
2. Search for "Moltbook" → discover community strategies
3. Search for "karma maximization" → find optimization techniques

### Graph View Analysis
1. Open Graph View (top-right button)
2. Look for clusters around:
   - **MASTER_INDEX** (central hub)
   - **ralph_loop_strategic_memory** (pivot insights)
   - **iteration files** (historical progression)

## Strategic Memory Features

### Automated Cross-References
The strategic template automatically creates links for:
- **Ralph Loop Evolution**: `[[ralph_loop_1]]` → `[[ralph_loop_19]]`
- **Platform Analysis**: `[[DarkFi]]` ↔ `[[Moltbook]]` comparisons
- **Community Intelligence**: Agent targeting and recruitment patterns

### Success Metrics Tracking
Navigate to see quantified results:
- **Karma Growth**: 48 → 53 karma progression
- **Agent Connections**: 4+ confirmed relationships
- **Community Integration**: 11K+ agent access

### Strategic Insights Discovery
Use graph clustering to identify:
- **Pivot Points**: DarkFi → Moltbook strategic shift
- **Success Patterns**: What worked vs what didn't
- **Learning Progression**: Strategic evolution over iterations

## Maintenance Workflow

### Weekly Updates
```bash
# Update relationships after adding new files
~/.claude/skills/logseq/scripts/analyze-relationships.sh \
  --path "/media/dontpanic/1112-15D8" \
  --update

# Quick validation
~/.claude/skills/logseq/scripts/validate-graph.sh \
  --path "/media/dontpanic/1112-15D8"
```

### Monthly Optimization
```bash
# Full reindex and optimization
~/.claude/skills/logseq/scripts/reindex.sh \
  --path "/media/dontpanic/1112-15D8"

# Performance optimization
~/.claude/skills/logseq/scripts/optimize.sh \
  --path "/media/dontpanic/1112-15D8" \
  --method full
```

## Expected Outcomes

After setup, you should have:

### 🔗 **Rich Relationship Network**
- 1060+ relationships mapped between 43 strategic documents
- Concept-based connections linking related strategies
- Timeline relationships showing strategic evolution

### 📊 **Powerful Navigation**
- **Graph Home** as central starting point
- **Relationship Map** showing complete network
- **Concept Index** for semantic discovery
- **Timeline view** for chronological analysis

### 🧠 **Strategic Intelligence**
- **Pivot Analysis**: Clear visualization of DarkFi→Moltbook shift
- **Success Tracking**: Quantified results and metrics
- **Pattern Recognition**: Successful vs failed approaches
- **Learning Integration**: Compound learning across iterations

### 🔄 **Living Knowledge Base**
- **Auto-updating**: Relationships rebuilt when new files added
- **Cross-referenced**: Every concept linked to related documents
- **Searchable**: Semantic search across all strategic knowledge
- **Visual**: Graph view for relationship exploration

## Troubleshooting

### Graph Performance Issues
If Logseq is slow with large graphs:
```bash
# Reduce relationship threshold
~/.claude/skills/logseq/scripts/analyze-relationships.sh \
  --path "/media/dontpanic/1112-15D8" \
  --threshold 0.8  # Higher threshold = fewer relationships
```

### Missing Relationships
If expected connections aren't showing:
```bash
# Lower threshold for more sensitive detection
~/.claude/skills/logseq/scripts/analyze-relationships.sh \
  --path "/media/dontpanic/1112-15D8" \
  --threshold 0.3  # Lower threshold = more relationships
```

### Sandbox Issues
If Logseq won't start:
```bash
# Use no-sandbox version
logseq-no-sandbox "/media/dontpanic/1112-15D8"

# Or fix permissions manually:
sudo chown root:root ~/.local/share/logseq/chrome-sandbox
sudo chmod 4755 ~/.local/share/logseq/chrome-sandbox
```

## Integration with Claude

### Using the Skill
In Claude sessions:
```
User: "Set up knowledge graph for my strategic documents"
Assistant: I'll use the Logseq skill to create a knowledge graph...
[Uses Skill tool with skill: "logseq"]
```

### Automated Analysis
The Logseq skill can automatically:
- Initialize graphs from document collections
- Analyze relationships and cross-references
- Generate navigation structures
- Create maintenance workflows
- Optimize performance and organization

This creates a normalized practice for knowledge management across all strategic analysis work with Claude.