---
name: layer-ownership
description: Check whether each PR change belongs in the correct Dash system layer, crate, or module. Use during PR reviews focused on architecture, ownership, dependency direction, duplicated validation, or business logic in bindings; do not expand into general correctness, style, security, or test review.
---

# Layer Ownership

Review only whether each changed responsibility is in the correct system layer,
crate, and module. Treat every repository in `dashpay` as an owned layer.
Reviews are read-only unless the user requests changes or comments.

## Workflow

1. Resolve exact base and head SHAs.
2. Read `book/src/contributing/coding-conventions.md` completely.
3. Inspect the changed paths and diff. Do not rescan unchanged subsystems;
   inspect only the minimal callers, dependency manifests, and owned `dashpay`
   repositories needed to settle uncertain placement.
4. Classify every changed path against the conventions, but report only
   misplaced responsibilities. The conventions are authoritative.

## Output

Lead with `pass` or `changes requested`. For each finding give the changed
file/line, misplaced responsibility, correct owner, and smallest move. State
the reviewed SHAs and any unavailable evidence. Do not report unrelated issues.
