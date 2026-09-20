# Implementation Plan: Quarantine the Second TCB

**Branch**: `053-tcb-quarantine` | **Date**: 2026-09-20 | **Spec**: [spec.md](./spec.md)

## Summary

Collapse Sigil's two trusted computing bases back into one. The disk-backed
signing core is already correct; the work is removing or fencing every path
that reaches a signature, or weakens the air gap, without passing through it:
the network-exposed HTTP bridge, the mock signer compiled into release
binaries, the Logseq assets that document an outbound path from the mother
device, the `curl | sudo bash` install, and the world-reachable IPC socket.

## Technical Context

**Language/Version**: Rust 2021, MSRV 1.75
**Primary Dependencies**: axum 0.7, tower-http 0.5, tokio 1.x, clap 4
**Storage**: N/A (disk format unchanged)
**Testing**: `cargo test` (unit + integration), `sigil-tests` for E2E
**Target Platform**: Linux (udev disk detection), macOS for mother tooling
**Project Type**: Rust cargo workspace
**Performance Goals**: N/A — no hot paths touched
**Constraints**: No change to `DiskFormat`, presignature consumption, or the
daemon's disk-presence checks. No new dependency in a TCB crate.
**Scale/Scope**: 5 crates touched, ~4 docs, 1 skill directory removed.

## Constitution Check

| Principle | Status | Note |
|---|---|---|
| I. One TCB | **Enforced** | Bridge leaves `default-members`, gains `publish = false` and a banner; `SECURITY.md` gains an explicit membership list. |
| II. No signature without physical consent | **Enforced** | Mock signing returns an error; mock support moves behind a non-default feature. |
| III. Default deny at the edge | **Enforced** | Loopback default, acknowledged non-loopback bind, CORS opt-in per origin, bearer token on all `/api/*`. |
| IV. Key material off convenience transports | **Enforced** | Shard-import routes deleted from the bridge. |
| V. Auditable install | **Enforced** | README switches to `cargo install --locked`; `install.sh` refuses to run from a pipe. |
| VI. No outbound path from the air-gapped side | **Enforced** | Logseq assets removed from this repository. |
| VII. Least privilege locally | **Enforced** | IPC socket mode restricted, default path moved out of `/tmp`, parent directory created 0700. |

No violations. No complexity-tracking entries required.

## Design Decisions

### D1 — `default-members` over workspace removal

The issue offers "delete bridge from default members **or** bind it to
localhost and mark it out of TCB". We do both, but keep the crate as a
workspace *member* while removing it from `default-members`.

Rationale: full `exclude` would give the bridge its own lockfile and
independent dependency resolution, and would silently drop it from
`cargo clippy --workspace`, so it would rot unchecked. `default-members`
achieves the stated goal — a bare `cargo build` at the root does not build it —
while `--workspace` still lints and tests it. CI keeps a clearly labelled
out-of-TCB job.

### D2 — Token rather than an allowlist

An IP or origin allowlist does not survive the threat it needs to: the wildcard
CORS policy meant any browser on the LAN was a confused deputy with the
operator's own network position. A bearer token checked before any IPC call
removes the confused-deputy class outright. It is explicitly *not* a claim that
the bridge is safe — it is a lock on a door marked "not part of the TCB".

Token resolution order: `--token-file` → `SIGIL_BRIDGE_TOKEN` → generated and
written to `$XDG_RUNTIME_DIR/sigil-bridge.token` (mode 0600). There is no
"no token" state.

### D3 — Mock mode errors instead of disappearing

Mock status (`check_disk`, presignature counts) is genuinely useful for
harnessing agent clients, and fabricating *status* does not falsify the
product's claim. Fabricating a *signature* does. So `sign`/`sign_frost` in mock
mode return `ClientError::MockSigningDisabled`, and the whole mock surface sits
behind `feature = "mock"`, active for the crate's own tests via
`cfg(any(test, feature = "mock"))`.

### D4 — Socket hardening, not a socket redesign

`UnixIpcTransport::bind` gains a `fchmod`-equivalent step after binding and
creates the parent directory at 0700 when absent. The default path falls back
to `/run/sigil/sigil.sock` for root and `$XDG_RUNTIME_DIR` otherwise, instead
of `/tmp`. Mode defaults to `0o660` so the existing `root:sigil` systemd model
keeps working; it is configurable for non-systemd deployments.

### D5 — install.sh stays, pipe execution does not

The script does real work that `cargo install` cannot: udev rules, the systemd
unit, the `sigil` group. Deleting it would push operators to worse ad-hoc
steps. It gains a guard that detects stdin execution (`BASH_SOURCE[0]` absent
or non-regular) and exits, and its own header stops advertising the pipe.

## Project Structure

### Documentation (this feature)

```
specs/053-tcb-quarantine/
├── spec.md
├── plan.md
└── tasks.md
```

### Source Code (repository root)

```
Cargo.toml                          # + default-members (bridge excluded)
crates/
├── sigil-bridge/                    # OUT OF TCB
│   ├── Cargo.toml                   # publish = false, rand dep for token
│   ├── README.md                    # NOT IN TCB banner
│   └── src/
│       ├── main.rs                  # loopback default, auth layer, CORS removal,
│       │                            #   import routes deleted, ack gate
│       ├── auth.rs                  # NEW: token resolution + middleware
│       └── client.rs                # import methods removed
├── sigil-daemon/
│   └── src/
│       ├── config.rs                # default path off /tmp, ipc_socket_mode
│       └── ipc/unix.rs              # parent dir 0700, socket mode after bind
└── sigil-mcp/
    ├── Cargo.toml                   # [features] mock
    └── src/
        ├── client.rs                # mock sign -> error, cfg-gated mock mode
        ├── server.rs                # cfg-gated with_mock
        ├── main.rs                  # cfg-gated --mock flag
        └── handlers/mod.rs          # cfg-gated new_with_mock / Default
.claude/skills/logseq/               # REMOVED
scripts/install.sh                   # pipe guard, header rewrite
README.md, SECURITY.md, CHANGELOG.md, docs/docs/recovery.html, mobile/*.md
.github/workflows/mobile.yml         # bridge job labelled out-of-TCB
```

## Complexity Tracking

None. No principle is being violated, so no justification is owed.

## Risks

| Risk | Mitigation |
|---|---|
| Mobile app breaks for existing users | Documented: bridge now needs a token and an acknowledged bind or a tunnel. `mobile/README.md` and `DEMO.md` updated with the new startup line. |
| Feature-gating mock breaks downstream test harnesses | `cfg(any(test, feature = "mock"))` keeps in-crate tests working; `--features mock` documented. |
| Socket path change breaks running deployments | Explicit `ipc_socket_path` in config still wins; only the *fallback* changes. `install.sh` writes the new path. |
| `default-members` hides bridge from CI lint | `--workspace` still covers it; CI job retained and relabelled. |
