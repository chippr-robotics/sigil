# Sigil Cryptographic Specification

**Version:** 0.1.0
**Status:** Draft for Review
**Last Updated:** 2026-01-17

## Abstract

This document specifies the cryptographic protocols used in the Sigil MPC-secured floppy disk signing system. Sigil implements a 2-of-2 threshold ECDSA signing scheme using presignatures, designed to provide physical containment for agentic signing operations.

## 1. Notation

| Symbol | Description |
|--------|-------------|
| $G$ | Generator point of secp256k1 |
| $n$ | Order of secp256k1 ($\approx 2^{256}$) |
| $\mathbb{F}_n$ | Scalar field modulo $n$ |
| $H(\cdot)$ | SHA-256 hash function |
| $\parallel$ | Concatenation |
| $[x]$ | Elliptic curve scalar multiplication: $x \cdot G$ |
| $P_1, P_2$ | Cold party (disk), Agent party |

## 2. System Overview

### 2.1 Parties

1. **Mother Device** (air-gapped): Holds master cold shard, generates child shards and presignatures
2. **Cold Party** ($P_1$): Represented by the floppy disk containing presignature shares
3. **Agent Party** ($P_2$): Server/daemon holding agent presignature shares

### 2.2 Key Hierarchy

```
Master Level:
  cold_master_shard (sk₁) ←── Mother Device
  agent_master_shard (sk₂) ←── Agent Device
  master_pubkey = [sk₁ + sk₂]

Child Level (derived):
  child_cold_shard[i] = HD(sk₁, path_i)
  child_agent_shard[i] = HD(sk₂, path_i)
  child_pubkey[i] = [child_cold_shard[i] + child_agent_shard[i]]
```

## 3. Key Generation

### 3.1 Master Key Generation

Performed once during mother device initialization.

```
MASTER_KEYGEN():
  1. Sample sk₁ ←$← 𝔽ₙ  (cold master shard)
  2. Sample sk₂ ←$← 𝔽ₙ  (agent master shard)
  3. Compute PK = [sk₁] + [sk₂] = [sk₁ + sk₂]
  4. Return (sk₁, sk₂, PK)
```

**Security Requirement:** sk₁ and sk₂ must be generated with cryptographically secure randomness (256 bits of entropy minimum).

### 3.2 Child Key Derivation

Uses SLIP-10 (BIP32 for secp256k1) for hierarchical deterministic derivation.

```
CHILD_DERIVE(master_shard, path):
  1. Parse path as [purpose'/coin'/account'/index']
  2. Apply SLIP-10 derivation: child_shard = SLIP10(master_shard, path)
  3. Return child_shard
```

**Path Format:** `m/44'/60'/0'/i'` for Ethereum-compatible chains.

### 3.3 Child Public Key Computation

```
CHILD_PUBKEY(cold_child_shard, agent_child_shard):
  1. P₁ = [cold_child_shard]
  2. P₂ = [agent_child_shard]
  3. child_pubkey = P₁ + P₂
  4. Return compress(child_pubkey)
```

## 4. Presignature Generation

### 4.1 Overview

Presignatures enable non-interactive signing by pre-computing the nonce commitment. Each presignature can only be used once.

### 4.2 Presig Generation Protocol

Executed by the mother device for each presignature:

```
PRESIG_GEN(cold_child_shard, agent_child_shard):
  1. Sample k₁ ←$← 𝔽ₙ  (cold nonce share)
  2. Sample k₂ ←$← 𝔽ₙ  (agent nonce share)
  3. Compute R = [k₁ + k₂]
  4. Compute r = x_coord(R) mod n
  5. If r = 0, restart from step 1

  6. Cold share:
     - R_point = compress(R)
     - k_cold = k₁
     - χ_cold = cold_child_shard

  7. Agent share:
     - R_point = compress(R)  (must match)
     - k_agent = k₂
     - χ_agent = agent_child_shard

  8. Return (ColdShare, AgentShare)
```

### 4.3 Presignature Structure

**Cold Share (256 bytes on disk):**
```
struct PresigColdShare {
    r_point: [u8; 33],    // Compressed R point
    k_cold: [u8; 32],     // Cold nonce share
    chi_cold: [u8; 32],   // = cold_child_shard
    status: u8,           // 0=fresh, 1=used, 2=void
    reserved: [u8; 158],
}
```

**Agent Share:**
```
struct PresigAgentShare {
    r_point: [u8; 33],    // Must match cold share
    k_agent: [u8; 32],    // Agent nonce share
    chi_agent: [u8; 32],  // = agent_child_shard
}
```

## 5. Signature Generation

### 5.1 Signing Protocol

Given message hash $m \in \{0,1\}^{256}$:

```
SIGN(m, cold_share, agent_share):
  1. Verify cold_share.R_point = agent_share.R_point
  2. Decompress R = decompress(R_point)
  3. r = x_coord(R) mod n
  4. If r = 0, abort (invalid presig)

  5. Reconstruct nonce: k = k_cold + k_agent mod n
  6. If k = 0, abort (invalid presig)

  7. Reconstruct private key contribution:
     χ = χ_cold + χ_agent mod n

  8. Compute z = m mod n (message as scalar)

  9. Compute s = k⁻¹ · (z + r · χ) mod n

  10. Normalize to low-S (BIP-62):
      If s > n/2: s = n - s

  11. Return σ = (r, s)
```

### 5.2 Signature Verification

Standard ECDSA verification against child_pubkey:

```
VERIFY(m, σ, PK):
  1. Parse σ = (r, s)
  2. Verify 1 ≤ r, s < n
  3. Compute z = m mod n
  4. Compute u₁ = z · s⁻¹ mod n
  5. Compute u₂ = r · s⁻¹ mod n
  6. Compute R' = [u₁] + u₂ · PK
  7. Verify r = x_coord(R') mod n
```

