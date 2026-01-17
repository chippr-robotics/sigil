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

## Distributed Key Generation (DKG)

Sigil supports two key generation modes:

| Mode | Trust Model | Setup Complexity | Use Case |
|------|-------------|------------------|----------|
| **Trusted Dealer** | Mother sees full key during generation | Simple (one device) | Quick setup, air-gapped mother |
| **DKG** | Neither party sees full key | Interactive (QR codes) | Maximum security |

### DKG Overview

FROST DKG is a 2-round protocol based on Pedersen's DKG:

```
┌─────────────────────────────────────────────────────────────────┐
│                        FROST DKG Ceremony                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ROUND 1: Commitment Generation                                │
│                                                                 │
│   Mother Device                      Agent Device               │
│   ┌─────────────┐                    ┌─────────────┐           │
│   │ Generate    │                    │ Generate    │           │
│   │ secret poly │                    │ secret poly │           │
│   │ f₁(x)       │                    │ f₂(x)       │           │
│   │             │                    │             │           │
│   │ Commitments │                    │ Commitments │           │
│   │ C₁ = [a₁₀]  │                    │ C₂ = [a₂₀]  │           │
│   └──────┬──────┘                    └──────┬──────┘           │
│          │                                  │                   │
│          │◄────── Exchange via QR ─────────►│                   │
│                                                                 │
│   ROUND 2: Share Distribution                                   │
│                                                                 │
│   Mother Device                      Agent Device               │
│   ┌─────────────┐                    ┌─────────────┐           │
│   │ Compute     │                    │ Compute     │           │
│   │ f₁(2) share │─── QR ───────────► │ Verify      │           │
│   │ for Agent   │                    │ against C₁  │           │
│   │             │                    │             │           │
│   │ Verify      │ ◄─────── QR ───────│ f₂(1) share │           │
│   │ against C₂  │                    │ for Mother  │           │
│   └─────────────┘                    └─────────────┘           │
│                                                                 │
│   FINALIZATION: Both compute same group public key              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### DKG Ceremony Guide

#### Prerequisites

- **Mother Device**: Air-gapped, has camera + display
- **Agent Device**: Network-connected, has camera + display
- **Communication**: QR codes (offline-safe, auditable)

#### Step 1: Initialize Ceremonies

**On Mother Device:**
```bash
sigil ceremony dkg-init --role mother --scheme taproot

# Output:
# Participant ID: 1 (Mother)
# Scheme: Taproot (secp256k1)
# Threshold: 2-of-2
#
# [QR Code: Round 1 Package]
#
# Scan Agent's Round 1 QR to continue...
```

**On Agent Device:**
```bash
sigil ceremony dkg-init --role agent --scheme taproot

# Output:
# Participant ID: 2 (Agent)
# Scheme: Taproot (secp256k1)
# Threshold: 2-of-2
#
# [QR Code: Round 1 Package]
#
# Scan Mother's Round 1 QR to continue...
```

#### Step 2: Exchange Round 1

1. Mother displays QR → Agent scans
2. Agent displays QR → Mother scans

Both devices verify the other's commitments.

#### Step 3: Exchange Round 2

After scanning Round 1, both devices generate Round 2 packages:

1. Mother displays Round 2 QR → Agent scans
2. Agent displays Round 2 QR → Mother scans

Each device verifies the received share against the Round 1 commitments.

#### Step 4: Finalize

Both devices compute their final key shares and the group public key:

```
═══════════════════════════════════════════════════════════════
                    DKG CEREMONY RESULTS
═══════════════════════════════════════════════════════════════

Group Public Key: 02a4b3c2d1e0f9...  (33 bytes, compressed)

Your Key Share:   [HIDDEN - stored securely]
  - Share index:  1
  - Threshold:    2-of-2

Verification Hash (both devices should match):
  SHA256(GroupPubKey): 7f3a9c2b...

═══════════════════════════════════════════════════════════════

