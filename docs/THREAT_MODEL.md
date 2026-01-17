# Sigil Threat Model

**Version:** 0.1.0
**Status:** Draft for Review
**Last Updated:** 2026-01-17

## 1. Executive Summary

This document provides a comprehensive threat model for the Sigil MPC-secured floppy disk signing system. We analyze adversaries, attack surfaces, threats, and mitigations using a defense-in-depth approach.

## 2. System Assets

### 2.1 Critical Assets (Compromise = Total Loss)

| Asset | Location | Description |
|-------|----------|-------------|
| Master Cold Shard | Mother device | Root of cold key hierarchy |
| Master Agent Shard | Agent server | Root of agent key hierarchy |
| Child Cold Shard | Embedded in presigs | Per-child signing capability |
| Child Agent Shard | Agent store | Per-child signing capability |

### 2.2 High-Value Assets (Compromise = Bounded Loss)

| Asset | Location | Description |
|-------|----------|-------------|
| Presignature Cold Shares | Floppy disk | N signing operations |
| Presignature Agent Shares | Agent store | N signing operations |
| Usage Log | Floppy disk | Audit trail |
| Mother Signature | Disk header | Authenticity proof |

### 2.3 Operational Assets

| Asset | Location | Description |
|-------|----------|-------------|
| Child Registry | Mother device | Track all children |
| zkVM Proofs | Agent/IPFS | Signing attestations |
| IPC Socket | Agent server | Control channel |

## 3. Adversary Models

### 3.1 ADV-1: External Network Attacker

**Capabilities:**
- Network traffic interception
- Malicious RPC responses
- DNS hijacking

**Limitations:**
- No physical access
- No code execution on agent

**Goal:** Steal funds, cause unauthorized transactions

### 3.2 ADV-2: Compromised Agent

**Capabilities:**
- Full control of agent daemon
- Access to agent shard and presig shares
- Can modify signing requests

**Limitations:**
- No physical access to floppy disk
- Cannot forge mother signatures

**Goal:** Sign unauthorized transactions, exfiltrate keys

### 3.3 ADV-3: Physical Disk Thief

**Capabilities:**
- Possesses stolen floppy disk
- Can read/write disk contents
- May have cloned disk

**Limitations:**
- No agent shard access
- Cannot connect to agent daemon

**Goal:** Sign unauthorized transactions, clone disk for later use

### 3.4 ADV-4: Compromised Mother Device

**Capabilities:**
- Full access to master cold shard
- Can generate malicious presigs
- Can modify child registry

**Limitations:**
- Air-gapped (no network)
- Cannot distribute malicious presigs without physical transfer

**Goal:** Backdoor all future children, forge audit trails

### 3.5 ADV-5: Supply Chain Attacker

**Capabilities:**
- Modify source code or dependencies
- Insert backdoors in builds
- Compromise development machines

**Limitations:**
- Detectable via code review/audit
- Requires persistent access

**Goal:** Insert cryptographic backdoors

### 3.6 ADV-6: Side-Channel Attacker

**Capabilities:**
- Timing measurements
- Power analysis
- Electromagnetic emanations
- Cache attacks

**Limitations:**
- Requires physical proximity or co-location
- Statistical analysis needed

**Goal:** Extract secret keys through side channels

## 4. Attack Surface Analysis

### 4.1 Physical Attack Surface

| Component | Exposure | Risk Level |
|-----------|----------|------------|
| Floppy disk | User carries it | HIGH |
| Mother device | Air-gapped room | LOW |
| Agent server | Data center | MEDIUM |
| USB interface | User laptop | MEDIUM |

### 4.2 Network Attack Surface

| Component | Exposure | Risk Level |
|-----------|----------|------------|
| IPC socket | Local only | LOW |
| Blockchain RPC | Internet | MEDIUM |
| No external APIs | N/A | LOW |

### 4.3 Software Attack Surface

| Component | Complexity | Risk Level |
|-----------|------------|------------|
| Disk parsing | Medium | HIGH |
| Cryptographic ops | High | CRITICAL |
| zkVM execution | High | HIGH |
| Serialization | Medium | MEDIUM |

## 5. Threat Catalog

### T1: Unauthorized Signing

**Description:** Adversary signs transactions without legitimate authorization.

**Attack Scenarios:**
- T1.1: Agent compromise + disk theft
- T1.2: Replay of previous signing session
- T1.3: Presig share theft from both parties

**Likelihood:** Medium
**Impact:** Critical (financial loss)

**Mitigations:**
- M1.1: 2-of-2 requirement makes single-party compromise insufficient
- M1.2: Presigs are single-use; nonce reuse is detectable
- M1.3: Physical separation of shards

### T2: Key Extraction

**Description:** Adversary extracts long-term private keys.

