# Historical documents

Plans for work that has since shipped. They are kept for the reasoning they
record, not as descriptions of the system.

**Do not read these as current.** Both carry unchecked task lists for
components that exist and are tested:

| Document | Claims | Reality |
| --- | --- | --- |
| `MCP_INTEGRATION_PLAN.md` | 35 checkboxes, none checked | `sigil-mcp` is 5,164 lines with 86 tests |
| `SIGIL_MOTHER_TUI_PLAN.md` | 58 checkboxes, none checked | `sigil-mother-tui` is 6,801 lines |

A contributor reading either in `documentation/` would conclude the component
had not been built. That is why they moved here rather than being converted
into specs: a completed plan turned into a spec describes a past intention
while reading as current, which is worse than having no spec at all.

The shipped behaviour of both components is specified from the code instead —
see [`specs/README.md`](../../specs/README.md), backlog items 15 and 17.

`E2E_TEST_PLAN.md` deliberately stayed in `documentation/`. It describes 58
scenarios against 7 implemented, so it is aspirational rather than stale, and
it is being reconciled into spec acceptance criteria.
