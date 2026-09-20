# Spec backlog

Spec-kit arrived with issue #53 and produced exactly one spec. Everything else
in this repository predates both it and the constitution. This file is the
inventory of what needs specifying, ranked, so that specs accrete deliberately
rather than only where someone happened to file an issue.

It exists because **CI cannot enforce what is not written down**, and three of
the constitution's seven principles currently have nothing asserting them.

Tracking issue: [#59](https://github.com/chippr-robotics/sigil/issues/59).

## Conventions

- Specs live in `specs/<NNN>-<slug>/`, created by
  `.specify/scripts/bash/create-new-feature.sh`.
- **`NNN` is sequential and independent of issue and PR numbers.** It counts
  specs, nothing else. A spec links its issue in its header instead. See the
  constitution's Development Workflow section.
- A spec precedes implementation for any change touching the TCB boundary.
- One spec per coherent behaviour, not per crate and not per file. Where the
  grouping is genuinely unclear it is marked below rather than guessed at.

## Status legend

| Mark | Meaning |
| :-: | --- |
| ✅ | Spec written |
| 🟡 | Source document exists; needs conversion to a spec with requirement IDs |
| ⬜ | Nothing written |

---

## The finding that should drive ranking

`crates/sigil-daemon/src/signer.rs` is **478 lines with zero tests.**

`Signer::sign()` is the entire physical-consent enforcement path:

```
load_full_disk()      re-read the disk from the block device
get_next_presig()     take an unused presignature share
complete_signature()  combine cold + agent shares
mark_presig_used()    burn it
write_disk()          persist the burn
```

Every claim Sigil makes about itself lives in that function, and nothing
asserts any of it. The eight daemon tests added in #57 cover socket
permissions and config defaults — the transport around the signer, not the
signer.

This is not a coverage statistic. It is the product's central claim being
unverified, and it sets the order below.

## Coverage today

| Crate | LOC | Tests | Note |
| --- | ---: | ---: | --- |
| `sigil-mother` | 7,828 | 84 | Best-covered TCB crate |
| `sigil-mother-tui` | 6,801 | 8 | Largest crate, thinnest ratio |
| `sigil-mcp` | 5,164 | 86 | Well covered |
| `sigil-frost` | 4,122 | 25 | Threshold crypto |
| `sigil-mother-zkvm` | 3,465 | 35 | |
| `sigil-core` | 3,236 | 64 | Disk format, primitives |
| `sigil-daemon` | 2,795 | 10 | 8 of these arrived in #57 |
| `sigil-cli` | 858 | **0** | Operator signing path |
| `sigil-zkvm` | 654 | 1 | |
| `sigil-tests` | 3 | 7 | E2E |

Largest TCB modules with **zero** tests:

| Module | LOC | What it does |
| --- | ---: | --- |
| `sigil-mother-tui/src/app/mod.rs` | 548 | TUI event dispatch |
| `sigil-mother/src/ceremony.rs` | 517 | Key generation ceremony |
| **`sigil-daemon/src/signer.rs`** | **478** | **Physical-consent enforcement** |
| `sigil-mother/src/main.rs` | 440 | Mother CLI entry |
| `sigil-daemon/src/ipc/server.rs` | 330 | IPC dispatch, handles `Sign` |
| `sigil-cli/src/tools.rs` | 328 | |
| `sigil-core/src/types.rs` | 270 | Core type invariants |
| `sigil-cli/src/client.rs` | 272 | |
| `sigil-cli/src/commands.rs` | 220 | |

---

## Backlog

Ranked. Priority reflects TCB criticality multiplied by how unverified the
area currently is, not size.

### P1 — the central claim

| # | Feature | Crate | Source doc | Tests | Spec |
| :-: | --- | --- | --- | ---: | :-: |
| 1 | **Physical-consent enforcement** — disk re-read per signature, presig consumption, burn-on-use, fail-closed, rollback detection | `sigil-daemon` | — | 13 | ✅ [`002`](002-physical-consent-enforcement/) |
| 2 | **Presignature lifecycle** — generation, allocation, exhaustion, double-spend prevention | `sigil-core`, `sigil-mother` | `CRYPTO_SPEC.md` | partial | 🟡 |
| 3 | **Daemon IPC protocol** — 8 operations, dispatch, error surface | `sigil-daemon` | `proto/signer.proto` (design note only) | **0** | ⬜ |
| 4 | **Operator CLI** — 8 subcommands | `sigil-cli` | — | **0** | ⬜ |

### P2 — cryptography

| # | Feature | Crate | Source doc | Tests | Spec |
| :-: | --- | --- | --- | ---: | :-: |
| 5 | Disk format and versioning | `sigil-core` | `CRYPTO_SPEC.md` | 64 | 🟡 |
| 6 | ECDSA secp256k1 signing path | `sigil-core`, `sigil-daemon` | `CRYPTO_SPEC.md` | partial | 🟡 |
| 7 | FROST across three curves + DKG ceremony | `sigil-frost` | `FROST.md` | 25 | 🟡 |
| 8 | Timing-safety requirements | `sigil-core`, `sigil-frost` | `TIMING_SAFETY.md` | ⬜ | 🟡 |
| 9 | zkVM proof generation and verification | `sigil-zkvm`, `sigil-mother-zkvm` | `ZKVM_PROOFS.md` | 36 | 🟡 |

> **Open question — granularity.** Item 7 may be one spec or three. The three
> curves share a ceremony and a threshold scheme, so one spec avoids
> triplicating requirements with the curve name swapped. Decide by writing it,
> not in advance.

### P3 — mother device and operations

| # | Feature | Crate | Source doc | Tests | Spec |
| :-: | --- | --- | --- | ---: | :-: |
| 10 | Key generation ceremony | `sigil-mother` | `GENESIS_OPERATIONS.md` | **0** | 🟡 |
| 11 | Child disk creation and refill | `sigil-mother` | `GENESIS_OPERATIONS.md` | 8 | 🟡 |
| 12 | Reconciliation and anomaly detection | `sigil-mother` | — | partial | ⬜ |
| 13 | Nullification and revocation | `sigil-mother` | — | 6 | ⬜ |
| 14 | Hardware signer abstraction (ledger / trezor / pkcs11) | `sigil-mother` | — | 3 | ⬜ |
| 15 | Mother TUI flows | `sigil-mother-tui` | `SIGIL_MOTHER_TUI_PLAN.md` (retire) | 8 | ⬜ |
| 16 | Recovery procedures | — | `RECOVERY.md` | n/a | 🟡 |

### P4 — agent-facing and cross-cutting

| # | Feature | Crate | Source doc | Tests | Spec |
| :-: | --- | --- | --- | ---: | :-: |
| 17 | MCP server and its 5 tools | `sigil-mcp` | `MCP_INTEGRATION_PLAN.md` (retire) | 86 | ⬜ |
| 18 | Threat model | — | `THREAT_MODEL.md` | n/a | 🟡 |
| 19 | Install and system integration | `scripts/` | — | ⬜ | ⬜ |
| 20 | Versioning and release | — | `VERSIONING.md` | n/a | 🟡 |
| — | TCB quarantine | several | — | 40 | ✅ [`001-tcb-quarantine`](001-tcb-quarantine/) |

---

## Documentation triage

Ten files in `documentation/` function as specs. They predate the
constitution, carry no requirement IDs, and nothing traces to them.

| File | Lines | Disposition |
| --- | ---: | --- |
| `CRYPTO_SPEC.md` | 372 | **Convert** — feeds specs 2, 5, 6 |
| `FROST.md` | 708 | **Convert** — feeds spec 7 |
| `THREAT_MODEL.md` | 705 | **Convert** — feeds spec 18 |
| `ZKVM_PROOFS.md` | 373 | **Convert** — feeds spec 9 |
| `TIMING_SAFETY.md` | 203 | **Convert** — feeds spec 8 |
| `GENESIS_OPERATIONS.md` | 1,071 | **Keep as runbook**, extract requirements into specs 10, 11 |
| `RECOVERY.md` | 314 | **Keep as runbook**, extract requirements into spec 16 |
| `E2E_TEST_PLAN.md` | 1,540 | **Reconcile** — see below |
| `MCP_INTEGRATION_PLAN.md` | 1,234 | **Retire** — see below |
| `SIGIL_MOTHER_TUI_PLAN.md` | 1,248 | **Retire** — see below |

### The two plans are actively misleading

`MCP_INTEGRATION_PLAN.md` has **35 checkboxes, 0 checked.**
`SIGIL_MOTHER_TUI_PLAN.md` has **58 checkboxes, 0 checked.**

Both describe work that shipped. `sigil-mcp` is 5,164 lines with 86 tests;
`sigil-mother-tui` is 6,801 lines. A contributor reading either document would
conclude the component does not exist.

Converting a completed plan into a spec produces a document describing what
someone intended in the past, which is worse than no spec. Retire both — move
to `documentation/history/` or delete, with the shipped behaviour captured in
specs 15 and 17 written from the code.

### The E2E plan is a backlog, not history

`E2E_TEST_PLAN.md` describes **58 scenarios**; `sigil-tests` implements
**7**. Unlike the two above it is not stale — it is aspirational and largely
unbuilt. Reconcile it into spec acceptance criteria rather than retiring it,
so the 51 unimplemented scenarios become traceable requirements instead of
prose.

---

## Constitution conformance

What actually fails when a principle is violated:

Executable assertions live in
[`crates/sigil-tests/tests/constitution_conformance.rs`](../crates/sigil-tests/tests/constitution_conformance.rs).

| Principle | Enforced? | By what |
| --- | :-: | --- |
| I. One TCB | ✅ | `p1_default_members_equals_members`, `p1_out_of_tcb_crates_are_unpublished` |
| II. No signature without physical consent | ✅ | Mock-signing refusal, plus 13 signer tests including disk-rollback detection ([`002`](002-physical-consent-enforcement/)) |
| III. Default deny at the network edge | ✅ | `p3_no_crate_depends_on_an_http_server` |
| IV. Key material off convenience transports | 🟡 | The P-III test removes the transport; a direct assertion on shard paths is still to write |
| V. Supported install is auditable | ✅ | `p5_install_script_refuses_pipe_execution`, `p5_install_script_guard_admits_real_file_execution`, `p5_no_document_instructs_piping_into_a_shell` |
| VI. No outbound path from the air-gapped side | ✅ | `p6_no_knowledge_base_sync_tooling` |
| VII. Least privilege on the local machine | ✅ | `sigil-daemon` IPC tests — socket mode, dir mode, non-`/tmp` path (#57) |

Every assertion was negative-tested: each was shown to fail, naming the exact
violation, before being committed. An assertion nobody has seen fail is the
same category of object as a `Security Audit` job that cannot fail.

`p5_no_document_instructs_piping_into_a_shell` caught a real instance on its
first run — `docs/docs/zkvm-proofs.html` documented SP1's toolchain install as
`curl -L https://sp1.succinct.xyz | bash`. #57's sweep had grepped for
`| sudo bash` and missed it. Now documented as download, read, then run.

**P-II is asserted.** Spec [`002`](002-physical-consent-enforcement/) covers
the signer with 13 tests: fail-closed without a disk, fail-closed on removal,
fresh re-read per operation, exhaustion, R-point mismatch, burn-on-use
persisted to disk, distinct index per signature, one usage-log entry per
signature, and disk-rollback detection.

Writing the spec surfaced a vulnerability, since fixed. The agent-side
consumption mark was maintained and never consulted, so a restored disk image
could re-spend a presignature: same nonce, two messages, private key
recoverable from the pair. `Signer::sign` now refuses an index below the mark,
and refill resets the mark by enforcement at import rather than trusting the
imported payload. Both fixes were negative-tested.

**All seven principles are now executable.** What remains is breadth — each
backlog item below still needs its spec — not a hole in the constitution.

**P-IV** is partially covered as a side effect: key material cannot travel over
a convenience transport that does not exist. A direct assertion — that no
shard-carrying type crosses a serialization boundary outside the mother tooling
— is worth writing once the specs name those types.

> **Naming note.** `sigil-mcp/src/invariants/` already exists and is *input
> validation* — hex strings, chain IDs, URIs — not constitution conformance.
> The conformance suite needs a different home, `sigil-tests` being the
> natural one, to avoid the collision.

---

## Sequencing

Not one pull request. The inventory is one; each spec is its own; the
conformance suite is its own. A single PR touching every spec is unreviewable,
which defeats the purpose.

Suggested order:

1. ~~This file.~~ Done.
2. ~~Conformance assertions for P-I, P-V, P-VI.~~ Done — and P-III as well.
3. ~~Spec 1 (physical-consent enforcement) with the tests it implies.~~ Done —
   [`002`](002-physical-consent-enforcement/), 13 tests, FR-014 fixed.
4. Spec 4 (`sigil-cli`) with tests, since it is at zero.
5. Retire the two stale plans.
6. Everything else, by priority.
