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

`rs-sdk` and `rs-platform-wallet` give their guarantees on their own. Apps
(Dash Evo Tool, dashwallet-ios, Swift/Kotlin SDKs, wasm) only call, ask, and
display. If a client must know an internal detail to stay correct, the
library has leaked it, even when the client already copes.

Leaks to design out while implementing and to report in review:

- **Internal state in the API**: a parameter, field, or callback that a client
  uses only to rebuild a protocol rule or the library's state. Examples: an
  identity revision to guess whether an update landed, an outpoint to match
  an in-flight asset lock, a nonce, or a contract snapshot.
- **Unwritten caller contracts**: "do not call X while Y runs" or "call A
  before B". The library enforces this itself, with a guard, a typed error, or
  by joining the call already in flight.
- **Client-side tracking**: the app records what the library sent, signed, or
  persisted in order to recover after a crash. The owner keeps that record
  itself, for example an outbox persisted through its normal changesets.
- **Client-supplied inputs the library can resolve**: contracts, protocol
  versions, keys, or levels. The library resolves each one once and uses the
  same snapshot for both its checks and the transition it builds.
- **Outputs passed back through inputs**: a result that a client reads back
  from a settings object it passed in. Return it from the call, using a named
  type that can grow (a wrapper, not a bare hash).
- **Inferred status**: a client guessing from timers or database rows whether
  a call is waiting or has hung. The owner exposes the state as a query or as
  a typed outcome, for example `Landed | NeverLands | Unknown`.

Test each public input, output, and callback: what does a client do with it?
If the answer is "track, re-derive, or coordinate something the library
already knows", it is a leak. Likewise, if a second client would have to
repeat the same logic to be correct, that logic belongs to the library. Fix a
leak by moving the state and the decision to their owner: orchestration and
persistence go to `rs-platform-wallet`; transport, hooks, and contract or
version resolution go to `rs-sdk`. Then remove the parameter instead of
documenting it. Changing `rs-sdk` to close a leak is preferred over pushing
the work to clients. Public API changes stay visible: no `#[non_exhaustive]`,
so downstream builds break and developers notice.

## Output

For every item, explicitly name current and target owner: repository,
crate/package, and module (current: `new` for new code). For PRs, lead with
`pass` or `changes requested`; each finding gives the file/line, misplaced
responsibility, and smallest move. A leak finding also names the client-side
knowledge it forces and the owner-side state or query that replaces it. State
reviewed SHAs and unavailable evidence.
