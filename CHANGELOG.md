# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Note**: Starting from version 0.1.0, versions are automatically bumped when changes are merged to main, based on [Conventional Commits](https://www.conventionalcommits.org/). See [VERSIONING.md](VERSIONING.md) for details.

## [Unreleased]

## [0.7.0] - 2026-09-23

See commit history for changes in this release.


### Added

- **`DiskStatus` reports the disk's child public key** (`child_pubkey`: 33-byte
  compressed secp256k1, hex). Clients can now name the disk's account and
  recover the ECDSA `v` without spending a presignature. `sigil disk` prints
  it, and `sigil-mcp` fills `DiskState.public_key` from it. The field is
  optional on the wire, so older daemons still parse. See
  `specs/004-disk-public-key/`.

## [0.6.0] - 2026-09-20

### Fixed

**The release train had never run**

Four version tags (`v0.2.0` … `v0.5.0`) existed and zero releases did. The
tags were pushed by `auto-version.yml` using `secrets.GITHUB_TOKEN`, and GitHub
does not trigger workflows from events created by a workflow's own
`GITHUB_TOKEN` — so `release.yml`, which triggers on `push: tags`, never
started. Six auto-version runs, all reporting success, produced no release, no
binaries and no published crate. `VERSIONING.md` documented step 6 as
"Triggers the release workflow", which had never been true.

Tags are now pushed by a person, which does start `release.yml`. The workflow
also gained `workflow_dispatch`, so it can be re-run against a tag that already
exists.

**Release artifacts were incomplete and unverifiable**

The release tarball shipped `sigil`, `sigil-daemon` and `sigil-mother`, and
omitted `sigil-mother-tui` and `sigil-mcp` — three fifths of the product, with
no indication the rest was missing. The packaging step now enumerates every
binary the workspace ships and fails if one is absent, and publishes a
`.sha256` beside the tarball.

### Changed

**CI proposes releases; it no longer authors commits on integration branches**

`auto-version.yml` bumped the version, committed, tagged and pushed directly to
`main` under CI credentials — a commit on the default branch that no person had
read. It is replaced by `release-prep.yml`, which opens a
`chore: release vX.Y.Z` pull request against `staging` and stops.

The re-entry guard changed with it. The old one skipped when the last commit
message began `chore: bump version to`, which only recognised its own commits
and was defeated by a squash or a merge commit. The new one proposes a bump
only when the workspace version equals the latest release tag, so a prepared
release awaiting its tag cannot be bumped past.

`release.yml` now verifies the tagged commit — `fmt`, `clippy` and the full
test suite — before building anything. A tag can point at any commit, so CI
being green on `main` is not evidence about the one being released.

### Removed

**crates.io publishing, until coverage justifies it**

The `publish` job could not have succeeded under any circumstances: internal
dependencies are declared `{ path = ... }` with no version requirement, so
`cargo package` refuses them; `sigil-frost` is a dependency of `sigil-daemon`
and `sigil-mother` and was absent from the publish order; and the crate name
`sigil-cli` belongs to an unrelated crate on crates.io. Every step carried
`continue-on-error: true`, so the job reported success for all of it.

Publishing is on hold until test coverage and end-to-end assurance justify
putting key-custody crates into a public namespace, where a version cannot be
withdrawn. Tracked as backlog item 21 in `specs/README.md`. The supported
install is `cargo install --locked --git ... --tag`, which does not use
crates.io.

### Security

**Two constitutional requirements became executable**

`ci_never_pushes_to_an_integration_branch` and
`no_workflow_step_reports_success_on_failure` in
`crates/sigil-tests/tests/constitution_conformance.rs`. Both were negative
tested: restoring `auto-version.yml` fails the first naming
`auto-version.yml:181: git push origin main`, and restoring the old
`release.yml` fails the second naming all five `continue-on-error: true` lines
in the `publish` job.

The constitution's Security Requirements gained the matching clauses: CI must
not push to an integration branch, and a workflow must not claim an outcome it
did not produce.

### Added

