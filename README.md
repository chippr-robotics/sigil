# Sigil

A physical containment system for agentic MPC management. Sigil enables secure blockchain transaction signing using 2-of-2 MPC threshold signatures with presignature-based key material stored on floppy disks.

## Overview

Sigil splits signing authority between two parties:
1. **Cold shard**: Lives on a floppy disk (via presignature shares)
2. **Agent shard**: Lives on the agent's server/device

Neither party can sign alone. The floppy doesn't hold key material - it holds **presignature shares** that are consumed on use.

### Key Properties

- **Bounded damage**: If disk is stolen, attacker gets at most N presigs (e.g., 1000)
- **Physical consent**: Agent cannot sign without disk inserted
- **Auditability**: All signing happens inside zkVM, producing proofs
- **Expiration**: Presigs expire after N days, forcing reconciliation

## Architecture

```
MOTHER DEVICE (Air-gapped)
├── cold_master_shard
├── child_registry
└── reconciliation_logs
     │
     ▼ HD derivation
CHILD FLOPPY DISK
├── child_pubkey, derivation_path
├── presig_cold_shares[1000]
├── usage_log[]
└── mother_signature
     │
     ▼ USB attached
AGENT (Daemon + CLI)
├── agent_master_shard (encrypted)
├── presig_agent_shares
└── zkVM executor (SP1)
     │
     ▼
CLAUDE CLI TOOLS
├── sign_blockchain_transaction
├── check_signing_disk
└── estimate_transaction
```

## Crate Structure

```
sigil/
├── crates/
│   ├── sigil-core/        # Shared types, disk format, crypto
│   ├── sigil-zkvm/        # SP1 signing program (no_std)
│   ├── sigil-daemon/      # System daemon, disk watcher, IPC
│   ├── sigil-cli/         # Claude CLI tools
│   └── sigil-mother/      # Air-gapped mother tooling
├── proto/
│   └── signer.proto       # IPC definitions
└── scripts/
    └── udev-rules/        # Linux disk detection
```

## Installation

### Prerequisites

- Rust 1.75+
- Linux (for udev disk detection)
- SP1 zkVM SDK (for proof generation)

### Build

```bash
cargo build --release
```

### Build with Ledger Support

To enable Ledger hardware wallet support for secure key generation on the mother device:

```bash
# Install system dependencies (Linux)
sudo apt-get install libudev-dev

# Build with Ledger feature
cargo build --release --features "sigil-mother/ledger"
```

### Install (Linux)

```bash
sudo ./scripts/install.sh
```

This will:
- Create the `sigil` group
- Install udev rules for disk detection
- Install the systemd service
- Create configuration directories

## Usage

### Mother Device (Air-gapped)

Initialize the mother device:
```bash
sigil-mother init
```

Create a new child disk:
```bash
sigil-mother create-child --presig-count 1000 --output disk.img --agent-output agent_shares.json
```

Reconcile a returning disk:
```bash
sigil-mother reconcile --disk disk.img
```

Refill a disk after reconciliation:
```bash
sigil-mother refill --disk disk.img --presig-count 1000 --agent-output new_agent_shares.json
```

### Ledger Hardware Wallet Integration

Sigil supports Ledger Nano S/X for secure master key generation. This provides hardware-backed entropy and keeps the seed derivation in the Ledger's secure element.

#### Prerequisites

1. Ledger Nano S or Nano X
2. Ledger firmware up to date
3. Ethereum app installed and open on the Ledger
4. Build with `--features ledger` enabled

#### Setup (Linux)

Add udev rules for Ledger device access:
```bash
# Create udev rules file
sudo tee /etc/udev/rules.d/20-ledger.rules << 'EOF'
# Ledger Nano S
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="0001", MODE="0660", GROUP="plugdev"
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="1011", MODE="0660", GROUP="plugdev"
# Ledger Nano X
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="0004", MODE="0660", GROUP="plugdev"
SUBSYSTEMS=="usb", ATTRS{idVendor}=="2c97", ATTRS{idProduct}=="4011", MODE="0660", GROUP="plugdev"
EOF

# Reload rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Add yourself to plugdev group
sudo usermod -aG plugdev $USER
```

#### Check Ledger Status

```bash
sigil-mother ledger-status
```

