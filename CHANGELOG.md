# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Note**: Starting from version 0.1.0, versions are automatically bumped when changes are merged to main, based on [Conventional Commits](https://www.conventionalcommits.org/). See [VERSIONING.md](VERSIONING.md) for details.

## [Unreleased]

### Added
- Automatic semantic versioning on merge to main
- Auto-version GitHub Actions workflow
- Conventional Commits support for version control
- Comprehensive documentation for automated versioning

## [0.1.0] - 2026-01-20

Initial release of Sigil - a physical containment system for agentic MPC management.

### Added
- 2-of-2 MPC threshold signature implementation using FROST
- Support for multiple cryptographic ciphersuites:
  - Bitcoin (secp256k1-tr with Taproot)
  - Ethereum (secp256k1)
  - Solana/Cosmos/other EdDSA chains (Ed25519)
  - Zcash shielded transactions (Ristretto255)
- Floppy disk-based presignature storage and management
- Mother device for air-gapped key management and child disk creation
- Daemon for disk detection and transaction signing
- CLI tools for signing operations
- Model Context Protocol (MCP) server for AI agent integration
- zkVM integration (SP1) for provable signing operations
- Hardware wallet support (Ledger) for secure key generation
- Comprehensive security model with reconciliation
- Documentation for cryptographic specifications and threat model

### Security
- Initial security audit completed
- Timing-safe cryptographic operations
- Air-gapped master key storage
- Physical consent requirement for signing operations

[Unreleased]: https://github.com/chippr-robotics/sigil/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/chippr-robotics/sigil/releases/tag/v0.1.0
