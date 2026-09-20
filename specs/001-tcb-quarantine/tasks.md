# Tasks: Quarantine the Second TCB

**Input**: [spec.md](./spec.md), [plan.md](./plan.md)
**Tests**: Requested — every principle regression in scope gets a test, per Constitution §Security Requirements.

## Format: `[ID] [P?] [Story] Description`

- `[P]` = parallelisable (different files, no dependency)
- `[USn]` = the user story it serves

## Path Conventions

Rust cargo workspace; paths are repository-root relative.

---

## Phase 1: Setup

- [X] T001 Establish the TCB boundary in `.specify/memory/constitution.md`.
- [X] T002 Add `default-members` to root `Cargo.toml` listing every crate except `sigil-bridge`.

## Phase 2: Foundational

- [X] T003 [US1] Mark `sigil-bridge` out of TCB: `publish = false` in `crates/sigil-bridge/Cargo.toml`, `# NOT IN TCB` banner at the top of `crates/sigil-bridge/README.md`, crate-level `//!` warning in `crates/sigil-bridge/src/main.rs`.

## Phase 3: User Story 1 — Bridge unreachable from the network (P1) 🎯 MVP

### Tests

- [X] T004 [US1] Test: default `Args` resolve to a loopback bind host.
- [X] T005 [US1] Test: a non-loopback host without acknowledgement is rejected by the bind-policy check.
- [X] T006 [US1] Test: `/api/sign` without a bearer token returns `401`.
- [X] T007 [US1] Test: `/api/sign` with a wrong bearer token returns `401`.
- [X] T008 [US1] Test: `/api/import-agent-shard` and `/api/import-child-shares` return `404`.
- [X] T009 [US1] Test: `/health` is reachable without a token and discloses no disk state.
- [X] T010 [US1] Test: constant-time token comparison rejects equal-length mismatches.

### Implementation

- [X] T011 [US1] Create `crates/sigil-bridge/src/auth.rs`: token resolution (`--token-file` → `SIGIL_BRIDGE_TOKEN` → generate), 0600 persistence, constant-time compare, axum middleware returning `401`.
- [X] T012 [US1] `crates/sigil-bridge/src/main.rs`: change `--host` default to `127.0.0.1`.
- [X] T013 [US1] `crates/sigil-bridge/src/main.rs`: add `--allow-non-loopback` acknowledgement gate; refuse to start otherwise; `warn!` when engaged.
- [X] T014 [US1] `crates/sigil-bridge/src/main.rs`: delete the permissive `CorsLayer`; add repeatable `--allow-origin` building an explicit-origin layer, absent by default.
- [X] T015 [US1] `crates/sigil-bridge/src/main.rs`: remove `/api/import-agent-shard` and `/api/import-child-shares` routes and their handlers/DTOs.
- [X] T016 [US1] `crates/sigil-bridge/src/client.rs`: remove `import_agent_shard` and `import_child_shares`.
- [X] T017 [US1] `crates/sigil-bridge/src/main.rs`: apply the auth middleware to the `/api` router only, leaving `/health` open.
- [X] T018 [US1] `crates/sigil-bridge/Cargo.toml`: add `rand`, `subtle`, `axum` middleware deps as needed; `publish = false`.

## Phase 4: User Story 2 — A mock never produces a signature (P1)

### Tests

- [X] T019 [US2] Test: mock-mode `sign` returns `Err(MockSigningDisabled)`.
- [X] T020 [US2] Test: mock-mode `sign_frost` returns an error.
- [X] T021 [US2] Test: mock-mode `get_disk_status` and `get_presig_count` still succeed.

### Implementation

- [X] T022 [US2] `crates/sigil-mcp/Cargo.toml`: add non-default `mock` feature.
- [X] T023 [US2] `crates/sigil-mcp/src/client.rs`: gate `DaemonMode::Mock` on `cfg(any(test, feature = "mock"))`; make mock `sign`/`sign_frost` return `ClientError::MockSigningDisabled`.
- [X] T024 [US2] `crates/sigil-mcp/src/server.rs` and `handlers/mod.rs`: gate `with_mock`, `new_with_mock`, and the `Default` impl behind the same cfg.
- [X] T025 [US2] `crates/sigil-mcp/src/main.rs`: gate the `--mock` flag and its branch behind `feature = "mock"`; keep a clear error when the feature is absent.

