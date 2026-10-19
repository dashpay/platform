---
name: layer-ownership
description: Place Dash changes in the correct system layer, crate, or module. Use when planning or implementing code and during PR reviews to check architecture, ownership, dependency direction, duplicated validation, and business logic in bindings; keep review output limited to placement.
---

# Layer Ownership

Use during planning, implementation, and PR reviews. Treat every repository in
`dashpay` as an owned layer. Reviews are read-only unless the user requests
changes or comments.

## Workflow

1. Read `book/src/contributing/coding-conventions.md` completely.
2. For planning, map each planned responsibility to its owner before naming
   files. Inspect only the minimal existing boundary, callers, dependency
   manifests, and owned `dashpay` repositories needed to settle placement.
3. During implementation, apply that placement and recheck the actual changed
   paths before handoff.
4. For a PR, resolve exact base and head SHAs, then classify every changed path.
   Report only misplaced responsibilities, not unrelated issues.

Do not rescan unchanged subsystems. The conventions are authoritative.

## Output

For every item, explicitly name current and target owner: repository,
crate/package, and module (current: `new` for new code). For PRs, lead with
`pass` or `changes requested`; each finding gives the file/line, misplaced
responsibility, and smallest move. State reviewed SHAs and unavailable evidence.
