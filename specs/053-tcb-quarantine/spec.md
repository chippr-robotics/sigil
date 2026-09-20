# Feature Specification: Quarantine the Second TCB

**Feature Branch**: `053-tcb-quarantine`

**Created**: 2026-09-20

**Status**: Draft

**Tracking Issue**: [#53](https://github.com/chippr-robotics/sigil/issues/53)

**Input**: User description: "Air-gap + physical consent is the product. sigil-bridge POST /api/sign, the Logseq 'mother node' skill, and curl | sudo bash are a second TCB. Delete bridge from default members or bind it to localhost and mark it out of TCB. Move Logseq out of the key repo. Supported install is cargo install, not sudo-pipe."

---

## Problem

Sigil's security claim is that a signature cannot exist without a physically
present floppy disk. The cryptographic core honours that claim: `sigil-daemon`
re-reads the disk from the block device on every signing operation and fails
closed with `NoDiskDetected`.

The claim is nonetheless false in practice, because the repository ships four
surfaces that reach the signer without going through physical consent, or that
weaken the air gap the consent model depends on:

1. **`sigil-bridge`** is a first-class workspace member that binds `0.0.0.0:8080`
   by default, applies `CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)`,
   and exposes unauthenticated `POST /api/sign`, `POST /api/sign-frost`,
   `POST /api/import-agent-shard`, and `POST /api/import-child-shares`. Any host
   on the LAN — and any web page open in any browser on that LAN, via the
   wildcard CORS policy — can drain presignatures from an inserted disk or push
   attacker-chosen shard material into the daemon's agent store. It is built by
   `cargo build --workspace` and released by CI.

2. **`sigil-mcp --mock`** returns a hard-coded 65-byte hex string from
   `DaemonClient::sign()` with a fabricated `proof_hash`, on the same response
   path as a real signature. `McpServerState::default()` selects mock mode. A
   caller cannot distinguish a mock signature from a real one, and the mock code
   is compiled into every release binary.

3. **The Logseq skill** (`.claude/skills/logseq/`) lives in the key repository
   and its `sigil-mother-node.md` example instructs the operator to merge
   mother-device material into a networked, indexed knowledge graph. This is an
   outbound path from the air-gapped side, documented as a feature.

4. **`curl -sSL … | sudo bash`** is the headline install in `README.md`, in
   `scripts/install.sh`'s own header, and in `docs/docs/recovery.html`. It asks
   operators of a key-custody product to execute unread, unpinned root code
   fetched over the network.

A fifth issue is in scope because it is the same class one hop earlier: the
daemon binds its IPC socket at `/tmp/sigil.sock` with default umask
permissions, in a world-writable directory. Any local user can connect and
sign; any local user can squat the path before the daemon starts.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The Bridge Cannot Be Reached From the Network (Priority: P1)

An operator runs an agent device with a Sigil disk inserted. An attacker on the
same LAN, or a malicious web page open in a browser on that LAN, attempts to
drive `/api/sign` and `/api/import-agent-shard`.

**Why this priority**: This is the live, remotely reachable signing oracle. It
is the only item in this issue that an unauthenticated third party can exploit
today with a single HTTP request.

**Independent Test**: Start the bridge with no arguments, confirm it listens on
loopback only, confirm an unauthenticated `POST /api/sign` is rejected, and
confirm the shard-import routes return 404.

**Acceptance Scenarios**:

1. **Given** the bridge started with no arguments, **When** it binds, **Then** it
   binds `127.0.0.1` and no non-loopback interface accepts a connection.
2. **Given** the bridge is running, **When** a request arrives at `/api/sign`
   without a valid bearer token, **Then** the response is `401` and no IPC call
   is made to the daemon.
3. **Given** the bridge is running, **When** a request arrives at
   `/api/import-agent-shard` or `/api/import-child-shares`, **Then** the route
   does not exist.
4. **Given** an operator passes `--host 0.0.0.0` without acknowledgement,
   **When** the bridge starts, **Then** it refuses to start and explains what
   acknowledgement is required.
5. **Given** a browser page from `https://evil.example`, **When** it issues a
   cross-origin request to the bridge, **Then** no permissive CORS headers are
   returned and the browser blocks the response.
6. **Given** a developer runs `cargo build` or `cargo test` at the workspace
   root, **When** the build completes, **Then** `sigil-bridge` was not built.

---

### User Story 2 - A Mock Never Produces a Signature (Priority: P1)

An agent integrator wires `sigil-mcp` into a client and, deliberately or by
copy-paste, runs it in mock mode. A downstream system broadcasts what it
believes is a signed transaction.

**Why this priority**: A fabricated signature returned on the real response
path is a direct falsification of the product's central claim, and it ships in
every release binary today.

**Independent Test**: Invoke the `sign_evm` tool against a mock-mode server and
assert an error is returned; build the release binary and assert `--mock` is not
accepted.

**Acceptance Scenarios**:

1. **Given** a mock-mode MCP server, **When** `sign_evm` or `sign_frost` is
   called, **Then** an error is returned and no signature-shaped value appears
   in the response.
2. **Given** a mock-mode MCP server, **When** `check_disk` or the presignature
   count is requested, **Then** fabricated status is still returned, because
   status is not consent.
3. **Given** a binary built from default features, **When** `--mock` is passed,
   **Then** the flag is unrecognised.
4. **Given** `McpServerState` is constructed by its `Default` impl outside of
   tests, **Then** that construction is not available in a default-feature
   build.

---

### User Story 3 - The Air-Gapped Side Has No Documented Outbound Path (Priority: P2)

An operator follows repository documentation to set up their knowledge
management. Nothing in the key repository instructs them to index mother-device
material into a networked system.

**Why this priority**: No remote attacker triggers this; it requires the
operator to follow the instructions. But the instructions are in the key repo
and carry its authority.

**Independent Test**: Search the repository for Logseq integration assets and
mother-node merge instructions; confirm none remain and that the removal is
explained.

**Acceptance Scenarios**:

1. **Given** a checkout of the repository, **When** it is searched for Logseq
   skill assets, **Then** none are present.
2. **Given** an operator looks for the removed skill, **When** they read the
   changelog and security documentation, **Then** they find why it was removed
   and that its history is preserved in git.

---

### User Story 4 - The Supported Install Is Readable Before It Runs (Priority: P2)

A new operator installs Sigil. The documented path lets them see the source and
pin the version before any code executes as root.

**Why this priority**: Supply-chain exposure at install time compromises
everything downstream of it, but it is a one-time window rather than a standing
listener.

**Independent Test**: Follow `README.md`'s installation section end to end and
confirm no step pipes network content into a shell; pipe `install.sh` into bash
and confirm it refuses.

**Acceptance Scenarios**:

1. **Given** `README.md`, **When** an operator reads the install section,
   **Then** the primary instruction is `cargo install --locked` from a pinned
   source, or a cloned checkout.
2. **Given** `scripts/install.sh` is piped into `bash` from `curl`, **When** it
   starts, **Then** it exits non-zero with instructions to clone and inspect it
   first.
3. **Given** the repository documentation, **When** it is searched for
   `| sudo bash`, **Then** no instance instructs an operator to run one.

---

### User Story 5 - Local IPC Is Not Open to Every Local User (Priority: P2)

Two users share an agent machine. The second user attempts to connect to the
daemon's IPC socket and sign.

**Why this priority**: Real, but requires prior local access, so it ranks below
the network-reachable bridge.

**Independent Test**: Start the daemon, stat the socket, and confirm mode and
parent-directory ownership restrict access.

**Acceptance Scenarios**:

1. **Given** the daemon has bound its IPC socket, **When** the socket is
   stat'ed, **Then** its mode grants no access to `other`.
2. **Given** no `XDG_RUNTIME_DIR` is set, **When** the daemon resolves its
   default socket path, **Then** the path is not inside a world-writable
   directory.
3. **Given** the socket's parent directory does not exist, **When** the daemon
   starts, **Then** it creates the directory with restrictive permissions
   before binding.

---

### Edge Cases

- Operator intentionally needs LAN access for the mobile app: acknowledgement
  flag plus a token must make this possible, loudly, without editing source.
- Token not supplied: the bridge must generate one, persist it with
  owner-only permissions, and tell the operator where it is — never fall back
  to no authentication.
- Existing deployments whose config pins `/tmp/sigil.sock`: an explicit
  configured path must still be honoured, with the hardening applied to it.
- `cargo test --workspace` in CI: excluding the bridge from default members
  must not silently stop it from ever being compiled; it must still be built by
  an explicit, clearly labelled job or command.
- Mobile app and its documentation point at `192.168.1.100:8080`: docs must be
  corrected, or the app's supported configuration becomes impossible to follow.

## Requirements *(mandatory)*

### Functional Requirements

**Bridge quarantine**

- **FR-001**: `sigil-bridge` MUST be excluded from the workspace's
  `default-members`, so a bare `cargo build`/`cargo test` at the root does not
  build it.
- **FR-002**: `sigil-bridge` MUST declare `publish = false` and carry an
  out-of-TCB banner in both its crate-level documentation and its README.
- **FR-003**: The bridge's default bind host MUST be `127.0.0.1`.
- **FR-004**: The bridge MUST refuse to start on a non-loopback address unless
  the operator supplies an explicit acknowledgement, and MUST log a warning
  naming the risk when it does.
- **FR-005**: The bridge MUST NOT apply a permissive CORS policy. Cross-origin
  access MUST be opt-in per explicit origin.
- **FR-006**: Every `/api/*` route MUST require a bearer token. Missing or
  incorrect tokens MUST yield `401` before any daemon IPC occurs.
- **FR-007**: The bridge MUST generate a token when none is configured, persist
  it with owner-only permissions, and report its location on startup.
- **FR-008**: The bridge MUST NOT expose any route that transports shard or key
  material. `/api/import-agent-shard` and `/api/import-child-shares` MUST be
  removed.
- **FR-009**: `/health` MAY remain unauthenticated and MUST NOT disclose disk
  state, presignature counts, addresses, or child identifiers.

**Mock signing**

- **FR-010**: Mock mode MUST return an error from every signing operation.
- **FR-011**: Mock mode MAY continue to return fabricated disk status and
  presignature counts.
- **FR-012**: Mock support MUST sit behind a non-default cargo feature, so the
  default-feature binary cannot construct a mock signer.
- **FR-013**: The `--mock` CLI flag MUST exist only in builds with that feature
  enabled.

**Logseq**

- **FR-014**: All Logseq skill assets MUST be removed from this repository.
- **FR-015**: The removal MUST be recorded with its rationale in `CHANGELOG.md`
  and in security documentation.

**Install**

- **FR-016**: `README.md` MUST present `cargo install --locked` from a pinned
  source, or a cloned checkout, as the supported install.
- **FR-017**: `scripts/install.sh` MUST detect that it is being executed from a
  pipe rather than a file on disk and MUST exit non-zero in that case.
- **FR-018**: No documentation in the repository may instruct an operator to
  pipe network content into a privileged shell.

**Local IPC**

- **FR-019**: The daemon MUST set explicit permissions on its Unix IPC socket
  after binding, granting no access to `other`.
- **FR-020**: The daemon's default socket path MUST NOT fall back to a
  world-writable directory.
- **FR-021**: The daemon MUST create its socket's parent directory with
  restrictive permissions when it is absent.

**Boundary documentation**

- **FR-022**: `SECURITY.md` MUST state the TCB membership list and name the
  out-of-TCB components explicitly.

**CI gates** *(added after review feedback: the checks must be worth having)*

- **FR-023**: The CI test job MUST run the whole workspace, not a
  hand-maintained list of crates.
- **FR-024**: The security-audit job MUST fail the build on an advisory
  reachable from a default build. Exceptions MUST be declared individually,
  in-repo, each with a written reason.
- **FR-025**: CI jobs MUST NOT hold write access to repository contents, and
  MUST NOT push commits to the branch under test.
- **FR-026**: CI MUST run for pull requests targeting every integration branch,
  `staging` included.
- **FR-027**: Unused dependencies MUST NOT remain declared in TCB crates.

### Key Entities

- **TCB**: `sigil-core`, `sigil-frost`, `sigil-zkvm`, `sigil-daemon`,
  `sigil-cli`, `sigil-mother`, `sigil-mother-tui`, `sigil-mother-zkvm`.
- **Out of TCB**: `sigil-bridge`, `mobile/`, mock modes, demo harnesses.
- **Bridge token**: an operator-supplied or auto-generated shared secret
  authenticating `/api/*` callers; not key material and not a substitute for
  physical consent.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A default `cargo build` at the repository root produces zero
  binaries that listen on a network socket.
- **SC-002**: An unauthenticated HTTP request to any bridge signing route is
  rejected with `401` in 100% of cases, verified by test.
- **SC-003**: Zero routes accepting shard or key material remain in the bridge,
  verified by test.
- **SC-004**: Signing through any mock path returns an error in 100% of cases,
  verified by test.
- **SC-005**: Zero occurrences of `| sudo bash` as an instruction remain in
  repository documentation, verified by grep.
- **SC-006**: Zero Logseq skill assets remain in the repository, verified by
  grep.
- **SC-007**: The daemon's IPC socket grants no permissions to `other`,
  verified by test.
- **SC-008**: `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test` pass.
- **SC-009**: Every crate's tests run in a job whose failure fails CI, verified
  by the absence of any per-crate allowlist in the test job.
- **SC-010**: `cargo audit` exits non-zero when an advisory is not explicitly
  ignored, verified by running it with an empty ignore list.
- **SC-011**: Zero CI jobs declare `contents: write`, verified by inspection of
  `.github/workflows/ci.yml`.
- **SC-012**: Zero unused dependencies remain declared in `sigil-daemon` or
  `sigil-cli`, verified by grep for their symbols across the crate sources.

## Assumptions

- The mobile app remains an out-of-TCB convenience client. Operators who want
  it keep using the bridge, now on loopback with a token, reaching it over a
  tunnel or an explicit acknowledged bind. Rewriting the mobile transport is
  out of scope.
- `scripts/install.sh` continues to exist for system integration (udev rules,
  systemd unit, `sigil` group). Only its network-pipe invocation is removed.
- The Logseq skill's content is preserved in git history; relocating it to a
  separate repository is the operator's action, not part of this change.
- The daemon's disk-presence enforcement is already correct and is not modified.
- Bearer-token authentication is a coarse gate on an out-of-TCB surface, not a
  security boundary the product's claims rest on. Physical consent remains the
  boundary.