## Phase 5: User Story 3 — No documented outbound path (P2)

- [X] T026 [US3] Remove `.claude/skills/logseq/` in its entirety.
- [X] T027 [US3] Record the removal and rationale in `CHANGELOG.md` and `SECURITY.md`.

## Phase 6: User Story 4 — Auditable install (P2)

- [X] T028 [US4] `README.md`: replace the `curl | sudo bash` one-liner with `cargo install --locked` and a cloned-checkout path; keep `install.sh` documented for system integration only.
- [X] T029 [US4] `scripts/install.sh`: rewrite the header; add a stdin-execution guard that exits non-zero.
- [X] T030 [P] [US4] `docs/docs/recovery.html`: replace the pipe one-liner.
- [X] T031 [P] [US4] `mobile/README.md`, `mobile/DEMO.md`: document loopback + token startup, remove LAN-IP-as-default guidance.

## Phase 7: User Story 5 — Local IPC least privilege (P2)

### Tests

- [X] T032 [US5] Test: after `bind`, the socket's mode grants nothing to `other`.
- [X] T033 [US5] Test: `bind` creates an absent parent directory with mode 0700.
- [X] T034 [US5] Test: the default socket path is not under `/tmp` when `XDG_RUNTIME_DIR` is unset.

### Implementation

- [X] T035 [US5] `crates/sigil-daemon/src/ipc/unix.rs`: create parent dir at 0700 when absent; `set_permissions` on the socket after bind.
- [X] T036 [US5] `crates/sigil-daemon/src/config.rs`: add `ipc_socket_mode` (default `0o660`); change the fallback path away from `/tmp`.
- [X] T037 [US5] `crates/sigil-daemon/src/ipc/server.rs` / `connection.rs`: thread the configured mode through to `bind`.
- [X] T038 [US5] `scripts/install.sh`: write the new socket path into the generated config; add `RuntimeDirectory=sigil` to the systemd unit.

## Phase 8: Polish & Cross-Cutting

- [X] T039 `SECURITY.md`: add the explicit TCB membership list and the out-of-TCB list.
- [X] T040 `.github/workflows/mobile.yml`: relabel the bridge job as out-of-TCB and build it by explicit `-p` rather than relying on the default build.
- [X] T041 `CHANGELOG.md`: record every change in this feature under a Security heading.
- [X] T042 `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [X] T043 `cargo test --workspace` green.

## Phase 9: CI gates (added after review) — "valuable, not checkbox theater"

- [X] T044 `.github/workflows/ci.yml`: replace the five-crate `-p` allowlist in the test job with `cargo test --workspace --exclude sigil-zkvm`, plus `-p sigil-zkvm --lib` and `-p sigil-mcp --features mock`.
- [X] T045 `.cargo/audit.toml`: declare each advisory exception individually with its reason and the optional feature that reaches it.
- [X] T046 `.github/workflows/ci.yml`: remove `continue-on-error: true` from the Security Audit job; add it to `ci-success`'s `needs`.
- [X] T047 `.github/workflows/ci.yml`: convert Format Check and Clippy Lint from auto-fix-and-push into read-only gates; drop `contents: write`.
- [X] T048 `.github/workflows/ci.yml`: add `staging` to the push and pull_request branch filters.
- [X] T049 `crates/sigil-daemon/Cargo.toml`, `crates/sigil-cli/Cargo.toml`, root `Cargo.toml`: remove unused `tonic`/`prost`.
- [X] T050 `cargo update` to clear the 17 advisories with semver-compatible fixes.

## Dependencies & Execution Order

- Phase 1 → Phase 2 → Phases 3–7 (independent of each other) → Phase 8.
- Within US1: T011 before T017; T015 before T016 (or together).
- Within US2: T022 before T023–T025.
- US3, US4, US5 are fully parallel with US1 and US2 — different files.

## Implementation Strategy

US1 alone is a viable ship: it closes the only remotely exploitable hole. US2
is equally urgent by severity of claim but requires no attacker. US3–US5 are
documentation and local hardening and can land in the same PR without
increasing risk.
