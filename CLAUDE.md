# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build (requires libudev-dev on Linux: sudo apt-get install libudev-dev)
cargo build --release

# Build with Ledger hardware wallet support
cargo build --release --features "sigil-mother/ledger"

# Run tests
cargo test                              # All tests
cargo test -p sigil-core                # Single crate
cargo test -p sigil-core --lib          # Unit tests only
cargo test -p sigil-core --test disk_format_tests  # Specific integration test
cargo test -p sigil-tests --test e2e_workflow_test # E2E tests

# Lint and format
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --fix --workspace --all-targets --allow-dirty --allow-staged  # Auto-fix

# Generate docs
cargo doc --workspace --no-deps

# Run with debug logging
RUST_LOG=debug cargo run --bin sigil-daemon
```

## Fuzzing

```bash
cd fuzz
cargo +nightly fuzz run fuzz_disk_format   # or fuzz_presig, fuzz_usage_log, etc.
```

## Architecture

Sigil is a 2-of-2 MPC threshold signing system where signing authority is split between:
- **Cold shard**: Stored on a floppy disk as presignature shares (physical containment)
- **Agent shard**: Stored on the agent's server/device

Neither party can sign alone. The floppy holds **presignatures** (pre-computed partial signatures) that are consumed on use, bounding potential damage if the disk is stolen.

### System Components

```
MOTHER DEVICE (air-gapped)          AGENT DEVICE (network-connected)
├── cold_master_shard               ├── agent_master_shard (encrypted)
├── child_registry                  ├── sigil-daemon (disk watcher, IPC)
└── sigil-mother / sigil-mother-tui └── sigil-cli / sigil-mcp
         │                                    │
         └───── CHILD FLOPPY DISK ────────────┘
                ├── presig_cold_shares[1000]
                ├── usage_log[]
                └── mother_signature
```

### Crate Responsibilities

- **sigil-core**: Foundational types, disk format (`DiskHeader`, `DiskFormat`), crypto primitives, presignature structures. Supports `no_std` for zkVM.
- **sigil-frost**: FROST threshold Schnorr signatures for multiple curves (secp256k1-tr/Taproot, Ed25519, Ristretto255). Includes DKG ceremony support.
- **sigil-mother**: Air-gapped tooling: key generation, child disk creation, presignature generation, reconciliation, nullification, agent registry.
- **sigil-mother-tui**: Terminal UI for mother device operations.
- **sigil-mother-zkvm**: SP1 zkVM programs for provable mother operations (keygen, derivation, batch ops).
- **sigil-daemon**: System daemon that watches for floppy disk insertion via udev, handles IPC, manages signing requests.
- **sigil-cli**: CLI tools for signing operations and disk management.
- **sigil-mcp**: Model Context Protocol server enabling AI agents to sign transactions. Implements tools like `sign_evm`, `sign_frost`, `check_disk`.
- **sigil-zkvm**: SP1 zkVM signing program (no_std) that produces proofs of correct signing.
- **sigil-tests**: End-to-end integration tests.

### Key Data Flow

1. **Child disk creation** (mother): Generate presignatures, split between cold/agent shares
2. **Disk detection** (agent): Daemon detects floppy via udev, reads presig shares
3. **Signing** (agent): Combine agent shard + cold shard presig to complete signature
4. **Reconciliation** (mother): Verify disk usage logs, detect anomalies, refill presigs

### Feature Flags

Mother crate optional features:
- `ledger`, `trezor`, `pkcs11` - Hardware wallet/HSM support
- `zkvm`, `zkvm-mock`, `zkvm-sp1` - zkVM proving support

FROST crate features:
- `taproot` - Bitcoin Taproot (BIP-340)
- `ed25519` - Solana, Cosmos, etc.
- `ristretto255` - Zcash shielded

### Signature Schemes

- **ECDSA (secp256k1)**: Ethereum, legacy Bitcoin, EVM chains
- **FROST Taproot**: Bitcoin Taproot
- **FROST Ed25519**: Solana, Cosmos, Near, Polkadot
- **FROST Ristretto255**: Zcash shielded transactions

## Commit Messages

Use conventional commit format: `<type>(<scope>): <description>`

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`

Example: `feat(daemon): add disk expiration warnings`
