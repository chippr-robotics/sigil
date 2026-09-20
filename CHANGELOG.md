# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Note**: Starting from version 0.1.0, versions are automatically bumped when changes are merged to main, based on [Conventional Commits](https://www.conventionalcommits.org/). See [VERSIONING.md](VERSIONING.md) for details.

## [Unreleased]

### Security

Quarantines the second trusted computing base ([#53](https://github.com/chippr-robotics/sigil/issues/53)).
Sigil's claim is that a signature cannot exist without a physically present
disk. The signing core honoured that; four surfaces around it did not. Full
specification in [`specs/001-tcb-quarantine/`](specs/001-tcb-quarantine/).

**`sigil-bridge` is now fenced out of the TCB**

- Removed from the workspace's `default-members`. A bare `cargo build` at the
  repository root no longer produces a binary that listens on a network socket.
  Build it deliberately with `cargo build -p sigil-bridge`.
- Marked `publish = false`, with out-of-TCB banners in its crate docs, README,
  and `--help`.
- **Default bind changed from `0.0.0.0` to `127.0.0.1`.** A non-loopback bind
  now requires `--allow-non-loopback` and logs a warning naming the exposure.
- **Removed the wildcard CORS policy** (`allow_origin(Any)` /
  `allow_methods(Any)` / `allow_headers(Any)`), which made any browser that
  could route to the bridge a confused deputy for `POST /api/sign`. Origins are
  now opt-in per `--allow-origin`.
- **Every `/api/*` route now requires a bearer token.** Unauthenticated
  requests are rejected with `401` before any IPC call reaches the daemon. The
  token comes from `--token-file`, `$SIGIL_BRIDGE_TOKEN`, or is generated at
  startup and persisted mode `0600`. There is no unauthenticated mode.
- **Removed `POST /api/import-agent-shard` and `POST /api/import-child-shares`**
  entirely. They transported secret key material over unauthenticated HTTP.
  Import shards with `sigil-cli` on the device itself.
- `/health` remains open but now discloses only liveness — no disk state,
  presignature counts, addresses, or child identifiers.

**Mock mode can no longer produce a signature**

- `sigil-mcp`'s mock mode returned a hard-coded signature and `proof_hash` on
  the same response path as a real signature, and was compiled into every
  release binary. Signing in mock mode now returns
  `ClientError::MockSigningDisabled`.
- Mock support moved behind a non-default `mock` cargo feature. A
  default-feature build cannot construct a mock signer, and `--mock` is not a
  recognised flag.
- Mock *status* (disk present, presignature counts) still works: status is not
  consent.

**Logseq assets removed from the key repository**

- Deleted `.claude/skills/logseq/`. Its `sigil-mother-node` example instructed
  operators to merge air-gapped mother device material into a networked,
  indexed knowledge graph — an outbound path from the side of the air gap that
  must not have one. Content remains in git history; it belongs in a separate
  repository holding no key material.

**Supported install is `cargo install`, not a pipe into root**

- `README.md` and `docs/docs/recovery.html` now document
  `cargo install --locked --git ... --tag <version>` or a cloned checkout.
- `scripts/install.sh` refuses to run when piped from stdin and explains what
  to do instead. It remains the supported way to do system integration (udev
  rules, systemd unit, `sigil` group) from a checkout you have read.

**Local IPC is no longer reachable by every local user**

- The daemon now applies explicit permissions (`0o660`, configurable via
  `ipc_socket_mode`) to its Unix socket after binding, instead of inheriting
  the process umask.
- The default socket path no longer falls back to `/tmp/sigil.sock`. It is
  `$XDG_RUNTIME_DIR/sigil.sock`, else `/run/sigil/sigil.sock`. A world-writable
  parent let any local user squat the path before the daemon started.
- The daemon creates an absent socket directory at mode `0700`; the systemd
  unit declares `RuntimeDirectory=sigil` with mode `0750`.

**CI checks are gates, not reports**

The checks that were supposed to be defending this repository were largely
decorative. Fixed in the same change, since a security PR whose CI does not
run the security tests proves nothing.

- **The `Security Audit` job could not fail.** It ran with
  `continue-on-error: true`, so it reported **22 vulnerabilities** and went
  green anyway — including timing side-channels and signature-validation
  bypasses in AWS-LC, and a TLS 1.3 handshake flaw in rustls. It is now a gate.
  `cargo update` cleared 17. The remaining 5 are declared individually in
  `.cargo/audit.toml`, each with its reason and the non-default feature that
  reaches it (`pkcs11` → cryptoki; `zkvm-sp1` → sp1-sdk → aws-sdk-kms). An
  advisory reachable from a default build is never ignored.
- **The `Unit Tests` job skipped 135 tests.** It ran a hand-maintained list of
  five `-p` invocations that omitted `sigil-frost` (threshold crypto, in the
  TCB), `sigil-mcp`, `sigil-mother-tui`, and `sigil-bridge` — every test
  asserting the invariants above among them. It now runs the whole workspace,
  plus `sigil-mcp --features mock`.
- **`Format Check` and `Clippy Lint` no longer push commits.** Both held
  `contents: write` and committed auto-fixes to the branch under test. A gate
  that rewrites the code it is gating is not a gate, and CI write access is an
  unaudited path to shipped code. They are now read-only and print the command
  to run. This also ends a loop between them: `clippy --fix` does not run
  rustfmt, so its commits were routinely not fmt-clean.
- **PRs targeting `staging` had no CI.** The `pull_request` trigger listed only
  `main`. `staging` added to `ci.yml`. Deliberately *not* added to
  `mobile.yml`, which cannot pass: `flutter pub get` fails on
  `flutter_clipboard_manager ^0.0.4`, an unresolvable dependency that no code
  imports. That workflow is a blocker rather than a gate and is scheduled for
  removal with the mobile app.
- **The workflow ran twice per push.** `claude/**` in the `push` filter plus a
  PR against an integration branch matched both triggers, producing two
  identical full runs per commit. `push` is now integration branches only —
  branch work is gated by its pull request — and a `concurrency` group
  supersedes in-flight PR runs when a new commit lands.
- `Security Audit` added to `ci-success`'s `needs`, so it can actually block.

**Unused gRPC stack removed from TCB crates**

`sigil-daemon` and `sigil-cli` declared `tonic` and `prost` without a single
reference in any source file and no `build.rs` generating proto types. They
pulled hyper 0.14, axum 0.6, h2 and rustls-webpki into the dependency graph of
two TCB crates. Removed. IPC is Unix sockets / named pipes carrying JSON;
`proto/signer.proto` is retained as a design note.

### Added

- `.specify/memory/constitution.md` — project constitution stating the TCB
  boundary and the invariants that keep the physical-consent claim true.
- `SECURITY.md` now lists TCB membership, out-of-TCB components, standing
  invariants, and removed surfaces.
- `README.md` gained a "Trust boundary" section.

### Changed

- Mobile documentation now describes the supported setup: an SSH tunnel to the
  bridge's loopback port with a bearer token, rather than a LAN IP.

## [0.5.0] - 2026-01-31

See commit history for changes in this release.


## [0.4.0] - 2026-01-28

See commit history for changes in this release.


## [0.3.0] - 2026-01-23

See commit history for changes in this release.


## [0.2.0] - 2026-01-23

See commit history for changes in this release.


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

[Unreleased]: https://github.com/chippr-robotics/sigil/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/chippr-robotics/sigil/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/chippr-robotics/sigil/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/chippr-robotics/sigil/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/chippr-robotics/sigil/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/chippr-robotics/sigil/releases/tag/v0.1.0