IMPORTANT: Neither device has ever seen the full private key.
           The key only exists as the sum of both shares.
```

### Programmatic DKG

```rust
use sigil_frost::dkg::{DkgCeremony, DkgConfig};
use sigil_frost::dkg::taproot::TaprootDkg;
use sigil_frost::SignatureScheme;

// Mother device
let config1 = DkgConfig::mother_2of2(SignatureScheme::Taproot);
let mut ceremony1: DkgCeremony<TaprootDkg> = DkgCeremony::new(config1)?;

// Agent device
let config2 = DkgConfig::agent_2of2(SignatureScheme::Taproot);
let mut ceremony2: DkgCeremony<TaprootDkg> = DkgCeremony::new(config2)?;

// Round 1: Generate and exchange
let r1_mother = ceremony1.generate_round1()?;
let r1_agent = ceremony2.generate_round1()?;

ceremony1.add_round1(r1_agent)?;
ceremony2.add_round1(r1_mother)?;

// Round 2: Generate and exchange
let r2_mother = ceremony1.generate_round2()?;
let r2_agent = ceremony2.generate_round2()?;

// Add packages (in 2-of-2, each generates one package for the other)
for pkg in r2_agent {
    ceremony1.add_round2(pkg)?;
}
for pkg in r2_mother {
    ceremony2.add_round2(pkg)?;
}

// Finalize
let output1 = ceremony1.finalize()?;
let output2 = ceremony2.finalize()?;

// Both have the same group public key
assert_eq!(output1.verifying_key.data, output2.verifying_key.data);
assert_eq!(output1.verification_hash, output2.verification_hash);

// But different key shares
println!("Mother key share ID: {}", output1.key_share.identifier);  // 1
println!("Agent key share ID: {}", output2.key_share.identifier);   // 2
```

### QR Code API

For air-gapped communication, use the QR encoding/decoding API:

```rust
use sigil_frost::dkg::{DkgQrEncoder, DkgQrDecoder};

// Encoding (display on screen)
let qr_package = DkgQrEncoder::encode_round1(&round1_package)?;

// For terminal display
let ascii_qr = DkgQrEncoder::to_ascii(qr_package.single().unwrap())?;
println!("{}", ascii_qr);

// Or generate PNG for display
let png_bytes = DkgQrEncoder::to_png(qr_package.single().unwrap(), 400)?;

// Decoding (from camera scan)
let mut decoder = DkgQrDecoder::new();
let complete = decoder.add_chunk(&scanned_data)?;
if complete {
    let package = decoder.decode_round1()?;
}
```

### DKG vs Trusted Dealer

| Aspect | Trusted Dealer | DKG |
|--------|---------------|-----|
| Setup complexity | Single device | 2 devices, 4 QR scans |
| Air-gap friendly | Excellent | Good (QR-based) |
| Trust assumption | Mother is honest | Neither trusted |
| Key exposure risk | Mother saw full key | Never combined |
| Ceremony time | ~30 seconds | ~5 minutes |
| Audit proof | Trust mother's logs | Cryptographic transcript |
| Recovery | Mother can regenerate | Requires both parties |

### When to Use DKG

Use DKG when:
- Maximum security is required
- Regulatory compliance demands proof that no single party held the full key
- Multiple independent organizations are participating
- Audit trail of key generation is important

Use Trusted Dealer when:
- Quick setup is needed
- Mother device is already trusted
- Simplicity is preferred
- Recovery flexibility is important

## Changelog

- **v0.2.0** (2026-01-17): Added DKG support
  - FROST DKG for 2-of-2 ceremonies
  - QR code encoding/decoding for air-gapped communication
  - Support for all three cipher suites (Taproot, Ed25519, Ristretto255)
- **v0.1.0** (2026-01-17): Initial FROST implementation
  - Taproot (BIP-340) support
  - Ed25519 (Solana/Cosmos) support
  - Ristretto255 (Zcash shielded) support