Expected output:
```
=== Ledger Device Status ===

Ledger device connected
Model: Ledger Nano
Ethereum app is open
Address: 0x742d35Cc6634C0532925a3b844Bc454e...
```

#### Initialize with Ledger

```bash
sigil-mother init --ledger
```

This will:
1. Connect to your Ledger device
2. Request a signature on a derivation message (confirm on device)
3. Derive the cold master shard from the Ledger signature
4. Generate an agent shard using software RNG
5. Display both the master public key and the agent shard

The Ledger's private key never leaves the secure element. The signature is used as a deterministic seed for the cold shard.

#### Recovery

If you need to recover the cold shard:
1. Same Ledger device with same seed phrase
2. Same derivation message (displayed during init, save it!)
3. Re-sign the message to regenerate the identical cold shard

### Agent Side

Start the daemon:
```bash
sigil-daemon
# or
sudo systemctl start sigil-daemon
```

Check daemon status:
```bash
sigil status
```

Check disk status:
```bash
sigil disk
```

Sign a message (for testing):
```bash
sigil sign --message "0x..." --chain-id 1 --description "Test transaction"
```

### Claude CLI Integration

The CLI exposes tools for Claude Code:

```rust
// Tool: sign_blockchain_transaction
SignTransactionTool {
    transaction_hash: "0x...",
    chain_id: 1,
    description: "Send 0.1 ETH to vitalik.eth"
}

// Tool: check_signing_disk
CheckDiskTool {}
```

Example Claude interaction:
```
User: "Send 0.1 ETH to vitalik.eth"

Claude: I'll prepare that transfer.

        Transaction Summary:
        ├─ To: vitalik.eth (0xd8dA...)
        ├─ Amount: 0.1 ETH (~$370)
        └─ Gas: ~0.0005 ETH ($1.85)

        Please insert your signing disk.

[User inserts floppy]

Claude: ✓ Disk detected (sigil_7a3f)
        ├─ Presigs: 847/1000 remaining
        └─ Expires: 12 days

        ✓ Signing... ✓ Proving... ✓ Broadcasting...
        ✓ Confirmed: 0x8f2a...

        Logged to disk. You may remove it now.
```

## Disk Format

The floppy disk (~1.44MB) contains:

| Offset | Size | Field |
|--------|------|-------|
| 0x0000 | 8 | Magic: "SIGILDSK" |
| 0x0008 | 4 | Version |
| 0x000C | 32 | Child ID |
| 0x002C | 33 | Child public key |
| 0x004D | 32 | Derivation path |
| 0x006D | 4 | Presig total |
| 0x0071 | 4 | Presig used |
| 0x0075 | 8 | Created timestamp |
| 0x007D | 8 | Expires timestamp |
| 0x0085 | 8 | Reconciliation deadline |
| 0x008D | 4 | Max uses before reconcile |
| 0x0091 | 4 | Uses since reconcile |
| 0x0095 | 64 | Mother signature |
| 0x0100 | 256KB | Presig table (1000 x 256 bytes) |
| 0x3E900 | ~1.1MB | Usage log |

## Security Model

### Threat: Disk Theft
- **Mitigation**: Bounded to N presignatures, all useless without agent shard
- **Detection**: Usage log anomalies during reconciliation

### Threat: Agent Compromise
- **Mitigation**: Agent cannot sign without physical disk
- **Recovery**: Nullify all children, rotate master shards

### Threat: Mother Compromise
- **Mitigation**: Air-gapped, no network access
- **Recovery**: All children become unrefillable

### Reconciliation Anomalies

During reconciliation, the mother checks for:
- Presig count mismatches
- Gaps in presig indices
- Timestamps out of order
- Missing usage log entries
- Invalid signatures

## Configuration

Daemon config (`/etc/sigil/daemon.json`):
```json
{
    "agent_store_path": "/var/lib/sigil/agent_store",
    "ipc_socket_path": "/tmp/sigil.sock",
    "enable_zkvm_proving": false,
    "disk_mount_pattern": "/media/*/SIGIL*",
    "signing_timeout_secs": 60,
    "dev_mode": false
}
```

## Development

Run tests:
```bash
cargo test
```

Run with debug logging:
```bash
RUST_LOG=debug cargo run --bin sigil-daemon
```

## License

Apache-2.0
