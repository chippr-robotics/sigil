# sigil

A physical containment system for agentic MPC management

## Overview

Sigil is an MPC-secured floppy disk signing system that eliminates single points of key exposure while preserving physical consent for blockchain transactions. The system uses 2-of-2 threshold signatures with presignatures stored on physical media (floppy disks).

## Key Features

- **MPC Security**: Split signing authority between cold (floppy) and agent shards
- **Physical Consent**: Agent cannot sign without disk inserted
- **Bounded Damage**: Stolen disks expose at most N presignatures (e.g., 1000)
- **Auditability**: All signing operations logged on disk
- **Expiration**: Presigs expire after N days, forcing reconciliation
- **HD Derivation**: BIP44-compatible hierarchical key derivation

## Architecture

```
MOTHER DEVICE (Air-gapped)
├── cold_master_shard
├── child_registry
└── reconciliation_logs

     │ HD derivation (SLIP-10)
     ▼

CHILD FLOPPY DISK
├── child_id, child_pubkey
├── presig_cold_shares[1000]  ← THE FUEL
├── usage_log[]
└── expiry configuration

     │ USB attached
     ▼

AGENT (Claude CLI + Daemon)
├── agent_master_shard (encrypted)
├── presig_agent_shares{child_id: [1000]}
└── zkVM executor (planned)
```

## Project Structure

```
sigil/
├── crates/
│   ├── sigil-core/           # Core types, disk format, crypto
│   └── sigil-cli/            # CLI tool (planned)
└── Cargo.toml
```

## Disk Format

### Floppy Disk (~1.44MB)

- **Header** (256 bytes): Magic, version, child ID, pubkey, expiry config
- **Presignature Table** (256KB): 1000 × 256-byte presignature shares
- **Usage Log** (~1.1MB): Transaction history with signatures

### Expiry Configuration

- **Presig Validity**: 30 days (default)
- **Reconciliation Deadline**: 45 days (default)
- **Max Uses Before Reconcile**: 500 transactions (default)

## Usage (Planned)

```bash
# Initialize storage
sigil storage init --path /media/floppy

# Create presignatures for a child shard
sigil presig generate --child-index 0 --count 1000

# Sign a transaction (requires disk insertion)
sigil transaction sign --tx transaction.json --output signed.json

# Check disk status
sigil disk info --path /media/floppy
```

## Development

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run linter
cargo clippy

# Format code
cargo fmt
```

## Security Model

- **2-of-2 MPC**: Neither cold nor agent shard can sign alone
- **Presignatures**: Pre-computed nonce shares consumed on use
- **Physical Consent**: Disk must be physically present for signing
- **Usage Audit**: All signatures logged with transaction details
- **Expiration**: Time-based expiry forces regular reconciliation

## License

MIT
