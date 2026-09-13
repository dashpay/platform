# Coding Conventions

The other chapters of this book explain how each subsystem works. This chapter
is the prescriptive companion: the rules a change has to follow to keep the
codebase maintainable, and the reason behind each one. Most of these rules were
learned in code review rather than designed up front, so each entry says what
goes wrong when the rule is skipped. Where a rule has mechanics, it links to the
chapter that shows them.

Read this before your first non-trivial PR. Skim the checklists at the end
before every PR.

## The one constraint behind every rule

Dash Platform is a replicated state machine. Every masternode must produce a
byte-identical state root for every block, including blocks from years ago that
a fresh node replays from genesis. That means code that shipped inside a
released protocol version must keep behaving exactly as it did on the day it
shipped, forever, while the same binary also runs the newest behaviour for
current blocks.

Almost everything below follows from that constraint: why behaviour is split
into frozen generations, why numbers live in version tables, why a stray
`unwrap` is a network outage and not a bug report, and why a check has to sit
in the right validation tier. When a rule here seems pedantic, the question to
ask is "could two nodes disagree because of this?"

## Where code goes

The crates are layered, and dependencies only point downward:
`platform-version` at the bottom, then `dpp`, then `drive`. Above `drive` the
graph forks: `drive-abci` (the node) and `dash-sdk` (the client) both depend on
`drive`, the SDK with the `verify` feature only, and `dash-sdk` does not depend
on `drive-abci` at all (see the
[Monorepo Overview](../architecture/overview.md)). Put a change in the lowest
crate that has what the change needs, and no lower.

| You are adding… | It lives in | Notes |
|---|---|---|
| A protocol type, its wire shape, or a pure-data invariant | `packages/rs-dpp` | Knows nothing about storage. Structural validation that needs only the data itself goes here. |
| Storage layout, GroveDB operations, index maintenance, query lowering, proof generation | `packages/rs-drive` (`server` feature) | One method per directory with versioned dispatch. |
| Proof verification a client runs | `packages/rs-drive/src/verify/` | Must compile with `--no-default-features --features verify`. |
| A validation step that reads platform state, block execution, an ABCI handler | `packages/rs-drive-abci/src/execution/` | Handlers under `abci/` stay thin and delegate. |
| A gRPC query handler | `packages/rs-drive-abci/src/query/<name>/` plus the proto in `packages/dapi-grpc` | Request version dispatch mirrors method version dispatch. |
| A version number, a limit, a fee rate | `packages/rs-platform-version` | Never a literal in a method body. |
| Client-side proof composition (`FromProof`, Tenderdash signature check) | `packages/rs-drive-proof-verifier` | Calls into `drive::verify`, never into GroveDB directly. |
| A client API | `packages/rs-sdk` | Follow the query checklist in `packages/rs-sdk/README.md`. |
| A JavaScript binding | `packages/wasm-dpp2`, `packages/wasm-sdk` | Mirror the Rust shape; never validate. See `packages/wasm-dpp2/CONVENTIONS.md`. |
| Mobile orchestration (sync, identity registration, DashPay) | `packages/rs-platform-wallet` | The FFI crates and the Swift and Kotlin SDKs marshal; they do not decide. |

Three boundaries are enforced by CI and worth knowing by name:

- **The verify-only cut.** `packages/wasm-drive-verify` builds `drive` with
  `default-features = false, features = ["verify"]`. Anything under
  `src/verify/**` that reaches a `server`-gated helper breaks the JavaScript
  build on a Rust-only PR. A helper both the prover and the verifier need goes
  in `drive::util::common`, and the server-side code delegates to it.
- **The transport-free cut.** `drive-proof-verifier`, `dash-platform-queries`,
  and the wasm32 dependency trees of `dash-sdk` and `wasm-sdk` must not pull in
  `hyper`, `rustls`, `tower`, or `mio`. Embedders with their own networking
  consume verification through these crates, so a dependency added for
  convenience in a shared crate can fail a job that has nothing to do with the
  change.