**Operator CLI: spec and tests** ([#59](https://github.com/chippr-robotics/sigil/issues/59) backlog item 4)

`sigil-cli` was 858 lines with zero tests, and it is the path an operator
actually runs to make a signature happen. `specs/003-operator-cli/` specifies
it; 21 tests now cover the eight subcommands, the daemon connection, fail-closed
behaviour and the display formatting.

**Requirement traceability gate**

`crates/sigil-tests/tests/spec_traceability.rs` gates the specs themselves:
every `FR-xxx` must appear in its spec's Coverage table, every test named in a
Coverage table must exist, no spec past Draft may carry `NEEDS CLARIFICATION`,
and every spec must be listed in the backlog.

It found two gaps on its first run: spec 001 had 27 requirements and **no
Coverage table at all**, and a range row that hid five requirements inside it.
Both fixed. It lives in `sigil-tests`, so it runs in the existing `Unit Tests`
job — no new CI job needed.

### Fixed

**The CLI could not reach a default-configured daemon**

`#57` moved the daemon's default socket off world-writable `/tmp` to
`/run/sigil/sigil.sock`, updating `sigil-daemon` and `sigil-bridge` — and
missing `sigil-cli` in two places: the `--socket` clap default and
`SigilClient::new`'s fallback.

So the CLI looked for the daemon where the daemon no longer listened, in a
world-writable directory where any local user can create a socket and receive
the operator's signing requests. Both now use one `DEFAULT_UNIX_SOCKET_PATH`
constant, pinned by test to the daemon's value — the two halves live in crates
that cannot see each other's constants, so the pin is what keeps them honest.

**A malformed signature could panic the CLI**

`format_signing_result_for_display` did `&s[..18]`, which panics on a
signature shorter than 18 bytes and again on a multi-byte character straddling
the boundary. Both are reachable, since the shortening runs on whatever the
daemon returned. Replaced with a character-counting truncation; negative-tested
by restoring the old slicing, which reproduces both panics.

### Changed

**Two stale plan documents retired**

`MCP_INTEGRATION_PLAN.md` (35 checkboxes, none checked) and
`SIGIL_MOTHER_TUI_PLAN.md` (58, none checked) describe components that shipped
— 5,164 and 6,801 lines respectively. Moved to `documentation/history/` with a
README stating plainly that they are not current.

`E2E_TEST_PLAN.md` deliberately stays in `documentation/`: it describes 58
scenarios against 7 implemented, so it is aspirational rather than stale.

### Added

**Physical-consent enforcement: spec and tests**
([#59](https://github.com/chippr-robotics/sigil/issues/59) backlog item 1)

`sigil-daemon/src/signer.rs` was 478 lines with zero tests. Its only test
module was a placeholder reading "Integration tests would require full setup
with disk and agent store". `Signer::sign()` is where Sigil's claim is either
true or false, and it was the least-verified critical code in the repository.

`specs/002-physical-consent-enforcement/` specifies the path; nine tests now
cover it:

- Fail closed with no disk present, and when the disk is removed mid-session.
- The disk is re-read from the block device on every operation — state written
  out-of-band is observed, a cached copy is not trusted.
- Exhaustion is enforced; signing past the presignature supply fails.
- Cold and agent halves disagreeing on their R point is rejected, and the
  rejected attempt consumes nothing.
- A successful signature burns its presignature and the burn is persisted to
  the disk, not held in memory.
- N signatures consume N distinct indices — a repeated index would mean a
  reused ECDSA nonce, which discloses the private key.
- One usage-log entry per signature, carrying the index and description.

### Security

**Fixed: disk rollback was not detected (spec FR-014, FR-015)**

`AgentChildData::next_presig_index` was written by
`AgentStore::mark_presig_used` and read nowhere outside `agent_store.rs`.
`get_presig_share(child, i)` returned share `i` whether or not it had been
spent, so the disk's burn was the only thing preventing a presignature being
used twice.

Restoring an earlier disk image defeated that: the restored disk offers a spent
index, the agent store serves the matching half, the same nonce `k` signs two
different messages, and two signatures sharing `r` give
`k = (z₁ − z₂)/(s₁ − s₂)` and then `d = (s₁·k − z₁)/r`. Full private key
disclosure from a file restore.

- **FR-014**: `Signer::sign` now reads the mark before fetching the agent half
  and returns `DaemonError::PresigAlreadyConsumed { index, next_expected }`
  when the disk offers an index below it. The error explains that the disk may
  be a restored image and what to do about it.
- **FR-015**: refill resets the mark. `AgentStore::import_child_shares` zeroes
  `next_presig_index`, and the `ImportChildShares` IPC handler calls it instead
  of `store_child`. The reset is enforced there rather than trusted from the
  payload, because that handler deserializes `AgentChildData` straight from
  JSON — a stale value would brick the child and a crafted one would disable
  the guard.

Both were negative-tested: with the guard removed, a rolled-back disk returns a
real signature on an already-spent presignature.

Reconciliation remains the detective control, comparing usage logs after the
fact. FR-014 is the preventive one.

### Added

**Constitution conformance tests** ([#59](https://github.com/chippr-robotics/sigil/issues/59))

`crates/sigil-tests/tests/constitution_conformance.rs` makes the constitution
executable. Four of seven principles had nothing asserting them; three now do,
plus one that was only vacuously true.

| Principle | Asserted by |
| --- | --- |
| I. One TCB | `default-members == members`; any crate excluded from it must be `publish = false` |
| III. Default deny at the network edge | No crate may depend on an HTTP server framework |
| V. Supported install is auditable | `install.sh` refuses a pipe, admits a real file, and no document instructs piping into a shell |
| VI. No outbound path from the air-gapped side | Knowledge-base and sync tooling denylisted from `.claude/skills/` |

Every assertion was negative-tested — each shown to fail, naming the exact
violation, before being committed. An assertion nobody has seen fail is the same
category of object as a `Security Audit` job that cannot fail.

**The pipe-to-shell test caught a real instance on its first run.**
`docs/docs/zkvm-proofs.html` documented SP1's toolchain install as
`curl -L https://sp1.succinct.xyz | bash`. The sweep in #57 grepped for
`| sudo bash` and missed it. The mother device holds master key material, so
code that runs there should be read before it runs; now documented as download,
inspect, then run.

**Principle II remains the gap**, and it is the one the product rests on. The
daemon's disk re-read, presignature consumption and burn-on-use are still
untested — `sigil-daemon/src/signer.rs` is 478 lines with zero tests. That is
backlog item 1 in `specs/README.md`, and the next piece of work.

### Fixed

- `docs/docs/zkvm-proofs.html` no longer instructs piping a remote installer
  into a shell.

### Added

**`specs/README.md` — the spec backlog and coverage inventory**
([#59](https://github.com/chippr-robotics/sigil/issues/59))

Spec-kit arrived with one spec; everything else predates it and the
constitution. This is the inventory of what needs specifying, ranked, so specs
accrete deliberately rather than only where an issue happens to be filed.

What the survey turned up:

- **`sigil-daemon/src/signer.rs` is 478 lines with zero tests.** `Signer::sign()`
  is the entire physical-consent path — disk re-read, presignature consumption,
  burn-on-use, persist. Every claim Sigil makes lives there and nothing asserts
  any of it. The eight daemon tests added in #57 cover the transport around the
  signer, not the signer. This is ranked first.
- `sigil-cli` is 858 lines with zero tests, and it is the operator signing path.
- `sigil-mother/src/ceremony.rs` is 517 lines with zero tests.
- **`MCP_INTEGRATION_PLAN.md` has 35 checkboxes, none checked;
  `SIGIL_MOTHER_TUI_PLAN.md` has 58, none checked.** Both describe components
  that shipped — 5,164 and 6,801 lines respectively. A contributor reading
  either would conclude the component does not exist. Both are marked for
  retirement rather than conversion: a completed plan converted into a spec
  describes a past intention, which is worse than no spec.
- **`E2E_TEST_PLAN.md` describes 58 scenarios; 7 are implemented.** Unlike the
  two above it is aspirational rather than stale, so it is marked for
  reconciliation into spec acceptance criteria, not retirement.
- Three of seven constitution principles have nothing asserting them (I, V,
  VI). The file names the cheap assertion that closes each.
- Naming collision recorded: `sigil-mcp/src/invariants/` is input validation,
  not constitution conformance. The conformance suite needs a different home.

### Removed

**The mobile app and the HTTP bridge** ([#58](https://github.com/chippr-robotics/sigil/issues/58))

The mobile signing UX moves to the FairWins platform, so deployment and
management follow one pattern across the Chippr suite. Sigil keeps what it is:
the disk format, the threshold crypto, the daemon that will not sign without a
physically inserted disk, and the operator tooling.

- **`mobile/` deleted.** 20 Dart files of Flutter scaffolding that was never
  buildable: `flutter pub get` fails on `flutter_clipboard_manager ^0.0.4`,
  which resolves to no published version and which nothing in the app imports
  (the code uses Flutter's built-in `Clipboard` from `services.dart`). There
  were also no `android/`, `ios/`, or `test/` directories, while
  `.github/workflows/mobile.yml` ran `flutter build apk`, an iOS build, and
  `flutter test`. That workflow has never passed on any branch.
- **`.github/workflows/mobile.yml` deleted** with the app it built.
- **`crates/sigil-bridge` deleted.** The bridge existed for exactly one
  consumer — the app above. The surface issue #53 found in it (`0.0.0.0:8080`,
  wildcard CORS, unauthenticated `POST /api/sign`, shard import over HTTP) was
  serving a client that could not be compiled. Hardening it was the right first
  move; removing it is better. A remote UI is now FairWins' to build against an
  interface chosen deliberately, out of TCB, rather than an HTTP shim inherited
  from a demo. The hardened version remains in git history.

### Changed

- **Every workspace member is now in the TCB.** `default-members` is identical
  to `members`, and nothing in this repository terminates HTTP or listens on a
  network socket. Both lists stay explicit so that adding an out-of-TCB crate
  is a visible act in `Cargo.toml` rather than a silent default.
- `SECURITY.md` standing invariant 3 strengthened accordingly: there is no
  network-facing signing endpoint to authenticate, because there is no
  network-facing endpoint.
- `README.md` trust boundary and `.specify/memory/constitution.md` Principle I
  updated to match.
- `specs/001-tcb-quarantine/spec.md` carries a supersession note: its User
  Story 1 hardened endpoints that no longer exist.

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

[Unreleased]: https://github.com/chippr-robotics/sigil/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/chippr-robotics/sigil/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/chippr-robotics/sigil/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/chippr-robotics/sigil/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/chippr-robotics/sigil/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/chippr-robotics/sigil/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/chippr-robotics/sigil/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/chippr-robotics/sigil/releases/tag/v0.1.0
