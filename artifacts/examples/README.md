# Example Artifacts

This directory contains reference artifacts for documentation and demonstration purposes.

## Overview

The example artifacts demonstrate various states and operations in the Sigil system:

- **Genesis artifacts**: First child disk created after mother initialization
- **Used artifacts**: Disks after some signing operations
- **Refilled artifacts**: Disks after reconciliation and refill
- **Edge case artifacts**: Special states (expired, nullified, etc.)

## Generating Example Artifacts

To generate example artifacts for this directory, use the provided script:

```bash
# From repository root
./scripts/generate_example_artifacts.sh
```

Or manually create them:

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

## Current Examples

### (No examples yet)

Example artifacts can be generated using the methods above. Once created, document them here.

## Guidelines for Adding Examples

When adding new example artifacts:

1. Create the artifact using the commands above
2. Add entry in "Current Examples" section above
3. Include description with:
   - Purpose of the artifact
   - Number of presignatures
   - Status (fresh/used/nullified/etc.)
   - Use case or demonstration purpose

Example entry format:

```markdown
### basic_child.img

- **Purpose**: Basic example of a freshly created child disk
- **Presignatures**: 100 (all unused)
- **Status**: Fresh, never used
- **Use case**: Introduction to child disk structure
- **Paired with**: basic_child_agent_shares.json
```

## Size Guidelines

Keep examples small to avoid repository bloat:
- Maximum 2MB per disk image
- Use minimal presignature counts (100-500 max)
- Remove obsolete examples when format changes

## Security Note

⚠️ All example artifacts use **test keys only**. Never use these in production!

These artifacts are generated with the `sigil-mother` tool in a test environment and contain no real key material.