- **The wallet closure.** The wallet CI fast path compiles only the wallet
  crates and their known dependents. Adding a dependency on a wallet crate from
  a new place fails the build until the closure list in
  `.github/scripts/check-wallet-closure.py` is updated deliberately.

## Versioned behaviour

The mechanics of `PlatformVersion`, feature version tables, and the dispatcher
shape are covered in the [Versioning](../versioning/platform-version.md)
chapters. The rules here are about what to do with those mechanics.

### Shipped generations are frozen

A behaviour change to a versioned method means a new `vN` module selected only
by the tables of the unreleased protocol version. It never means editing a
shipped `vN`, and that includes "harmless" edits: threading a new parameter
through it, adding a version-table check inside it, or computing a gate that is
always false for old versions. Inside the new generation the capability is a
constant fact (`Index::try_from_value_map(map, true)`), not a runtime check.

Why: replay safety becomes structural instead of something a reviewer has to
prove about a diff. A dead version check inside `v1` misleads the next reader
into thinking `v1` can take that path. Shipped files should stay byte-identical
to what shipped.

How: copy the previous generation into the new module, make the change there,
move the tests that exercise the new behaviour into the new module, and bump
the method's number in the new protocol version's tables only. Duplication
between generations is the accepted cost; it is cheaper than a drift-prone
flag.

### Table versions follow protocol-version boundaries, not PRs

Version-table constants (`DRIVE_ABCI_VALIDATION_VERSIONS_V10`,
`CONTRACT_VERSIONS_V6`, `SYSTEM_LIMITS_V4`, and so on) exist to mark the point
where a released protocol version froze them. If the unreleased protocol
version already introduced a new table version, a second feature landing in
the same protocol version amends that table in place. Adding another table
version would leave one referenced by no `PLATFORM_V*` at all.

Leaf implementation modules are different: each feature still gets its own new
`vN` implementation module under the method directory. The rule is about the
tables.

### Shared helpers get a versioned method, not a capability flag

When new behaviour lives in a helper reached from several generations (a value
walker, a property-reference resolver, a shared insert path), the fix is the
standard versioned-method pattern: an `OptionalFeatureVersion` field in the
tables (`None` for versions that predate the feature, `Some(0)` to dispatch to
a `_v0` implementation), and the helper takes `&PlatformVersion`. A
`admit_property_references: bool` on a shared context struct is the wrong
shape, because it moves the version decision away from the tables into
whichever caller happened to set the flag.

Generation-owned grammar constants may live on a per-generation struct. Feature
gates reachable from more than one generation may not.

### Numbers live in the version tables

If a protocol version changes only a number (a cap, a floor, a count), the
number goes into `SystemLimits` or the relevant `*_constants` table, and the
versioned method reads it. Do not add a `vN` method module whose entire body is
`return NEW_CONSTANT`.

How: add the field with a doc comment naming the method version that reads it.
Backfill every shipped `SYSTEM_LIMITS_V*` (and the mock tables under
`version/mocks/`) with the old value so shipped protocol versions are
behaviour-preserving. Add the next `SYSTEM_LIMITS_V{n+1}` for the unreleased
protocol version with the new value. Keep a real method version only where the
logic differs: in `packages/rs-dpp/src/withdrawal/daily_withdrawal_limit/`,
`v0` computes a tiered percentage of total credits and `v2` reads
`daily_withdrawal_limit_percent` and `max_daily_withdrawal_amount` from
`SystemLimits`; that is a logic change and earns its own version. Raising the
percentage later would be a table edit, not a `v3`.

Why: reviewers look for limits in the tables. A constant hidden in a method
module is a second place to check and a dead module per bump.

### Fees change through named schedules

