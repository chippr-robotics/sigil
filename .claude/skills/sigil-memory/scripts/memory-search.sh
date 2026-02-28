#!/bin/bash
# Search memory files on sigil disk
set -e

SEARCH_TERM="$1"
MOUNT_PATH="${2:-/media/dontpanic/*/}"

if [ -z "$SEARCH_TERM" ]; then
    echo "Usage: memory-search.sh <search-term> [mount-path]"
    exit 1
fi

for dir in $MOUNT_PATH; do
    if [ -d "${dir}memory" ]; then
        echo "Searching in ${dir}memory..."
        grep -ril "$SEARCH_TERM" "${dir}memory/" 2>/dev/null || echo "No matches found"
        exit 0
    fi
done

echo "No memory directory found on any mounted disk"
exit 1
