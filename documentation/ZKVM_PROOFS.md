# zkVM Proof System Documentation

This document describes how Sigil uses zero-knowledge proofs to provide auditability and integrity guarantees for signing operations.

## 1. Overview

Every signature produced by Sigil is accompanied by a zero-knowledge proof that attests:

1. The signature was computed correctly from valid presignature shares
2. Both parties (cold and agent) contributed to the signature
3. The signature verifies against the claimed public key
4. No private key material is revealed in the proof

## 2. Proof System: SP1

Sigil uses [SP1](https://github.com/succinctlabs/sp1) by Succinct Labs, a zkVM (zero-knowledge virtual machine) that can prove arbitrary Rust/RISC-V computations.

### 2.1 Why SP1?

| Property | Benefit |
|----------|---------|
| Rust support | Write circuits in familiar language |
| RISC-V based | Standard ISA, auditable |
| Recursive proofs | Batch verification possible |
| Active development | Security updates |
| Audited | Third-party security audits |

### 2.2 Security Properties

- **Completeness**: Honest prover can always convince verifier
- **Soundness**: Malicious prover cannot forge proofs (with negligible probability)
- **Zero-knowledge**: Proof reveals nothing about private inputs

## 3. Circuit Specification

### 3.1 Public Inputs

```rust
struct PublicInputs {
    /// Compressed child public key (33 bytes)
    child_pubkey: [u8; 33],

    /// Message hash being signed (32 bytes)
    message_hash: [u8; 32],

    /// Index of the presignature used
    presig_index: u32,
}
```

### 3.2 Private Inputs (Witnesses)

```rust
struct PrivateInputs {
    /// Cold party's presignature share
    presig_cold: PresigShare {
        r_point: [u8; 33],
        k_share: [u8; 32],
        chi: [u8; 32],
    },

    /// Agent party's presignature share
    presig_agent: PresigShare {
        r_point: [u8; 33],
        k_share: [u8; 32],
        chi: [u8; 32],
    },
}
```

### 3.3 Public Outputs

```rust
struct PublicOutputs {
    /// The computed ECDSA signature (64 bytes)
    signature: [u8; 64],

    /// Presignature index (for logging)
    presig_index: u32,
}
```

### 3.4 Circuit Constraints

The SP1 program enforces:

```rust
// 1. R-point agreement (both parties committed to same R)
assert_eq!(presig_cold.r_point, presig_agent.r_point);

// 2. Reconstruct combined values
let k = k_cold + k_agent;  // Combined nonce
let chi = chi_cold + chi_agent;  // Combined private key

// 3. Compute signature
let r = x_coord(decompress(r_point)) mod n;
let s = k_inv * (z + r * chi) mod n;
let signature = (r, s);

// 4. Verify signature is valid
assert!(ecdsa_verify(child_pubkey, message_hash, signature));
```

## 4. Proof Generation

### 4.1 Process Flow

```
┌─────────────────┐     ┌─────────────────┐
│  Cold Share     │     │  Agent Share    │
│  (from disk)    │     │  (from store)   │
└────────┬────────┘     └────────┬────────┘
         │                       │
         └───────────┬───────────┘
                     │
                     ▼
           ┌─────────────────┐
           │   SP1 Prover    │
           │                 │
           │  Execute RISC-V │
           │  Generate trace │
           │  Produce proof  │
           └────────┬────────┘
                    │
                    ▼
         ┌──────────────────┐
         │  Proof + Output  │
         │  - signature     │
         │  - presig_index  │
         │  - proof_bytes   │
         └──────────────────┘
```

### 4.2 Code Example

```rust
use sp1_sdk::{ProverClient, SP1Stdin};

fn generate_signing_proof(
    child_pubkey: &[u8; 33],
    message_hash: &[u8; 32],
    presig_index: u32,
    presig_cold: &PresigColdShare,
    presig_agent: &PresigAgentShare,
) -> Result<(Signature, Vec<u8>)> {
    // Initialize prover
    let client = ProverClient::new();
    let (pk, vk) = client.setup(SIGIL_ELF);

    // Prepare inputs
    let mut stdin = SP1Stdin::new();
    stdin.write(child_pubkey);
    stdin.write(message_hash);
    stdin.write(&presig_index);
    stdin.write(presig_cold);
    stdin.write(presig_agent);

    // Generate proof
    let proof = client.prove(&pk, stdin)?;

    // Extract outputs
    let signature = proof.public_values.read::<[u8; 64]>();
    let proof_bytes = bincode::serialize(&proof)?;

    Ok((Signature::new(signature), proof_bytes))
}
```

## 5. Proof Verification

### 5.1 On-Chain Verification

SP1 proofs can be verified on-chain using Succinct's verifier contracts:

```solidity
// Ethereum verification
interface ISP1Verifier {
    function verifyProof(
        bytes32 programVKey,
        bytes calldata publicValues,
        bytes calldata proofBytes
    ) external view returns (bool);
}

contract SigilVerifier {
    ISP1Verifier public sp1Verifier;
    bytes32 public sigilProgramVKey;

    function verifySigningProof(
        bytes calldata publicValues,
        bytes calldata proof
    ) external view returns (bool) {
        return sp1Verifier.verifyProof(
            sigilProgramVKey,
            publicValues,
            proof
        );
    }
}
```

### 5.2 Off-Chain Verification

```rust
use sp1_sdk::ProverClient;

fn verify_signing_proof(
    proof_bytes: &[u8],
    expected_pubkey: &[u8; 33],
    expected_message: &[u8; 32],
) -> Result<bool> {
    let client = ProverClient::new();
    let (_, vk) = client.setup(SIGIL_ELF);

    let proof: SP1ProofWithPublicValues = bincode::deserialize(proof_bytes)?;

    // Verify the proof
    client.verify(&proof, &vk)?;

    // Check public inputs match expected
    let pubkey: [u8; 33] = proof.public_values.read();
    let message: [u8; 32] = proof.public_values.read();

    Ok(pubkey == *expected_pubkey && message == *expected_message)
}
```

## 6. Proof Storage

### 6.1 Storage Options

| Option | Pros | Cons |
|--------|------|------|
| On disk | Simple, local | Space limited (~1.1MB) |
| IPFS | Decentralized | Availability depends on pinning |
| Agent server | Fast access | Centralized |
| On-chain | Permanent, verifiable | Expensive |

### 6.2 Recommended Approach

1. **Hash on disk**: Store SHA-256 hash in usage log
2. **Full proof on IPFS**: Pin with redundant services
3. **Index on agent**: Map presig_index → IPFS CID

```rust
struct UsageLogEntry {
    // ...
    zkproof_hash: [u8; 32],  // SHA-256 of full proof
}

// Agent maintains:
struct ProofIndex {
    presig_index: u32,
    ipfs_cid: String,
    proof_size: u64,
}
```

## 7. Proof Batching

For high-frequency signing, proofs can be batched:

### 7.1 Recursive Aggregation

```rust
// Generate individual proofs
let proofs: Vec<SP1Proof> = signatures
    .iter()
    .map(|s| generate_proof(s))
    .collect();

// Aggregate into single proof
let aggregated = sp1_client.aggregate(&proofs)?;
```

### 7.2 Benefits

- Reduced verification cost (one proof vs. many)
- Smaller on-chain footprint
- Efficient audit trails

## 8. Security Considerations

### 8.1 Trusted Setup

SP1 uses a **transparent setup** (no trusted setup ceremony required):
- Based on hash functions and standard cryptographic assumptions
- No "toxic waste" to manage
- Anyone can verify the setup

### 8.2 Soundness Assumptions

- SHA-256 is collision-resistant
- FRI protocol assumptions hold
- RISC-V execution is deterministic

### 8.3 Potential Attacks

| Attack | Mitigation |
|--------|------------|
| Proof forgery | Cryptographic soundness |
| Replay | Unique presig_index per proof |
| Witness extraction | Zero-knowledge property |
| Implementation bugs | Use audited SP1 version |

## 9. Performance

### 9.1 Typical Metrics

| Metric | Value |
|--------|-------|
| Proof generation | ~10-30 seconds |
| Proof size | ~200-500 KB |
| Verification time | ~10 ms |
| On-chain gas | ~300-400K gas |

### 9.2 Optimization Options

- **Precompilation**: Ahead-of-time circuit compilation
- **GPU proving**: Parallel proof generation
- **Proof compression**: Groth16 wrapping for smaller proofs

## 10. Audit Trail Verification

### 10.1 Complete Verification Flow

```
1. Retrieve usage log entry from disk
2. Fetch full proof from IPFS using zkproof_hash
3. Verify proof against SP1 verification key
4. Check public outputs match log entry:
   - signature matches
   - presig_index matches
   - child_pubkey matches expected
5. Verify signature against blockchain state
```

### 10.2 Verification Script

```bash
#!/bin/bash
# verify_signing_trail.sh

CHILD_ID=$1
PRESIG_INDEX=$2

# Fetch proof from IPFS
PROOF=$(ipfs cat $(sigil-cli get-proof-cid $CHILD_ID $PRESIG_INDEX))

# Verify proof
sigil-cli verify-proof --proof "$PROOF" --child-id $CHILD_ID

# Check on-chain state
sigil-cli verify-signature --child-id $CHILD_ID --index $PRESIG_INDEX
```

## 11. Future Improvements

1. **Groth16 compression**: Wrap SP1 proofs in Groth16 for ~200 byte proofs
2. **Proof aggregation**: Batch multiple signatures into single proof
3. **Cross-chain verification**: Deploy verifiers on multiple chains
4. **Recursive proofs**: Prove correctness of entire signing history

## 12. References

- [SP1 Documentation](https://docs.succinct.xyz/)
- [SP1 Security Audit](https://github.com/succinctlabs/sp1/security)
- [RISC-V Specification](https://riscv.org/specifications/)
- [FRI Protocol](https://eccc.weizmann.ac.il/report/2017/134/)

## 13. Changelog

- **v0.1.0** (2026-01-17): Initial zkVM documentation