A fee change for a new protocol version is a new named schedule
(`FEE_VERSION2` in `packages/rs-platform-version/src/version/fee/v2.rs`, and
the next one for the next protocol version) referenced from that
`PLATFORM_V*`, with versioned fee-group constants where a group changed. Never
inline a nested `FeeVersion { .. }` override inside the platform version
constant. Preserve earlier schedules. If the unreleased protocol version
already owns a new schedule, amend that schedule instead of creating another
generation nobody references.

`fee_version_number` is a separate concept from the schedule generation: it
keys persisted fee history and storage-refund behaviour, and it is stored in
platform state. Inspect its consumers before changing it. `FEE_VERSION1` and
`FEE_VERSION2` both carry `fee_version_number: 1` because storage rates did not
change between them, only a fee group did. See the
[Fee System Overview](../fees/overview.md).

### Consensus code reads the version from state

`PlatformVersion::latest()` is what the binary supports. The active version is
what the network agreed on, obtained from
`platform_state.current_platform_version()`. During an upgrade window they
differ. `latest()` belongs in tests and in client code that is choosing which
format to emit; it never belongs on a path that decides what a block does.

Serde-based conversions of `DataContract` read a process-wide current version
set through `PlatformVersionCurrentVersion::set_current`. Tests that build
platforms at different protocol versions race on it. Prefer the explicit
`to_value(platform_version)` style API wherever a version is in hand.

### Dispatcher shape

The [Versioned Dispatch](../versioning/versioned-dispatch.md) chapter shows the
canonical `match` and its rules. Three details that come up in review:

- The implementation is `pub(super) fn name_v0` in `v0/mod.rs`, usually
  `#[inline(always)]`. Nothing outside the method directory calls it.
- The catch-all arm is `version => Err(...UnknownVersionMismatch { method,
  known_versions, received: version })`, using the crate's own error
  (`DriveError`, `ExecutionError`, `ProtocolError`). Optional stages add a
  `None => Err(...VersionNotActive { .. })` arm.
- `#[allow(clippy::too_many_arguments)]` on a dispatcher and its
  implementations is accepted. Do not invent a one-off parameter struct only
  to silence the lint.

### `platform_version` is the last parameter

The observed order in `drive` and `drive-abci` is: `&self`, domain inputs,
`block_info`, mode flags (`apply`, `stateful`), the estimation map,
`transaction`, the operations accumulator, `platform_version`. A new parameter
goes up with the context items, never after `platform_version`. The same holds
for fields on context structs such as parse contexts. The known exception is
`previous_fee_versions`, which trails it on fee-returning methods.

Why: readers of a few hundred versioned signatures expect the closing
parameter to be the version. Appending after it breaks that scan.

### Versioned types

A protocol type that may evolve is an enum over per-version structs
(`DataContract { V0(DataContractV0), V1(DataContractV1) }`) with accessor
traits (`DataContractV0Getters`) implemented on the enum, so call sites never
match on the variant. The serde tag for versioned enums is `$formatVersion`
with variants renamed to `"0"`, `"1"`; `$version` is reserved for the legacy
protocol-version fields and must not be reused. New variants are appended.

The full pattern, including the derive stack and the round-trip test template,
is in `docs/json-value-conversion-canonical-pattern.md` and the
[Serialization](../serialization/platform-serialization.md) chapters.

## Where a check belongs

A new validation rule has exactly one correct tier, determined by what it needs
to read. The [Validation Pipeline](../state-transitions/validation-pipeline.md)
chapter walks the stages in order; this table is the placement guide.

