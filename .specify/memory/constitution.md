# Sigil Constitution

**Version**: 1.0.0
**Ratified**: 2026-09-20
**Last Amended**: 2026-09-20

Sigil splits signing authority 2-of-2 between a cold shard on a physical floppy
disk and an agent shard on a networked device. The product is not "a signer" —
it is *physical consent for machine-initiated value transfer*. Every principle
below exists to keep that claim literally true.

## Core Principles

### I. One TCB, and It Contains a Floppy Disk (NON-NEGOTIABLE)

The trusted computing base is exactly: `sigil-core`, `sigil-frost`,
`sigil-zkvm`, `sigil-daemon`, `sigil-cli`, `sigil-mother`, `sigil-mother-tui`,
`sigil-mother-zkvm`.

Any component that can cause a signature to exist is in the TCB and is
governed by every principle here. A component that cannot must be
*structurally* unable to — not merely discouraged by documentation. Convenience
surfaces (HTTP bridges, mobile transports, note-taking integrations, demo
harnesses) are **out of TCB**, must be labelled as such in their crate docs and
README, must be excluded from the default build, and must not ship as part of a
supported install.

Adding a new crate to the TCB is a constitutional amendment, not a pull request.

### II. No Signature Without Physical Consent (NON-NEGOTIABLE)

Every signature is backed by a presignature share read fresh from a physically
present disk and consumed on use. There is no fallback, no cache, no
"development mode" that returns bytes shaped like a signature.

Mock, demo, and test modes may fabricate *status* (disk present, N presigs
remaining). They MUST NOT fabricate *signatures*. A caller that asks a mock for
a signature receives an error, never plausible-looking hex. A build that can
mock-sign MUST NOT be producible from the default feature set.

### III. Default Deny at the Network Edge

Nothing in the repository listens on a non-loopback interface by default.
Nothing accepts a cross-origin browser request by default. Nothing exposes a
signing or key-import operation without authenticating the caller.

Binding beyond loopback is an explicit, acknowledged, logged act by the
operator — never a default value in a struct.

### IV. Key Material Does Not Travel Over Convenience Transports

Shard import, shard export, and any operation that moves secret material
crosses the air gap by physical media or an in-TCB tool only. It is never an
endpoint on an HTTP server, never a body in a JSON request, never a field a
mobile app can populate.

### V. The Supported Install Is Auditable

The supported installation path lets an operator read what they are about to
run before it runs: `cargo install --locked` from a pinned source, or a cloned
checkout. `curl … | sudo bash` is not a supported install path and is not
documented as one. System integration scripts (udev, systemd, group creation)
may exist, but MUST refuse to execute when piped from stdin.

### VI. The Air-Gapped Side Has No Outbound Path

Tooling that indexes, syncs, merges, or mirrors mother-device material into a
networked system does not live in this repository. Documentation that shows an
operator how to do so is an exfiltration tutorial regardless of intent.

### VII. Least Privilege on the Local Machine

Local IPC endpoints are owner- or group-restricted at the filesystem layer, in
a directory the daemon controls. "Any local user can reach the signer" is the
same finding as "any network host can reach the signer", one hop earlier.

## Security Requirements

- Changes that widen an interface MUST state, in the PR body, what moves into
  or out of the TCB.
- CI checks MUST be gates. A job that reports a failure and exits zero, or that
  repairs the code it is checking and pushes the repair, is not a check. CI
  holds no write access to branches: a bot commit is code that reaches `main`
  without having been read by anyone.
- The test job MUST cover the whole workspace. A hand-maintained list of crates
  silently stops covering the crate added after it was last edited.
- Out-of-TCB components MUST carry a machine-checkable marker
  (`publish = false`, exclusion from `default-members`, a `# NOT IN TCB` README
  banner) so the boundary survives refactors.
- Defaults are part of the threat model. A safe option behind an unsafe default
  is an unsafe system.
- Regressions of Principles I–IV MUST be covered by a test, not a review
  convention.

## Development Workflow

- Specs live in `specs/<NNN>-<slug>/` and precede implementation for any change
  that touches the TCB boundary.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  and `cargo test` gate every change.
- Conventional commits: `<type>(<scope>): <description>`.

## Governance

This constitution supersedes convenience, roadmap pressure, and demo
requirements. Where a feature and a principle conflict, the feature ships
disabled or does not ship.

Amendments require an explicit version bump here and a note in `CHANGELOG.md`.
Compliance is reviewed at every PR that touches a TCB crate, a bind address, a
default feature set, or an install path.
