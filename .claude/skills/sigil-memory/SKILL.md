---
name: sigil:memory
description: Memory management for sigil disk. Stores and retrieves plain markdown notes on the sigil floppy disk following Obsidian-compatible conventions.
version: 3.0.0
allowed-tools: Read, Write, Edit, Bash
---

# Sigil Memory Management

## Overview

Stores concise markdown notes on the sigil floppy disk at `{disk_mount}/memory/`.
Follows the Claude Code auto-memory pattern with Obsidian-compatible conventions.

## Directory Structure

```
{disk_mount}/memory/
├── MEMORY.md           # Concise index loaded into context
├── architecture.md     # Detailed notes on architecture decisions
├── debugging.md        # Debugging patterns and solutions
├── signing-patterns.md # Common signing workflows
└── ...                 # Any other topic files
```

## Conventions

- **MEMORY.md**: Concise index with `[[wikilinks]]` to topic files. Keep under 200 lines.
- **Topic files**: Standard markdown with optional YAML frontmatter for date/tags.
- **Wikilinks**: Use `[[topic-name]]` for cross-references (Obsidian-compatible).
- **Frontmatter**: Optional. Use `date:` and `tags:` fields when useful.
- **1.44MB constraint**: The floppy disk is small. Keep notes concise and high-signal.

## IPC Commands

The sigil daemon provides these memory operations via IPC:

- **MemoryStore**: Store a note under a topic key. Creates or appends to a topic file.
- **MemoryQuery**: Search notes by text terms across all memory files.
- **MemoryStatus**: Check disk usage, file count, and index presence.

## Principles

1. Keep it simple -- plain markdown, no special syntax
2. Organize by topic, not chronologically
3. Prefer concise notes over exhaustive documentation
4. Use wikilinks for cross-references between topics
5. Respect the 1.44MB disk constraint