**Attack Scenarios:**
- T2.1: Nonce reuse attack (sign same R with different messages)
- T2.2: Side-channel attack during signing
- T2.3: Memory dumping of active process

**Likelihood:** Low
**Impact:** Critical (total compromise)

**Mitigations:**
- M2.1: Each presig has unique R; reuse is prevented by status tracking
- M2.2: Constant-time operations; signing in zkVM
- M2.3: Zeroization of sensitive memory after use

### T3: Disk Cloning Attack

**Description:** Adversary clones disk and uses copies in parallel.

**Attack Scenarios:**
- T3.1: Clone before first use, use both copies
- T3.2: Clone after partial use, divergent usage

**Likelihood:** Medium
**Impact:** High (bounded by N presigs)

**Mitigations:**
- M3.1: Reconciliation detects divergent usage logs
- M3.2: Agent tracks used presig indices
- M3.3: Nullification on anomaly detection

### T4: Denial of Service

**Description:** Adversary prevents legitimate signing operations.

**Attack Scenarios:**
- T4.1: Destroy/steal floppy disk
- T4.2: Exhaust presignatures maliciously
- T4.3: Corrupt disk data

**Likelihood:** Medium
**Impact:** Medium (operational disruption)

**Mitigations:**
- M4.1: Backup presigs (requires reconciliation)
- M4.2: Rate limiting; usage monitoring
- M4.3: Disk validation before use

### T5: Mother Device Compromise

**Description:** Adversary gains access to air-gapped mother device.

**Attack Scenarios:**
- T5.1: Physical break-in
- T5.2: Insider threat
- T5.3: Supply chain compromise of mother software

**Likelihood:** Low
**Impact:** Critical (root of trust)

**Mitigations:**
- M5.1: Physical security controls
- M5.2: Multi-person ceremony requirement
- M5.3: Reproducible builds; code audit

### T6: Agent Process Compromise

**Description:** Adversary gains code execution on agent daemon.

**Attack Scenarios:**
- T6.1: Exploit vulnerability in daemon
- T6.2: Malicious dependency
- T6.3: Container escape

**Likelihood:** Medium
**Impact:** High (can sign with present disk)

**Mitigations:**
- M6.1: Minimal attack surface; no external APIs
- M6.2: Dependency auditing; minimal deps
- M6.3: Sandboxing; capability restrictions

### T7: Malicious Disk Insertion

**Description:** Adversary tricks user into inserting malicious disk.

**Attack Scenarios:**
- T7.1: Disk with malformed data triggers parsing bug
- T7.2: Disk with malicious presigs (if mother compromised)
- T7.3: Social engineering to use attacker's disk

**Likelihood:** Medium
**Impact:** High (potential code execution)

**Mitigations:**
- M7.1: Robust parsing with fuzzing; sandboxed validation
- M7.2: Mother signature verification
- M7.3: User training; disk labeling

### T8: zkVM Exploit

**Description:** Adversary exploits vulnerability in SP1 zkVM.

**Attack Scenarios:**
- T8.1: Forge proofs without valid computation
- T8.2: Extract witnesses from proofs
- T8.3: DoS the prover

**Likelihood:** Low
**Impact:** High (breaks auditability)

**Mitigations:**
- M8.1: Use audited zkVM; verify proofs independently
- M8.2: Zero-knowledge property of proof system
- M8.3: Proof generation timeout; resource limits

### T9: Timing Attack on Reconciliation

**Description:** Adversary uses timing differences to infer disk contents.

**Attack Scenarios:**
- T9.1: Measure time to detect anomalies
- T9.2: Infer presig count from processing time

**Likelihood:** Low
**Impact:** Low (information leakage)

**Mitigations:**
- M9.1: Constant-time comparison where security-critical
- M9.2: No sensitive branching on disk contents

### T10: Rollback Attack

**Description:** Adversary reverts disk to previous state.

**Attack Scenarios:**
- T10.1: Restore backup of disk before some presigs used
- T10.2: Rewrite status bytes from "used" to "fresh"

**Likelihood:** Medium
**Impact:** High (presig reuse → key leakage)

**Mitigations:**
- M10.1: Agent maintains authoritative used-index list
- M10.2: Cross-reference agent state during signing
- M10.3: Reconciliation detects inconsistencies

## 6. Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                    TRUST BOUNDARY 1                         │
│                   (Air-gapped Zone)                         │
│  ┌─────────────────┐                                        │
│  │  Mother Device  │                                        │
│  │  - Master cold  │                                        │
│  │  - Child gen    │                                        │
│  │  - Presig gen   │                                        │
│  └─────────────────┘                                        │
└─────────────────────────────────────────────────────────────┘
         │ Physical transfer (disk)
         ▼
