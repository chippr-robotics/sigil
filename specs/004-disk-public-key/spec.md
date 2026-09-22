# Feature Specification: Disk public key over IPC

**Feature Branch**: `004-disk-public-key`

**Created**: 2026-09-22

**Status**: Draft

**Tracking Issue**: FairWins [chippr-robotics/fairwins-platform#1633](https://github.com/chippr-robotics/fairwins-platform/issues/1633): Sigil as a cold signer for people

---

## Why

A Sigil disk signs for exactly one key: the child public key in its header, at
offset `0x002C`. Nothing outside the daemon could learn that key.

- `GetDiskStatus` reported counts, expiry and a short child id, but not the key.
- The MCP `get_address` tool fell back to a hardcoded placeholder. Its
  `public_key` field was a `TODO: Add public_key to daemon's DiskStatus`.

A client that cannot name the account cannot do the following:

- show a person which address their disk controls;
- check that a disk it has seen before is the same disk;
- recover the ECDSA recovery id (`v`) for a signature. The daemon returns
  `r||s` only, and `v` is found by recovering against the known key.

The only workaround was to spend a presignature on a throwaway signature and
recover the key from it. That burns consent material to answer a read-only
question.

The FairWins platform needs this key to offer Sigil to people as a cold signer
(Protect ▸ Off chain). Per `SECURITY.md` invariant 3, FairWins builds that
transport itself, outside this repository.

## Scope

One field added to one existing response. No new operation, no new socket, and
no new dependency.

### What moves across the TCB boundary

The child public key leaves the TCB. The key is **public by construction**:
every signature the disk produces verifies against it, and anyone holding one
signature and its digest can already recover it.

No presignature share, agent share or secret leaves the TCB. The interface
is otherwise unchanged. Constitution Principles I–IV are untouched: nothing
new listens, nothing new is imported, and nothing new can cause a signature.

## Requirements

- **FR-001** `DiskStatus` carries `child_pubkey` when a disk is present. The
  value is the 33-byte compressed secp256k1 key from the header, hex encoded
  without `0x`.
- **FR-002** With no disk present, `child_pubkey` is absent from the wire. It
  is never a placeholder a client could mistake for an account.
- **FR-003** A client reading a response from a daemon that predates this field
  still parses it, with the key unknown (`serde(default)`).
- **FR-004** `sigil disk` prints the public key when the daemon reports one, and
  `sigil-mcp` populates `DiskState.public_key` from it instead of leaving it
  empty.

## Coverage

| Requirement | Test |
| --- | --- |
| FR-001 | `disk_status_reports_the_child_public_key` |
| FR-002 | `disk_status_without_a_disk_reports_no_public_key` |
| FR-003 | `disk_status_from_a_daemon_without_the_field_still_parses` |
| FR-004 | Not separately tested. It is a one-line field mapping in `sigil-cli/src/client.rs`, `commands.rs` and `sigil-mcp/src/client.rs`; the compiler enforces the struct shape, and the mapping has no branch to exercise. |

## Known adjacent defects (not fixed here)

These are in `sigil-mcp`, which is outside the TCB list. They are tracked
separately so this change stays one field wide:

- `get_address` still returns hardcoded placeholder addresses for every format
  except `hex`. With FR-001 it can derive the EVM address instead of inventing
  one.
- `sign_evm` expects a 130-hex-character `r||s||v` from the daemon, which
  returns 128 (`r||s`). As written, that path always errors.
