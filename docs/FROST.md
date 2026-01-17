# Sigil FROST: Multi-Chain Threshold Signatures

**Version:** 0.1.0
**Status:** Draft for Review
**Last Updated:** 2026-01-17

## Overview

FROST (Flexible Round-Optimized Schnorr Threshold) extends Sigil's MPC-secured signing to support multiple blockchain ecosystems beyond Ethereum and legacy Bitcoin. This document describes Sigil's FROST implementation and provides quick start guides for each supported ecosystem.

## Supported Signature Schemes

| Scheme | Curve | Use Cases | Feature Flag |
|--------|-------|-----------|--------------|
| ECDSA | secp256k1 | Ethereum, Bitcoin (legacy), BSC, Polygon | Default |
| Taproot/Schnorr | secp256k1 | Bitcoin Taproot (BIP-340) | `taproot` |
| Ed25519 | Curve25519 | Solana, Cosmos, Near, Polkadot, Cardano | `ed25519` |
| Ristretto255 | Ristretto | Zcash (shielded transactions) | `ristretto255` |

## Architecture

### FROST vs ECDSA Presignatures

FROST uses Schnorr signatures, which have a simpler algebraic structure than ECDSA:

```
ECDSA: s = k⁻¹(z + rx) mod n    (requires modular inverse)
Schnorr: s = k + ex mod n        (simple addition)
```

This simplicity enables true threshold signatures without complex MPC protocols.

### Two-Round Protocol

```
Round 1 (Pre-processing):          Round 2 (Signing):
┌─────────────────────┐            ┌─────────────────────┐
│ Generate nonce k_i  │            │ Receive message m   │
│ Compute R_i = k_i·G │            │ Compute challenge e │
│ Share commitment    │            │ Compute s_i = k_i + │
│                     │            │   e·x_i             │
│ Store: (k_i, R_i)   │            │ Aggregate: s = Σs_i │
└─────────────────────┘            └─────────────────────┘
        ↓                                   ↓
  Offline (disk creation)           Online (signing)
```

### Presignature Storage Model

FROST presignatures follow the same model as ECDSA:

```
Mother Device (air-gapped)    Child Disk           Agent Server
────────────────────────────  ──────────────────   ──────────────
1. Generate key shares   ───► cold_key_share       agent_key_share
2. Generate nonces      ───► frost_nonces[N]       frost_commitments[N]
3. Sign disk header     ───► mother_signature
```

Each FROST presignature consists of:
- **Cold share**: Nonce hiding value, nonce binding value, secret share
- **Agent share**: Commitments (hiding/binding points), verifying share

## Quick Start Guides

---

### Bitcoin Taproot (BIP-340)

Bitcoin Taproot uses BIP-340 Schnorr signatures with x-only public keys.

#### Key Characteristics
- 32-byte x-only public keys (no prefix byte)
- 64-byte signatures (R || s)
- Tagged hashing for domain separation
- Native SegWit v1 addresses (bc1p...)

#### Creating a Taproot Child Disk

```bash
# On mother device (air-gapped)
sigil ceremony create-child \
  --scheme taproot \
  --presigs 1000 \
  --label "btc-taproot-001"

# Outputs:
# - Child disk with 1000 FROST presignatures
# - Agent share file for transfer
```

#### Deriving Taproot Address

```rust
use sigil_frost::taproot::TaprootFrost;
use sigil_frost::FrostKeyGen;

// Generate 2-of-2 key shares
let (cold_share, agent_share, verifying_key) = TaprootFrost::generate_2of2()?;

// Get the x-only public key (32 bytes)
let xonly_pubkey = &verifying_key.data;

// Derive bech32m address
let address = bitcoin::Address::p2tr_tweaked(
    bitcoin::XOnlyPublicKey::from_slice(xonly_pubkey)?,
    bitcoin::Network::Bitcoin,
);
println!("Taproot address: {}", address);  // bc1p...
```

#### Signing a Bitcoin Transaction

```rust
use sigil_frost::taproot::{TaprootFrost, TaprootSigningContext};
use sigil_frost::{FrostSigner, FrostPresigGen};

// Load presignature from disk
let (cold_presig, agent_presig) = load_presig_pair(disk, agent_store, index)?;

// Create sighash (BIP-340 tagged hash)
let sighash = transaction.signature_hash(input_index, &prevout, SighashType::Default)?;

// Generate signature shares
let cold_sig_share = TaprootFrost::sign_with_presig(
    &cold_share,
    &cold_presig,
    &sighash,
)?;

let agent_sig_share = TaprootFrost::sign_with_presig(
    &agent_share,
    &agent_presig,
    &sighash,
)?;

// Aggregate into final signature
let mut ctx = TaprootSigningContext::new(verifying_key.clone(), sighash.to_vec())?;
ctx.add_signature_share(1, cold_sig_share)?;
ctx.add_signature_share(2, agent_sig_share)?;
let signature = ctx.aggregate()?;  // 64 bytes

// Verify before broadcast
TaprootFrost::verify(&verifying_key, &sighash, &signature)?;
```

