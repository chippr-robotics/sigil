# Feature Specification: Operator CLI

**Feature Branch**: `003-operator-cli`

**Created**: 2026-09-20

**Status**: Draft

**Tracking Issue**: [#59](https://github.com/chippr-robotics/sigil/issues/59) — backlog item 4

---

## Why

`sigil-cli` was 858 lines with **zero tests**, and it is the operator's signing
path — the thing a person actually runs to make a signature happen. Writing
this spec found two bugs, one of them introduced by [#57](https://github.com/chippr-robotics/sigil/pull/57).

## Scope

The eight subcommands (`status`, `disk`, `sign`, `update-tx`, `presig-count`,
`import-agent-shard`, `import-child-shares`, `list-children`), the daemon
connection, and the display formatting the operator reads.

Out of scope: the daemon's own behaviour, specified in
[`002-physical-consent-enforcement`](../002-physical-consent-enforcement/).

## Requirements

### Connection

- **FR-001**: The CLI's default socket path MUST resolve the same way the
  daemon's does, or the CLI cannot reach a default-configured daemon.
- **FR-002**: The default socket path MUST NOT be in a world-writable
  directory.
- **FR-003**: An explicitly supplied `--socket` path MUST be honoured.

### Fail closed

- **FR-004**: Every operation MUST return an error when no daemon is
  reachable. None may report success.
- **FR-005**: Connecting to an absent daemon MUST fail promptly rather than
  hanging.

### Argument handling

- **FR-006**: All eight subcommands MUST parse.
- **FR-007**: `sign` MUST require a message and MUST NOT default its usage-log
  description to an empty string — a blank audit entry is worse than none.
- **FR-008**: `import-agent-shard` MUST reject `--hex` and `--file` together;
  which material is authoritative would otherwise be ambiguous.
- **FR-009**: `import-child-shares` MUST NOT replace existing shares unless
  `--replace` is given.

### Display

- **FR-010**: Formatting a signing result MUST NOT panic for any signature
  value the daemon can return, including short, empty, and multi-byte.
- **FR-011**: A long signature MUST be elided rather than printed in full.
- **FR-012**: A failed result MUST show the error and MUST NOT resemble
  success.
- **FR-013**: An absent disk MUST tell the operator to insert one, without
  printing counts that do not exist.

## Findings

### 1. The CLI could not reach a default-configured daemon (fixed)

`#57` moved the daemon's default socket off world-writable `/tmp` to
`/run/sigil/sigil.sock`, updating `sigil-daemon` and `sigil-bridge` — and
missing `sigil-cli`, in two places:

- `commands.rs` — the `--socket` clap default
- `client.rs` — `SigilClient::new`'s fallback

So the CLI looked for the daemon where the daemon no longer listened. Worse,
it looked in a world-writable directory: any local user can create
`/tmp/sigil.sock` and receive the operator's signing requests.

**Fix**: one `DEFAULT_UNIX_SOCKET_PATH` constant in `client.rs`, used by both,
pinned by test to the daemon's value. The two halves live in crates that
cannot see each other's constants, so the pin is the only thing keeping them
honest.

### 2. The display could panic on the signature it was given (fixed)

`format_signing_result_for_display` did `&s[..18]`, which panics two ways:

```
byte index 18 is out of bounds of `0xabc`
byte index 18 is not a char boundary; it is inside '✓' (bytes 17..20)
```

Both are reachable — the shortening runs on whatever the daemon returned — and
a panic in the operator's signing path is a poor way to learn a signature was
malformed.

**Fix**: `truncate_for_display`, which counts characters rather than bytes.

## Not testable as written

`execute_sign_transaction` and `execute_check_disk` construct
`SigilClient::new()` internally, so they cannot be exercised without a live
daemon. Their validation logic — no disk, invalid disk, daemon absent — is
therefore untested here.

Injecting the client would fix it. That is a change to a TCB crate's public
shape for testability alone, so it is recorded rather than made. The
consequences are contained: the branches are short, and the daemon-side
equivalents *are* covered by spec 002.

## Success Criteria

- **SC-001**: Every requirement above is covered by a test.
- **SC-002**: No signature value causes the display to panic, verified across
  short, empty, multi-byte, and long inputs.
- **SC-003**: Every CLI operation errors without a daemon, verified by test.
- **SC-004**: The CLI and daemon default socket paths are equal, verified by
  test on both sides.

## Coverage

| Requirement | Test |
| --- | --- |
| FR-001 | `the_default_socket_path_matches_the_daemons` |
| FR-002 | `the_default_socket_path_is_not_world_writable`, `the_socket_flag_defaults_to_the_daemons_path` |
| FR-003 | `an_explicit_socket_path_is_honoured` |
| FR-004 | `every_operation_fails_closed_without_a_daemon` |
| FR-005 | `a_missing_daemon_is_an_error_not_a_hang` |
| FR-006 | `every_subcommand_parses`, `the_cli_definition_is_valid` |
| FR-007 | `sign_requires_a_message`, `sign_defaults_to_mainnet_with_an_audit_description` |
| FR-008 | `import_agent_shard_refuses_both_hex_and_file` |
| FR-009 | `import_child_shares_does_not_replace_unless_asked` |
| FR-010 | `a_short_signature_does_not_panic_the_display`, `a_multibyte_signature_does_not_panic_the_display`, `an_empty_signature_does_not_panic_the_display` |
| FR-011 | `a_long_signature_is_shortened_with_an_ellipsis` |
| FR-012 | `a_failed_result_shows_the_error_not_a_signature`, `a_failed_result_without_an_error_message_still_renders` |
| FR-013 | `an_absent_disk_asks_the_operator_to_insert_one`, `a_detected_disk_shows_its_remaining_presignatures_and_expiry`, `a_detected_disk_with_unknown_counts_renders_without_panicking` |

The panic fix was negative-tested: restoring `&s[..18]` reproduces both panic
modes.
