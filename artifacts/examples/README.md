# Example Artifacts

This directory contains reference artifacts for documentation and demonstration purposes.

## Overview

The example artifacts demonstrate various states and operations in the Sigil system:

- **Genesis artifacts**: First child disk created after mother initialization
- **Used artifacts**: Disks after some signing operations
- **Refilled artifacts**: Disks after reconciliation and refill
- **Edge case artifacts**: Special states (expired, nullified, etc.)

## Creating Example Artifacts

Example artifacts should be created using the standard Sigil tools with test data.

### Example: Creating a Basic Child Disk

```bash
# Initialize test mother
sigil-mother init --data-dir /tmp/example_mother

# Create first child disk
sigil-mother create-child \
  --presig-count 100 \
  --output artifacts/examples/basic_child.img \
  --agent-output artifacts/examples/basic_child_agent_shares.json \
  --data-dir /tmp/example_mother
```

## Using Examples

These artifacts can be used in:

1. **Documentation**: Reference in guides and tutorials
2. **Manual testing**: Load in test environments
3. **Development**: Understand disk format and structure
4. **Demonstrations**: Show Sigil capabilities

## Artifact Descriptions

When adding new example artifacts, create entries here:

### basic_child.img (Coming Soon)

- **Purpose**: Basic example of a freshly created child disk
- **Presignatures**: 100
- **Status**: Fresh, never used
- **Use case**: Introduction to child disk structure

### used_child.img (Coming Soon)

- **Purpose**: Example of a disk after several signing operations
- **Presignatures**: 50 used, 50 remaining
- **Status**: Partially used
- **Use case**: Demonstrate usage tracking and logging

## Guidelines

1. Keep examples small (< 2MB per disk)
2. Use minimal presignature counts (100-500 max)
3. Document each artifact clearly
4. Update when disk format changes
5. Remove obsolete examples

## Security Note

⚠️ All example artifacts use **test keys only**. Never use these in production!