## 6. zkVM Proving

### 6.1 Purpose

All signing operations execute inside SP1 zkVM to produce proofs that:
1. The signature was computed correctly from valid presig shares
2. Both parties contributed to the signature
3. The signature verifies against the claimed public key

### 6.2 Public Inputs

```
struct PublicInputs {
    child_pubkey: [u8; 33],
    message_hash: [u8; 32],
    presig_index: u32,
}
```

### 6.3 Private Inputs

```
struct PrivateInputs {
    presig_cold: PresigColdShare,
    presig_agent: PresigAgentShare,
}
```

### 6.4 Circuit Constraints

The zkVM program enforces:

1. **R-point Agreement:** `presig_cold.r_point == presig_agent.r_point`
2. **Valid Signature:** `VERIFY(message_hash, signature, child_pubkey) == true`
3. **Correct Computation:** Signature computed per Section 5.1

### 6.5 Public Outputs

```
struct PublicOutputs {
    signature: [u8; 64],
    presig_index: u32,
}
```

## 7. Security Properties

### 7.1 Unforgeability

**Claim:** An adversary controlling only one party (cold or agent) cannot forge signatures.

**Argument:**
- Without both nonce shares ($k_1$, $k_2$), the adversary cannot compute $k = k_1 + k_2$
- Without $k$, computing a valid $s = k^{-1}(z + rx)$ requires solving ECDLP
- The security reduces to the unforgeability of ECDSA under the hardness of ECDLP

### 7.2 Bounded Exposure

**Claim:** Compromise of a floppy disk exposes at most $N$ signing capabilities.

**Argument:**
- Each presignature can only produce one valid signature
- Once used, the nonce is consumed and the $(r, s)$ pair is fixed
- The disk contains exactly $N$ presignatures
- An attacker with the disk but without agent shares gets nothing useful

### 7.3 Forward Secrecy

**Claim:** Past signatures remain unforgeable even if the disk is later compromised.

**Argument:**
- Signatures already produced used presignatures that are now marked "used"
- The attacker cannot reuse those presignatures (R-point reuse is detectable)
- The attacker cannot correlate used presignatures to past signatures without additional information

### 7.4 Auditability

**Claim:** All signing operations are auditable via zkVM proofs.

**Argument:**
- Every signature is accompanied by a zkVM proof
- The proof attests to correct computation
- Proofs can be verified independently
- The usage log on the disk records all operations

## 8. Threat Mitigations

### 8.1 Nonce Reuse Prevention

**Threat:** Reusing a nonce $(k)$ for two different messages leaks the private key.

**Mitigation:**
- Each presignature has a unique index
- Status field prevents reuse (Fresh → Used transition is one-way)
- Mother device generates fresh nonces for each presignature
- Agent tracks used indices

### 8.2 Side-Channel Resistance

**Threat:** Timing or power analysis during signing.

**Mitigation:**
- Use constant-time scalar operations (k256 crate with `arithmetic` feature)
- Avoid branching on secret values
- Signature computation happens inside zkVM (abstracted execution)

### 8.3 Disk Cloning

**Threat:** Attacker clones disk and attempts parallel usage.

**Mitigation:**
- Reconciliation detects usage log anomalies
- Presig index gaps indicate potential cloning
- Mother maintains authoritative state

### 8.4 Replay Attacks

**Threat:** Attacker replays a valid signature for unintended transactions.

**Mitigation:**
- Signatures are over transaction hashes (unique per tx)
- Chain-specific replay protection (EIP-155 chain ID)
- Usage log records intended purpose

## 9. Cryptographic Assumptions

The security of Sigil relies on:

1. **ECDLP Hardness:** Discrete logarithm problem is hard on secp256k1
2. **Random Oracle Model:** SHA-256 behaves as a random oracle
3. **Secure Randomness:** System RNG provides 256 bits of entropy
4. **zkVM Soundness:** SP1 prover is computationally sound

## 10. Implementation Notes

### 10.1 Curve Parameters (secp256k1)

```
p = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFC2F
n = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141
G = (0x79BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798,
     0x483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8)
```

### 10.2 Hash Functions

- **Key Derivation:** HMAC-SHA512 (SLIP-10)
- **Message Hashing:** SHA-256 (for ECDSA)
- **Child ID:** SHA-256(compressed_pubkey)

### 10.3 Encoding

- **Scalars:** 32 bytes, big-endian
- **Points:** SEC1 compressed (33 bytes, prefix 0x02 or 0x03)
- **Signatures:** (r, s) concatenated, 64 bytes total

## 11. Test Vectors

### 11.1 Key Generation

```
cold_master_shard = 0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
agent_master_shard = 0xfedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210

Expected master_pubkey (uncompressed):
  [To be computed with reference implementation]
```

### 11.2 Presignature Generation

```
Test case pending reference implementation validation.
```

### 11.3 Signature Generation

```
Test case pending reference implementation validation.
```

## 12. References

1. Gennaro, R., & Goldfeder, S. (2020). One Round Threshold ECDSA with Identifiable Abort. IACR ePrint 2020/540.
2. Lindell, Y. (2017). Fast Secure Two-Party ECDSA Signing. CRYPTO 2017.
3. BIP-32: Hierarchical Deterministic Wallets
4. SLIP-10: Universal private key derivation from master private key
5. BIP-62: Dealing with malleability
6. EIP-155: Simple replay attack protection

## 13. Changelog

- **v0.1.0** (2026-01-17): Initial draft specification