#### Transaction Broadcast

```bash
# Sign and broadcast via CLI
sigil sign --disk /dev/sda \
  --scheme taproot \
  --tx-hex "02000000..." \
  --broadcast bitcoin-mainnet
```

---

### Solana (Ed25519)

Solana uses Ed25519 signatures with base58-encoded addresses.

#### Key Characteristics
- 32-byte public keys (base58 encoded)
- 64-byte signatures
- No transaction malleability (deterministic signatures)
- Program-derived addresses (PDAs) for smart contracts

#### Creating a Solana Child Disk

```bash
# On mother device (air-gapped)
sigil ceremony create-child \
  --scheme ed25519 \
  --presigs 5000 \
  --label "solana-wallet-001"
```

#### Deriving Solana Address

```rust
use sigil_frost::ed25519::{Ed25519Frost, solana};
use sigil_frost::FrostKeyGen;

// Generate 2-of-2 key shares
let (cold_share, agent_share, verifying_key) = Ed25519Frost::generate_2of2()?;

// Convert to Solana base58 address
let solana_pubkey = solana::to_solana_pubkey(&verifying_key);
println!("Solana address: {}", solana_pubkey);  // e.g., 7EcD...

// For SPL token accounts, derive ATA
let ata = solana::derive_ata(&solana_pubkey, &token_mint)?;
```

#### Signing a Solana Transaction

```rust
use sigil_frost::ed25519::{Ed25519Frost, Ed25519SigningContext};
use sigil_frost::FrostSigner;

// Serialize transaction message
let message_bytes = transaction.message.serialize();

// Generate signature shares
let cold_sig_share = Ed25519Frost::sign_with_presig(
    &cold_share,
    &cold_presig,
    &message_bytes,
)?;

let agent_sig_share = Ed25519Frost::sign_with_presig(
    &agent_share,
    &agent_presig,
    &message_bytes,
)?;

// Aggregate
let mut ctx = Ed25519SigningContext::new(verifying_key.clone(), message_bytes)?;
ctx.add_signature_share(1, cold_sig_share)?;
ctx.add_signature_share(2, agent_sig_share)?;
let signature = ctx.aggregate()?;

// Attach to transaction
transaction.signatures[0] = Signature::new(&signature.data);
```

#### Transaction Broadcast

```bash
# Sign and broadcast via CLI
sigil sign --disk /dev/sda \
  --scheme ed25519 \
  --tx-base64 "AQAAAA..." \
  --broadcast solana-mainnet
```

---

### Cosmos Ecosystem (Ed25519)

Cosmos chains support Ed25519 with Amino or Protobuf encoding.

#### Key Characteristics
- Same Ed25519 signatures as Solana
- Bech32 addresses with chain-specific prefixes (cosmos1..., osmo1...)
- SIGN_MODE_DIRECT for modern signing

#### Creating a Cosmos Child Disk

```bash
sigil ceremony create-child \
  --scheme ed25519 \
  --presigs 2000 \
  --label "cosmos-hub-001"
```

#### Deriving Cosmos Address

```rust
use sigil_frost::ed25519::Ed25519Frost;
use sha2::{Sha256, Digest};
use bech32::{ToBase32, Variant};

let (_, _, verifying_key) = Ed25519Frost::generate_2of2()?;

// Cosmos address = ripemd160(sha256(pubkey))
let sha256_hash = Sha256::digest(&verifying_key.data);
let ripemd_hash = ripemd160::Ripemd160::digest(&sha256_hash);

let address = bech32::encode("cosmos", ripemd_hash.to_base32(), Variant::Bech32)?;
println!("Cosmos address: {}", address);  // cosmos1...
```

#### Signing Cosmos Transactions

```rust
// Create sign doc bytes (SIGN_MODE_DIRECT)
let sign_bytes = sign_doc.to_bytes()?;

// Sign with FROST (same as Solana)
let signature = frost_sign_ed25519(&cold_share, &agent_share, &sign_bytes)?;

// Attach to TxRaw
tx_raw.signatures.push(signature.data);
```

---

### Zcash Shielded (Ristretto255)

Zcash shielded transactions use Ristretto255 for Sapling/Orchard.

#### Key Characteristics
- Privacy-preserving transactions (sender, receiver, amount hidden)
- PCZT (Partially Created Zcash Transactions) for threshold signing
- Viewing keys for audit without spending capability
- Note commitments and nullifiers

#### Creating a Zcash Child Disk

```bash
sigil ceremony create-child \
  --scheme ristretto255 \
  --presigs 500 \
  --label "zcash-shielded-001"
```

#### Understanding PCZT Integration

