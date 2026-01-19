# Sigil Development Artifacts

This directory contains development artifacts used for testing, examples, and collaboration on the Sigil MPC signing system.

## Directory Structure

```
artifacts/
├── child_disks/     # Child disk images (.img files)
├── agent_shards/    # Agent shard data files (.json files)
└── examples/        # Example/demo artifacts for documentation
```

## Purpose

When developing and testing Sigil, developers create various artifacts:

- **Child disk images** (`.img` files): Physical floppy disk images containing presignature shares
- **Agent shards** (`.json` files): Agent-side key material and presignature shares
- **Test fixtures**: Sample artifacts for automated testing

These artifacts should be tracked in version control when they are:

1. Used for reproducible tests
2. Needed for documentation or examples
3. Required for collaboration between developers
4. Part of the test suite or example workflows

## Naming Conventions

### Child Disk Images

Place child disk images in `child_disks/` with descriptive names:

```
child_disks/
├── test_disk_1000_presigs.img
├── example_ethereum_child.img
├── demo_expired_disk.img
└── e2e_test_fresh_disk.img
```

**Naming pattern**: `<purpose>_<description>.img`

Examples:
- `test_basic_signing.img` - Basic test disk for signing operations
- `example_1000_presigs.img` - Example disk with 1000 presignatures
- `demo_nullified.img` - Demo of a nullified disk state

### Agent Shards

Place agent shard files in `agent_shards/` with matching names:

```
agent_shards/
├── test_disk_1000_presigs_agent_shares.json
├── example_ethereum_child_agent_shares.json
├── demo_expired_disk_agent_shares.json
└── e2e_test_fresh_disk_agent_shares.json
```

**Naming pattern**: `<matching_disk_name>_agent_shares.json`

The agent shard filename should match the corresponding disk image name (without the `.img` extension), followed by `_agent_shares.json`.

### Examples

The `examples/` directory contains reference artifacts for documentation:

```
examples/
├── README.md                      # Documentation for example artifacts
├── genesis_child.img              # Example: First child disk after mother init
├── genesis_child_agent_shares.json
├── refilled_child.img             # Example: Child after reconciliation and refill
└── refilled_child_agent_shares.json
```

## When to Commit Artifacts

✅ **DO commit artifacts when:**

- They are part of the test suite and needed for reproducible tests
- They serve as examples in documentation
- They demonstrate specific features or edge cases
- They are needed by other developers for collaboration
- They are small enough to not bloat the repository (< 5MB per file)

❌ **DO NOT commit artifacts when:**

- They are temporary test outputs
- They contain sensitive or production key material
- They are large (> 5MB) and can be regenerated
- They are personal development artifacts not needed by others
- They are generated during CI/CD runs

## Security Considerations

⚠️ **IMPORTANT**: Artifacts in this directory are for **development and testing only**.

- Never commit artifacts with real key material or production keys
- All artifacts should use test/dummy keys only
- Disk images should be clearly marked as test artifacts
- Agent shards should be from test key generation only

## Creating Test Artifacts

### Creating a Test Child Disk

On the mother device (for testing):

```bash
# Initialize test mother device
sigil-mother init --data-dir ./test_mother_data

# Create test child disk
sigil-mother create-child \
  --presig-count 100 \
  --output artifacts/child_disks/test_basic_100_presigs.img \
  --agent-output artifacts/agent_shards/test_basic_100_presigs_agent_shares.json \
  --data-dir ./test_mother_data
```

### Using Artifacts in Tests

Tests can reference artifacts using relative paths:

```rust
#[test]
fn test_disk_loading() {
    let disk_path = "artifacts/child_disks/test_basic_100_presigs.img";
    let disk = DiskFormat::from_file(disk_path).unwrap();
    assert_eq!(disk.header.presig_total, 100);
}
```

## Artifact Lifecycle

1. **Creation**: Developer creates artifact during testing
2. **Validation**: Verify artifact works as expected
3. **Documentation**: Add comments or update this README
4. **Commit**: Add artifact to git with descriptive commit message
5. **Review**: Team reviews artifact in PR
6. **Maintenance**: Update or remove when no longer needed

## Maintenance

Periodically review artifacts in this directory:

- Remove obsolete test artifacts
- Update examples when disk format changes
- Regenerate artifacts if format/structure evolves
- Keep only necessary artifacts to avoid repository bloat

## Examples

See `examples/README.md` for detailed examples and use cases.

## Related Documentation

- [E2E Test Plan](../docs/E2E_TEST_PLAN.md) - End-to-end testing procedures
- [README.md](../README.md) - Main project documentation
- [SECURITY.md](../SECURITY.md) - Security considerations