| Tier | Location | May read | Error class |
|---|---|---|---|
| Pure-data invariants | `rs-dpp` (`validate_basic_structure`, document type and contract self-validation) | The value itself and `PlatformVersion` | `BasicError` |
| Basic structure | `drive-abci` `<transition>/basic_structure/vN/` | `Network` and `PlatformVersion` only | `BasicError` |
| Signature and nonces | `drive-abci` processor traits | The signing identity, nonces | `SignatureError`, unpaid rejection |
| Advanced structure without state | `drive-abci` `<transition>/advanced_structure/vN/` (`validate_advanced_structure`) | The transition, the fetched `PartialIdentity`, and `PlatformVersion`; no Drive reads | `ConsensusError`, paid |
| Advanced structure with state | `drive-abci` `batch/advanced_structure/vN/` and the per-action validators under `batch/action_validation/*/advanced_structure_vN/` (`validate_advanced_structure_from_state`) | The action produced by `transform_into_action`, including the contracts it fetched, plus block, network, and identity context; no further Drive reads in the validator | `ConsensusError`, paid |
| State | `drive-abci` `<transition>/state/vN/` | Drive through `PlatformRef` and the transaction | `StateError`, paid |
| Execution | `drive` operations | GroveDB | Internal `Error` only |

Rules that fall out of the table:

- Basic structure runs inside `check_tx` on every mempool entry. Keep it cheap
  and stateless.
- Advanced structure without state uses the transition and the already fetched
  signing identity. Advanced structure with state uses data fetched by
  `transform_into_action`, such as the contract schema carried by a document
  action: the transformer performs the Drive reads, and the advanced validator
  consumes the resulting action. A transition opts into the second variant
  through `has_advanced_structure_validation_with_state`; today only `Batch`
  does.
- Checks that require additional Drive queries, such as uniqueness checks,
  belong in state validation. Needing a fetched contract does not by itself
  make a structural check a state-validation check.
- Preserve mempool coverage. `Batch` runs advanced structure with state during
  `check_tx`, while full state validation is skipped there
  (`validates_full_state_on_check_tx` defaults to `false`; masternode votes
  are the one transition that opts in, because they are unpaid). Moving a
  contract-dependent structural check into state validation would remove that
  rejection from mempool admission.
- Validation outcomes are `ConsensusValidationResult`, returned as `Ok`. A
  `Result::Err` from a validation function means the node is broken, not that
  the transition is invalid. The block loop converts it into an internal-error
  result instead of halting, and `process_proposal` rejects any block that
  contains one.
- Validation lives once. `wasm-dpp2`, the SDKs, and the FFI layer never
  duplicate a length, range, or count check that `rs-dpp` enforces, even when
  the duplicate would give a friendlier early error. Two definitions of
  "valid" drift the day one of them changes.
- Consensus errors carry numeric codes in fixed bands
  (`packages/rs-dpp/src/errors/consensus/codes.rs`): basic errors in
  10000-19999, signature in 20000-29999, fee in 30000-39999, state in
  40000-49999, with sub-bands per domain. A new error takes the next free code
  in its band and is appended to its enum. See
  [Consensus Errors](../error-handling/consensus-errors.md) and
  [Error Codes](../error-handling/error-codes.md).

## Errors, panics, and arithmetic

### A panic in block execution halts the chain

`packages/rs-drive-abci/src/main.rs` installs a panic hook that cancels the
node, and there is no `catch_unwind` anywhere in the crate. A panic reachable
from `process_proposal` or `finalize_block` is deterministic, so every
validator hits it on the same block and the network stops. This is the
strongest rule in the codebase:

- On any path reachable from block execution, no `unwrap`, no `expect` without
  a proof, no slice indexing without a bounds check, no `unreachable!`, no
  division by a value that could be zero.
- `expect` is allowed when the message states the invariant established
  immediately above it, in the style the codebase already uses:
  `.expect("check above enforces it exists")`,
  `.expect("we have already shown there is 1 byte")`. If you cannot write that
  sentence, return an error.
- A condition that "cannot happen" but is reachable returns
  `DriveError::CorruptedCodeExecution` (or `CriticalCorruptedState` when the
  right response is to stall) or `ExecutionError::CorruptedCodeExecution`. See
  [Drive Errors](../error-handling/drive-errors.md).
- Action transformers under `state_transition_action/**/transformer.rs` run
  before fee and funding checks, so they are the most exposed surface. Treat
  every conversion there as untrusted input.

