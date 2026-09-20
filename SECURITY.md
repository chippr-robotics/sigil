# Security Policy

## Trusted Computing Base

Sigil's security claim is narrow and literal: **a signature cannot exist
without a physically present disk.** The trusted computing base is the set of
components that could make that claim false.

### In the TCB

| Crate | Role |
| --- | --- |
| `sigil-core` | Disk format, crypto primitives, presignature structures |
| `sigil-frost` | FROST threshold Schnorr signatures |
| `sigil-zkvm` | SP1 signing program |
| `sigil-mother-zkvm` | SP1 mother-operation programs |
| `sigil-daemon` | Disk watcher, IPC, signing orchestration |
| `sigil-cli` | Operator signing and disk management |
| `sigil-mother` | Air-gapped key generation, child disk creation |
| `sigil-mother-tui` | Terminal UI for mother operations |

### Explicitly NOT in the TCB

| Component | Why it is fenced, and how |
| --- | --- |
| `sigil-bridge` | HTTP transport for the mobile app. Excluded from the workspace's `default-members`, `publish = false`, binds loopback only unless explicitly acknowledged, bearer token on every `/api` route, transports no key material. |
| `mobile/` | Flutter client. Talks only to the bridge. |
| Mock / demo modes | `sigil-mcp`'s mock mode sits behind a non-default `mock` cargo feature and **returns an error for every signing operation**. It fabricates disk *status* only. A default-feature release binary cannot construct a mock signer. |

Adding a component to the TCB is a change to
[`.specify/memory/constitution.md`](.specify/memory/constitution.md), not an
ordinary pull request.

### Standing invariants

1. The daemon re-reads presignature shares from the block device on every
   signing operation and fails closed with `NoDiskDetected` without one.
2. No mock, test, or development mode returns signature-shaped bytes.
3. Nothing in this repository listens on a non-loopback interface by default,
   accepts a wildcard cross-origin request, or exposes a signing endpoint
   without authenticating the caller.
4. Shard import and export never cross a network transport. They use physical
   media and in-TCB tooling.
5. The local IPC socket grants no access to `other` and lives in a directory
   the daemon controls (`/run/sigil`, mode 0750), never world-writable `/tmp`.
6. The supported install is `cargo install --locked` from a pinned tag, or a
   cloned checkout. `scripts/install.sh` refuses to execute from a pipe.

### Removed surfaces

- **`POST /api/import-agent-shard`, `POST /api/import-child-shares`** —
  removed from `sigil-bridge`. They moved secret key material over
  unauthenticated HTTP. Import shards with `sigil-cli` on the device itself.
- **The Logseq knowledge-management skill** — removed from this repository. Its
  `sigil-mother-node` example instructed operators to merge air-gapped mother
  device material into a networked, indexed knowledge graph, which is an
  outbound path from the side of the air gap that must not have one. The
  content remains in git history; if you want it, it belongs in a separate
  repository that holds no key material.

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

**DO NOT** open a public GitHub issue for security vulnerabilities.

Instead, please email: **security@chippr-robotics.com** (placeholder - update with real contact)

Or use GitHub's private vulnerability reporting feature if available.

### What to Include

1. **Description** of the vulnerability
2. **Steps to reproduce** the issue
3. **Potential impact** assessment
4. **Suggested fix** (if any)
5. **Your contact information** for follow-up

### Response Timeline

- **24 hours**: Initial acknowledgment
- **72 hours**: Preliminary assessment
- **7 days**: Detailed response with remediation plan
- **90 days**: Public disclosure (coordinated)

### Scope

In scope:
- Cryptographic vulnerabilities
- Key extraction attacks
- Signature forgery
- Presignature reuse vulnerabilities
- Side-channel attacks
- zkVM proof forgery
- Disk parsing vulnerabilities
- IPC authentication bypass

Out of scope:
- Physical attacks requiring extended device access
- Social engineering attacks
- DoS attacks without security impact
- Issues in dependencies (report upstream)

## Security Hardening Checklist

### For Operators

- [ ] Air-gap the mother device (no network connection ever)
- [ ] Use a hardware random number generator
- [ ] Verify checksums of all software
- [ ] Perform key ceremony with multiple witnesses
- [ ] Store master shard backups in separate secure locations
- [ ] Enable disk encryption on agent server
- [ ] Run daemon with minimal privileges
- [ ] Monitor for anomalous signing patterns
- [ ] Regularly reconcile child disks
- [ ] Have incident response plan ready

### For Users

- [ ] Never share your floppy disk
- [ ] Store disk in a secure location when not in use
- [ ] Verify disk label before insertion
- [ ] Check presig count after each session
- [ ] Report lost or stolen disks immediately
- [ ] Return disk for reconciliation regularly
- [ ] Review usage log for unexpected entries

## Security Assumptions

The security of Sigil relies on:

1. **Cryptographic assumptions**:
   - ECDLP is hard on secp256k1
   - SHA-256 is collision-resistant
   - HMAC-SHA512 is a secure PRF

2. **Implementation assumptions**:
   - k256 crate provides constant-time operations
   - SP1 zkVM is computationally sound
   - System RNG provides sufficient entropy

3. **Operational assumptions**:
   - Mother device remains air-gapped
   - Physical security of floppy disks
   - Agent server is not fully compromised

## Known Limitations

1. **No post-quantum security**: ECDSA is vulnerable to quantum computers
2. **Single point of failure**: Master shard loss = permanent loss
3. **Physical medium**: Floppy disks can degrade or be damaged
4. **Trust in zkVM**: Proofs rely on SP1 implementation correctness

## Security Audit Status

| Component | Audit Status | Auditor | Date |
|-----------|--------------|---------|------|
| sigil-core | Pending | - | - |
| sigil-zkvm | Pending | - | - |
| sigil-daemon | Pending | - | - |
| sigil-mother | Pending | - | - |
| Cryptographic protocol | Pending | - | - |

## Bug Bounty

Currently, we do not have a formal bug bounty program. However, we commit to:

- Acknowledging all valid security reports
- Crediting researchers who report vulnerabilities responsibly
- Not pursuing legal action against good-faith security researchers

## Security Contacts

- **Primary**: security@chippr-robotics.com
- **PGP Key**: (To be published)
- **GitHub Security Advisories**: Enabled

## References

- [CRYPTO_SPEC.md](documentation/CRYPTO_SPEC.md) - Cryptographic specification
- [THREAT_MODEL.md](documentation/THREAT_MODEL.md) - Threat model analysis
- [documentation/RECOVERY.md](documentation/RECOVERY.md) - Recovery procedures
- [.specify/memory/constitution.md](.specify/memory/constitution.md) - Project constitution (TCB principles)
- [specs/001-tcb-quarantine/spec.md](specs/001-tcb-quarantine/spec.md) - TCB quarantine specification
