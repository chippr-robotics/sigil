#!/bin/bash
# Show memory status from sigil disk
set -e

MOUNT_PATH="${1:-/media/dontpanic/*/}"
MEMORY_DIR=""

for dir in $MOUNT_PATH; do
    if [ -d "${dir}memory" ]; then
        MEMORY_DIR="${dir}memory"
        break
    fi
done

if [ -z "$MEMORY_DIR" ]; then
    echo "No memory directory found on any mounted disk"
    exit 1
fi

echo "Memory directory: $MEMORY_DIR"
echo "Files: $(find "$MEMORY_DIR" -maxdepth 1 -name '*.md' | wc -l)"
echo "Usage: $(du -sh "$MEMORY_DIR" | cut -f1)"
echo "Index: $([ -f "$MEMORY_DIR/MEMORY.md" ] && echo "present" || echo "missing")"
echo ""
echo "Topics:"
find "$MEMORY_DIR" -maxdepth 1 -name '*.md' -printf '  %f\n' | sort