The `*_blocking` accessors in `rs-platform-wallet` are a related hazard on the
client side: `tokio::sync::RwLock::blocking_read` panics on a runtime worker,
and the iOS profiles build with `panic = "abort"`. When the current thread
may be inside a runtime, use `try_read` and return a typed busy error.

### Integer arithmetic wraps in release

The root `Cargo.toml` does not enable `overflow-checks`, so a release build
wraps silently and a debug build panics. Neither is acceptable in credit, fee,
or balance math. Use the checked forms and surface `Overflow`; `FeeResult`
already exposes `checked_add_assign` for the common case.

### Append-only structures are enforced by CI

Structures marked `// @append_only` (the `drive-abci` error enums, the data
contract error enums, document property types, storage key requirements,
`CheckTxLevel`) accept new variants at the end and nothing else. Structures marked `// @immutable` (`Epoch`, `BlockInfo`) accept no
change at all. The `Detect immutable structure changes` step in
`.github/workflows/tests-rs-workspace.yml` diffs the tagged block against the
base branch and fails the PR on a deletion or, for `@immutable`, any change.
The reason is serialization: these types are encoded into proofs, responses,
and stored state with positional discriminants. Reordering or removing a
variant changes what old bytes mean.

The same rule applies without a marker to anything with a `DO NOT CHANGE
ORDER` banner, to `StateTransitionType` discriminants, and to
`SystemDataContract` slots (note the reserved `FeatureFlags = 2`).

### Consensus errors versus internal errors

A `ConsensusError` is something two nodes must agree on: it is deterministic,
carries a code, and is serialized to the client. A `ProtocolError`, `DriveError`
or `ExecutionError` means the build is wrong, data is corrupt, or an API was
misused. Never return a consensus error for an internal failure or an internal
error for a user mistake; the block loop treats the two classes differently.

## Module layout and style

- **One method, one directory.** `method_name/mod.rs` holds the public
  dispatcher and its doc comment; `method_name/v0/mod.rs` holds the
  implementation. The dispatcher's doc comment has `# Parameters` and
  `# Returns` sections. `drive` and `drive-abci` compile with
  `#![deny(missing_docs)]`, so every public item needs a doc line.
- **Drive's naming triad.** `*_operations` gathers `Vec<LowLevelDriveOperation>`
  without applying anything and takes the `estimated_costs_only_with_layer_info`
  map for dry runs; `*_apply_and_add_to_operations` appends into a caller-owned
  accumulator and applies; the bare public method owns the accumulator and
  returns a `FeeResult`. Keep new storage methods in this shape so cost
  estimation and real execution share one code path. See
  [Drive Operations](../state-transitions/drive-operations.md).
- **Tests sit with the implementation.** A `#[cfg(test)] mod tests` block at
  the bottom of `vN/mod.rs`, or `vN/tests/` when it outgrows the file. The
  dispatcher's `mod.rs` may carry end-to-end tests that need every version.
- **Imports at the top.** No fully qualified `crate::a::b::c::function(...)`
  at a call site or in a signature. Add a `use` and call it bare, or keep at
  most one module qualifier when the bare name would be ambiguous. Inline paths
  are fine in doc comments and inside macros that cannot see imports.
- **No unsafe.** `dpp`, `drive`, and `drive-abci` compile with
  `#![forbid(unsafe_code)]`. Unsafe stays in the FFI crates and in
  dependencies.