┌─────────────────────────────────────────────────────────────┐
│                    TRUST BOUNDARY 2                         │
│                   (Physical Medium)                         │
│  ┌─────────────────┐                                        │
│  │  Floppy Disk    │                                        │
│  │  - Cold presigs │                                        │
│  │  - Usage log    │                                        │
│  └─────────────────┘                                        │
└─────────────────────────────────────────────────────────────┘
         │ USB connection
         ▼
┌─────────────────────────────────────────────────────────────┐
│                    TRUST BOUNDARY 3                         │
│                   (User Workstation)                        │
│  ┌─────────────────┐      ┌─────────────────┐              │
│  │   User/Claude   │◄────►│   Sigil CLI     │              │
│  └─────────────────┘      └─────────────────┘              │
│                                  │ IPC                      │
│                                  ▼                          │
│                           ┌─────────────────┐              │
│                           │  Sigil Daemon   │              │
│                           │  - Agent shards │              │
│                           │  - Signing      │              │
│                           └─────────────────┘              │
└─────────────────────────────────────────────────────────────┘
         │ Network
         ▼
┌─────────────────────────────────────────────────────────────┐
│                    TRUST BOUNDARY 4                         │
│                   (External Network)                        │
│  ┌─────────────────┐                                        │
│  │   Blockchain    │                                        │
│  │   - TX broadcast│                                        │
│  └─────────────────┘                                        │
└─────────────────────────────────────────────────────────────┘
```

## 7. Security Requirements

### 7.1 Confidentiality Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| C1 | Master shards never leave their devices | CRITICAL |
| C2 | Child shards embedded in presigs only | CRITICAL |
| C3 | Nonce shares are single-use | CRITICAL |
| C4 | Usage log does not leak private data | HIGH |

### 7.2 Integrity Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| I1 | Disk header authenticated by mother | CRITICAL |
| I2 | Presig status transitions are monotonic | HIGH |
| I3 | Usage log is append-only | HIGH |
| I4 | zkVM proofs are unforgeable | CRITICAL |

### 7.3 Availability Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| A1 | Disk readable after 1000 insertions | MEDIUM |
| A2 | Signing completes in < 60 seconds | MEDIUM |
| A3 | Graceful degradation on disk errors | LOW |

## 8. Security Controls

### 8.1 Preventive Controls

| Control | Threats Mitigated | Implementation |
|---------|-------------------|----------------|
| 2-of-2 MPC | T1, T2 | Cryptographic protocol |
| Single-use presigs | T2.1, T10 | Status byte + agent tracking |
| Mother signature | T7.2 | Ed25519 over header |
| Air-gapped generation | T5 | Operational procedure |
| Constant-time crypto | T6, T9 | k256 crate features |

### 8.2 Detective Controls

| Control | Threats Detected | Implementation |
|---------|------------------|----------------|
| Usage log | T3, T10 | On-disk audit trail |
| Reconciliation | T3, T10 | Mother ceremony |
| Anomaly detection | T1, T3 | Log analysis |
| zkVM proofs | All signing | SP1 attestations |

### 8.3 Corrective Controls

| Control | Response To | Implementation |
|---------|-------------|----------------|
| Nullification | Compromise detected | Registry update |
| Presig revocation | Disk stolen | Agent wipes shares |
| Emergency reserve | Near exhaustion | 50 presigs held back |

## 9. Residual Risks

### 9.1 Accepted Risks

| Risk | Rationale | Mitigation Owner |
|------|-----------|------------------|
| Physical theft of disk | Bounded by N presigs | User |
| Mother device physical security | Air-gapped, rare access | Operator |
| zkVM implementation bugs | Use audited implementation | SP1 team |

### 9.2 Risks Requiring Further Analysis

| Risk | Concern | Next Steps |
|------|---------|------------|
| SLIP-10 derivation security | Non-standard for secp256k1 | Review against BIP32 |
| SP1 soundness assumptions | Relatively new system | Track security advisories |
| Disk media degradation | Floppy disks are old tech | Test with modern USB floppies |

## 10. Recommendations

### 10.1 High Priority

1. **Formal verification** of the signature completion algorithm
2. **Independent security audit** of cryptographic code
3. **Penetration testing** of daemon IPC interface
4. **Fuzzing campaign** for disk parsing

### 10.2 Medium Priority

5. **Hardware security module** option for agent shard
6. **Multi-signature** support (2-of-3 or higher)
7. **Secure enclave** execution for signing
8. **Geographic distribution** of presig shares

### 10.3 Future Considerations

9. **Post-quantum** signature upgrade path
10. **Threshold** signature schemes (t-of-n)
11. **Verifiable secret sharing** for master shards

## 11. Changelog

- **v0.1.0** (2026-01-17): Initial threat model
