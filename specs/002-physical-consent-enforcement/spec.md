# Feature Specification: Physical-Consent Enforcement

**Feature Branch**: `002-physical-consent-enforcement`

**Created**: 2026-09-20

**Status**: Draft

**Tracking Issue**: [#59](https://github.com/chippr-robotics/sigil/issues/59) — backlog item 1

**Constitution**: Principle II (NON-NEGOTIABLE)

---

## Why this spec exists first

`crates/sigil-daemon/src/signer.rs` was 478 lines with zero tests. Its only
test module was a placeholder reading:

```rust
#[cfg(test)]
mod tests {
    // Integration tests would require full setup with disk and agent store
}
```

`Signer::sign()` is where Sigil's claim is either true or false. Everything
else — the disk format, the threshold crypto, the mother tooling — exists to
make that function's behaviour meaningful. It was the least-verified critical
code in the repository, and Principle II was the only constitution principle
with nothing asserting it.

## The path under specification

```
Signer::sign(request)
 1. load_full_disk()          re-read the block device; fail closed if absent
 2. disk.validate(now)        reject expired or structurally invalid disks
 3. disk.get_next_presig()    take the next unspent cold share
 4. agent_store.get_presig_share(child, index)
 5. cold.r_point == agent.r_point ?   halves must be from the same presignature
 6. complete_signature(...)   combine; verify against the child pubkey
 7. disk.mark_presig_used()   burn
 8. usage_log.push(entry)     record for reconciliation
 9. write_disk()              persist the burn
10. agent_store.mark_presig_used()
```

## Requirements

### Fail closed

- **FR-001**: Signing MUST fail when no disk is physically present.
- **FR-002**: The disk MUST be re-read from the block device on every signing
  operation. A cached copy MUST NOT be trusted.
- **FR-003**: Signing MUST fail when the disk is removed between operations.
- **FR-004**: Signing MUST fail when the disk is expired or fails validation.
- **FR-005**: Signing MUST fail when no unspent presignature remains.
- **FR-006**: Signing MUST fail when the cold and agent halves of a
  presignature disagree on their R point.
- **FR-007**: A failed signing attempt MUST NOT consume a presignature.

### Burn on use

- **FR-008**: A successful signature MUST mark its presignature used.
- **FR-009**: The burn MUST be persisted to the disk before the call returns,
  not held only in memory.
- **FR-010**: No two signatures may consume the same presignature index. A
  repeated index means a reused ECDSA nonce, which discloses the private key.
- **FR-011**: Each signature MUST append exactly one usage-log entry carrying
  the presignature index and the operator's description.
- **FR-012**: The signature MUST verify against the child public key before it
  is returned.

### Agent-side consumption — **currently not enforced**

- **FR-013**: The agent store MUST record that a presignature was consumed.
  *Satisfied today* — `mark_presig_used` advances `next_presig_index`.
- **FR-014**: The signing path MUST refuse a presignature index the agent side
  has already recorded as consumed. **Not satisfied.** See below.

## Finding: disk rollback is not detected

`AgentChildData::next_presig_index` is written by `AgentStore::mark_presig_used`
and **read nowhere outside `agent_store.rs`**. `get_presig_share(child, i)`
returns share `i` regardless of whether it has been spent.

The disk's burn is therefore the only thing preventing a presignature being
used twice — and a disk image restore defeats it:

1. Operator or attacker restores an earlier image of the child disk.
2. The restored disk offers a presignature index that was already spent.
3. `get_presig_share` serves the matching agent half without objection.
4. The same nonce `k` now signs two different messages.
5. Given two signatures sharing `r`, the private key follows from
   `d = (s₁·k − z₁)/r`, with `k = (z₁ − z₂)/(s₁ − s₂)`.

That is full key disclosure from a file-restore, and the check that would
prevent it already exists — the high-water mark is being maintained, it is
simply never consulted.

**Proposed fix (FR-014)**: in `Signer::sign`, after obtaining `presig_index`
from the disk, reject the operation when
`presig_index < agent_data.next_presig_index`.

**Why it is not in this change**: it alters TCB signing behaviour and could
reject legitimate flows — notably refill, which resets the disk's presignature
table and would need to reset or rebase the agent-side mark in the same
operation. That interaction needs deciding, not guessing. Raised for the
maintainer.

Reconciliation is a *detective* control for this: it compares usage logs
against expectations after the fact. FR-014 would be a *preventive* one.

## Success Criteria

- **SC-001**: Every requirement above except FR-014 is covered by a test in
  `crates/sigil-daemon/src/signer.rs`.
- **SC-002**: Signing without a physically present disk fails in 100% of
  cases, verified by test.
- **SC-003**: A rejected signing attempt leaves `presig_used` unchanged,
  verified by test.
- **SC-004**: Across N successful signatures, N distinct presignature indices
  are consumed, verified by test.
- **SC-005**: FR-014 is decided — implemented, or recorded as accepted risk
  with reconciliation named as the compensating control.

## Coverage

| Requirement | Test |
| --- | --- |
| FR-001 | `refuses_to_sign_without_a_disk` |
| FR-002, FR-003 | `refuses_to_sign_after_the_disk_is_removed_mid_session`, `the_disk_is_re_read_on_every_signing_operation` |
| FR-005 | `refuses_to_sign_once_presignatures_are_exhausted` |
| FR-006, FR-007 | `refuses_to_sign_when_cold_and_agent_shares_disagree` |
| FR-008, FR-009 | `a_successful_signature_burns_its_presignature_on_disk` |
| FR-010 | `every_signature_consumes_a_distinct_presignature` |
| FR-011 | `each_signature_appends_one_usage_log_entry` |
| FR-012 | Enforced in `complete_signature`; exercised by every successful-sign test |
| FR-013 | `the_agent_side_records_consumption_but_does_not_enforce_it` |
| FR-014 | **None — not implemented** |
| FR-004 | **None — expiry is covered in `sigil-core`; a signer-level test is still owed** |

## Test approach

Fixtures build cryptographically consistent presignature pairs, because
`complete_signature` verifies its own output with `verify_prehash` before
returning. For each index:

```
d   = chi_cold + chi_agent      the child private key
k   = k_cold   + k_agent        the per-signature nonce
R   = k · G                     the nonce commitment both halves carry
pub = d · G                     what the disk header carries
```

A `#[cfg(test)]` seam on `DiskWatcher` — `insert_disk_for_test` /
`remove_disk_for_test` — presents a disk file as inserted, since udev is not
available in a test process. It is `cfg(test)`, so a release binary has no way
to claim a disk is present when it is not. Same constraint as the mock-signing
gate in `sigil-mcp`.

## Assumptions

- ECDSA correctness is `sigil-core`'s to verify; this spec covers the signer's
  orchestration.
- zkVM proving is disabled in these tests. Proof generation is
  `specs/` item 9's subject.
- The disk format's own invariants (expiry, max uses, log integrity) are
  covered by `sigil-core` tests and the E2E suite.