- **Test-code lints are real lints.** CI runs
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`, so
  `clippy::type_complexity` in a test helper or `&vec![..]` passed as a slice
  in a test fails the build. `drive-abci` allows a fixed list of test lints at
  the crate root; do not extend it for convenience.
- **State access is snapshot-based.** `Platform.state` is an `ArcSwap`;
  read with `load()` and pass `PlatformRef` or `PlatformStateRef` down. The
  only locks are the ABCI application's `transaction` and
  `block_execution_context`. In async code, a guard's scope, not a `drop()`
  call, is what clears `clippy::await_holding_lock`; wrap the guard in a block
  that ends before the first `.await`.

## Tests

- **The latest generation tests against `PlatformVersion::latest()`.** Only
  frozen older generations pin an explicit version with
  `PlatformVersion::get(n)`. When you introduce `v(N+1)`, that is the moment
  to pin the now-frozen `vN` module's tests. Pinning the current generation to
  today's number only creates churn when the next version lands.
- **Test behaviour through the dispatcher, on both sides of the gate.** "PV13
  rejects this query shape, PV14 accepts it" is a real test. "Slot `foo` is 0
  in `PLATFORM_V13` and 1 in `PLATFORM_V14`" restates the table literal and
  cannot fail without the edit being deliberate; do not write it. The
  exception is a historical freeze test that guards a shipped on-chain table
  with a replay rationale, like `historical_method_table_freeze` in the drive
  document method versions.
- **Names start with `should`**, unit tests never touch the network, and
  randomness is seeded (`StdRng::seed_from_u64`). Platforms come from
  `TestPlatformBuilder`; Drives come from
  `setup_drive_with_initial_state_structure`. See
  [Unit Tests](../testing/unit-tests.md).
- **Multi-block behaviour is a strategy test.** Anything involving epochs,
  upgrades, withdrawals over time, or proposer churn belongs in
  `packages/rs-drive-abci/tests/strategy_tests/`. See
  [Strategy Tests](../testing/strategy-tests.md).
- **Fixtures are shared.** `rs-dpp` exposes its fixtures under the
  `fixtures-and-mocks` feature; `rs-sdk` records test vectors from a devnet
  and replays them offline. Add to those rather than hand-rolling payloads.
- **Struct-literal churn is caught by `--all-targets`.** A new required field
  compiles clean under a lib-only `cargo check` and then fails in a test module
  in CI. After adding a field or changing a signature, run
  `cargo check --workspace --all-targets`.

## Client layers

### Proof verification is layered

`drive::verify` does the GroveDB work: each verifier is a method on the
`Drive*Query` type, builds the path query with the same builder the prover
uses, and returns `(RootHash, T)`. `drive-proof-verifier` takes the full
`Proof` message, calls into that method, and layers the Tenderdash signature
check on top; the `FromProof` impl is the SDK's entry point. Never call
`GroveDb::verify_*` from the proof-verifier crate and never build the path
query at the SDK call site; both create a "these bytes must match the prover"
invariant that nothing checks. Entry types live in `drive` and are re-exported
so SDK consumers need no direct `drive` dependency.

### The wasm layer mirrors, it does not reshape

`rs-dpp` serde defines the canonical wire shape and `wasm-dpp2` mirrors it one
to one, differing only in primitive encoding (`Uint8Array` versus base64,
`bigint` versus number-or-string). Sum types are internally tagged with `type`,
never wrapped in `data`. Field-like accessors are properties, verbs are
methods. The rules and their reasons are in `packages/wasm-dpp2/CONVENTIONS.md`
and the [WASM](../wasm/binding-patterns.md) chapters.

### FFI and mobile SDKs hold no business logic

The FFI crates (`rs-sdk-ffi`, `rs-platform-wallet-ffi`, `key-wallet-ffi`) and
the Swift and Kotlin SDKs do three things: persist Rust-emitted state, load it
for views, and call a function that already exists in a Rust library. If the
wrapper is deciding anything (an index, a derivation path, which key, how
many), that decision moves into Rust, usually `rs-platform-wallet`. An FFI
function is named for the user-facing verb (`sendContactRequest`,
`sendToContact`), not for a cryptographic or wallet building block; DIP-14 and
DIP-15 derivation primitives stay internal. The motivating case was a Swift
view that iterated the gap limit, built the DIP-9 path, and pulled the mnemonic
across the FFI twice to do what one `preview_identity_registration_keys` call
now does.

## Commits, pull requests, and what to run before pushing

- **Conventional Commits, with the repository's own scope list.** Types and
  scopes are enforced by `.github/workflows/pr.yml`. The subject starts
  lowercase. A change that spans packages uses no scope rather than an
  invented one (`deps` is not a scope; `rs-platform-wallet-ffi` changes use
  `platform-wallet`).
- **`!` means consensus-breaking.** `feat!:` and `fix!:` are reserved for
  changes that would fork the network if rolled out unevenly: protocol rules,
  state-transition shapes, the fee model, anything nodes must agree on.
  Removing a public Swift symbol or an `extern "C"` function is ordinary
  `feat:` or `refactor:` work described in the PR body.
- **Design documents are working artifacts.** A spec or plan written to drive
  a change stays in the working tree and is never staged. Stage with explicit
  paths. Facts worth keeping after merge go into a code comment, a test name,
  or the PR description.
- **Local gate before pushing.** The exhaustive pass is CI's job; the local
  pass is scoped to what changed:

  ```bash
  cargo fmt --all
  cargo clippy -p <crate> --all-features --all-targets -- -D warnings
  cargo check --workspace --all-targets          # after any field or signature change
  cargo check -p drive --no-default-features --features verify   # when touching src/verify/**
  cargo test -p <crate> <filter>                 # the tests for the change, not the suite
  ```

  Do not block a push on the full `drive-abci` suite; it runs for a quarter of
  an hour and CI runs it anyway.

## Checklists

**Adding a new generation of a versioned method**

1. Copy `vN/` to `v(N+1)/`, make the change there, leave `vN/` byte-identical.
2. Add the `N+1 =>` arm to the dispatcher and extend `known_versions`.
3. Bump the method's number in the unreleased protocol version's tables only.
   If that protocol version already has a new table constant, amend it.
4. Move or write the behaviour tests in `v(N+1)/` against
   `PlatformVersion::latest()`; pin `vN/`'s tests to `PlatformVersion::get(n)`.
5. Add a test that runs both versions through the dispatcher.

**Changing a limit or a fee**

1. A number: add or update the `SystemLimits` (or `*_constants`) field, backfill
   shipped tables with the old value, add the next table for the unreleased
   protocol version.
2. A fee: add or amend the unreleased protocol version's named `FEE_VERSION*`
   schedule; do not touch `fee_version_number` unless storage rates changed and
   you have read its consumers.
3. The method reads the table. No new `vN` unless the logic changed.

**Adding a consensus error**

1. New file under `errors/consensus/{basic,state,signature,fee}/<domain>/` with
   the standard derive stack, private fields, `new()`, getters, and the
   ordering banner.
2. Append the variant to its sub-enum; add the `From` impl for
   `ConsensusError`.
3. Assign the next free code in the band in `codes.rs`.
4. If JavaScript branches on it, mirror the code in
   `packages/wasm-dpp2/src/consensus_error.rs`.

**Adding a check to a state transition**

1. Pick the tier from the table above by what the check reads.
2. Put it in `<transition>/<tier>/v(N+1)/` if the transition has shipped, or
   in the existing `vN` if that generation is still unreleased.
3. Return it through `ConsensusValidationResult`; never `Err`.
4. Test the rejection through `process_raw_state_transitions`, and test that
   the previous protocol version still accepts the input.

**Adding a query end-to-end**

1. Proto message in `packages/dapi-grpc/protos/platform/v0/platform.proto`,
   registered in `build.rs`'s versioned request and response lists.
2. Drive query type and prover, then `drive::verify` method with its
   `FeatureVersion` slot, with a prover-verifier round trip test.
3. `drive-abci` handler under `query/<name>/{mod.rs,v0/}` dispatching on the
   request version against `drive_abci.query` bounds.
4. `drive-proof-verifier` `FromProof` wrapper, then the `rs-sdk` `Query` and
   `Fetch`/`FetchMany` impls per the checklist in `packages/rs-sdk/README.md`.
5. Genesis test data and a recorded test vector so the SDK test runs offline.
