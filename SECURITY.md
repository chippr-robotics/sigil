# Security Policy

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
