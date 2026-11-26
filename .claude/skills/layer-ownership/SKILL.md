---
name: layer-ownership
description: Place Dash changes in the correct system layer, crate, or module. Use when planning or implementing code and during PR reviews to check architecture, ownership, dependency direction, duplicated validation, business logic in bindings, and implementation details leaking to API clients; keep review output limited to placement.
---

# Layer Ownership

Use during planning, implementation, and PR reviews. Treat every repository in
`dashpay` as an owned layer. Reviews are read-only unless the user requests
changes or comments.

## Workflow

1. Read `book/src/contributing/coding-conventions.md` completely from the `dashpay/platform` checkout.
2. For planning, map each planned responsibility to its owner before naming
   files. Inspect only the minimal existing boundary, callers, dependency
   manifests, and owned `dashpay` repositories needed to settle placement.
3. During implementation, apply that placement and recheck the actual changed
   paths before handoff.
4. For a PR, resolve exact base and head SHAs, then classify every changed path.
   Report only misplaced responsibilities, not unrelated issues.

Do not rescan unchanged subsystems. The conventions are authoritative.

## Implementation details stay behind the API

`rs-sdk` and `rs-platform-wallet` give their guarantees on their own; apps only
call, query, and display. For every public input, output, and callback, ask
what a client does with it. If it must track, re-derive, or coordinate
something the library knows (a revision, an in-flight operation, a contract
snapshot, a call-order rule), the responsibility has leaked. Move that state
and decision to its owner (`rs-platform-wallet` for orchestration and
persistence, `rs-sdk` for transport and contract or version resolution) and
remove the parameter rather than documenting it.

## Output

For every item, explicitly name current and target owner: repository,
crate/package, and module (current: `new` for new code). For PRs, lead with
`pass` or `changes requested`; each finding gives the file/line, misplaced
responsibility, and smallest move; a leak finding also names the knowledge
forced on the client. State reviewed SHAs and unavailable evidence.