```
Standard Zcash Flow:
  1. Create transaction → 2. Sign → 3. Broadcast

PCZT + Sigil Flow:
  1. Create PCZT (partial tx)
  2. Extract signing data
  3. Sign with FROST (air-gapped)
  4. Insert signature into PCZT
  5. Finalize and broadcast
```

#### Signing Workflow

```rust
use sigil_frost::ristretto255::{Ristretto255Frost, zcash};
use sigil_frost::FrostSigner;
use pczt::Pczt;

// 1. Load PCZT from wallet software
let pczt = Pczt::parse(&pczt_bytes)?;

// 2. Extract the spend authorization signature message
let sighash = pczt.extract_sighash(spend_index)?;

// 3. Generate FROST signature shares
let cold_sig_share = Ristretto255Frost::sign_with_presig(
    &cold_share,
    &cold_presig,
    &sighash,
)?;

let agent_sig_share = Ristretto255Frost::sign_with_presig(
    &agent_share,
    &agent_presig,
    &sighash,
)?;

// 4. Aggregate signature
let signature = frost_aggregate(cold_sig_share, agent_sig_share)?;

// 5. Insert back into PCZT
pczt.set_spend_signature(spend_index, signature)?;

// 6. Finalize
let tx = pczt.finalize()?;
```

#### Privacy Considerations

- **Viewing Keys**: Generate viewing keys to audit without spending
- **Diversified Addresses**: Support multiple addresses from one key
- **Turnstile**: Value cannot cross pools without re-shielding

```rust
// Generate viewing key for auditors
let full_viewing_key = zcash::derive_fvk(&verifying_key)?;
let incoming_viewing_key = zcash::derive_ivk(&full_viewing_key)?;

// Auditor can see incoming transactions but cannot spend
```

---

## Security Considerations

### Nonce Security

FROST nonces are critical for security:

```
WARNING: Nonce reuse across two messages reveals the private key!

  s₁ = k + e₁·x  (message 1)
  s₂ = k + e₂·x  (message 2)

  x = (s₁ - s₂) / (e₁ - e₂)  (attacker recovers key)
```

Sigil prevents nonce reuse through:
1. **Single-use presignatures**: Each nonce is generated once and consumed
2. **Status tracking**: Cold share status byte (Fresh→Used→Void)
3. **Agent index tracking**: Agent maintains authoritative used-index list
4. **Reconciliation**: Mother device detects anomalies during reconciliation

### Key Share Security

| Share Location | Protection | Threat Mitigation |
|---------------|------------|-------------------|
| Cold share (disk) | Physical possession | Requires agent share to sign |
| Agent share (server) | Encrypted storage | Requires cold share to sign |
| Verifying key | Public | No protection needed |

### Cross-Chain Considerations

When using the same key for multiple chains:

1. **Address Derivation**: Different chains derive addresses differently
2. **Replay Protection**: Sign chain-specific data (chain ID, genesis hash)
3. **Presig Allocation**: Reserve presigs per chain to prevent exhaustion

```rust
// Recommended: Use different child disks per ecosystem
let btc_disk = create_child(Taproot, 1000, "btc-001")?;
let sol_disk = create_child(Ed25519, 5000, "sol-001")?;
let zec_disk = create_child(Ristretto255, 500, "zec-001")?;
```

## Comparison: ECDSA vs FROST

| Aspect | ECDSA Presigs | FROST |
|--------|---------------|-------|
| Signature scheme | ECDSA | Schnorr |
| MPC complexity | High (modular inverse) | Low (linear) |
| Signature size | 64-71 bytes (DER) | 64 bytes |
| Batch verification | No | Yes (3x faster) |
| Adaptor signatures | Complex | Native |
| Supported chains | Ethereum, legacy BTC | Taproot, Solana, Zcash |

## Troubleshooting

### "No presignatures remaining"

```bash
# Check remaining presigs
sigil disk info /dev/sda

# Request more presigs from mother device
sigil ceremony replenish --disk /dev/sda --count 500
```

### "Signature verification failed"

1. Ensure correct scheme (`--scheme taproot` vs `--scheme ed25519`)
2. Verify message encoding (raw bytes vs tagged hash)
3. Check presig index synchronization between cold/agent shares

### "Nonce reuse detected"

This is a critical security event:

```bash
# Immediately nullify the affected child
sigil ceremony nullify --child-id abc123

# Investigate usage logs
sigil disk audit /dev/sda
```

## API Reference

See `sigil-frost` crate documentation:

```bash
cargo doc --package sigil-frost --open
```

Key traits:
- `FrostKeyGen`: Key generation for each scheme
- `FrostPresigGen`: Presignature generation
- `FrostSigner`: Signing operations
- `FrostCipherSuite`: Scheme-specific parameters

## Changelog

- **v0.1.0** (2026-01-17): Initial FROST implementation
  - Taproot (BIP-340) support
  - Ed25519 (Solana/Cosmos) support
  - Ristretto255 (Zcash shielded) support
