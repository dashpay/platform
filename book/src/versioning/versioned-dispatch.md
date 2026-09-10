# Versioned Dispatch

## The Problem: Running the Right Code

The previous two chapters explained *what* gets versioned (every method in the
platform) and *how* version numbers are stored (nested structs inside an
immutable `PlatformVersion` snapshot). This chapter covers the most important
part: how those version numbers actually select which code runs.

The core idea is simple. Every versioned function has a *dispatch method* that
reads a `FeatureVersion` value and calls the corresponding implementation. But
the way this dispatch is organized across files, the error handling conventions,
and the step-by-step process of adding a new version -- these are the details
that make the pattern work at scale.

## The Canonical Match Pattern

Here is the most common pattern in the codebase. This is from
`packages/rs-drive/src/util/grove_operations/grove_get_raw/mod.rs`:

```rust
impl Drive {
    pub fn grove_get_raw<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        match drive_version.grove_methods.basic.grove_get_raw {
            0 => self.grove_get_raw_v0(
                path,
                key,
                direct_query_type,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "grove_get_raw".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
```

Let us break down what is happening:

1. **The public method** (`grove_get_raw`) is the entry point. It takes all the
   business parameters plus a version reference (`drive_version: &DriveVersion`).

2. **The version lookup** reads the specific `FeatureVersion` for this method:
   `drive_version.grove_methods.basic.grove_get_raw`. This resolves to a `u16`.

3. **The match** dispatches to the right implementation. Version `0` calls
   `grove_get_raw_v0`. The catch-all arm (`version =>`) returns an error.

4. **The error** (`UnknownVersionMismatch`) includes the method name, the list
   of known versions, and the version that was actually received. This makes
   debugging version mismatches trivial.

This pattern appears hundreds of times across the codebase. It is the
fundamental building block of versioned execution.

## Multiple Versions

When a method has been revised, the match grows. Here is `update_contract`
from `packages/rs-drive/src/drive/contract/update/update_contract/mod.rs`:

```rust
impl Drive {
    pub fn update_contract(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .update
            .update_contract
        {
            0 => self.update_contract_v0(
                contract, block_info, apply,
                transaction, platform_version, previous_fee_versions,
            ),
            1 => self.update_contract_v1(
                contract, block_info, apply,
                transaction, platform_version, previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_contract".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
```

The structure is identical. The only differences are: there are now two known
versions (`0` and `1`), and the `known_versions` vector in the error arm lists
both. When a node running protocol version 1 processes a block, the version
number is `0` and `update_contract_v0` runs. When the network upgrades and the
version number becomes `1`, `update_contract_v1` runs instead.

Both v0 and v1 implementations coexist in the binary. Old code is never
deleted (at least not until a version is permanently retired from the network).
This is critical for replaying historical blocks -- a node syncing from genesis
needs to execute v0 for early blocks and v1 for later ones.

## OptionalFeatureVersion Dispatch

For features introduced after the initial protocol version, the dispatch
handles a `None` case:

```rust
// From identity_create/mod.rs

match platform_version
    .drive_abci
    .validation_and_processing
    .state_transitions
    .identity_create_state_transition
    .basic_structure
{
    Some(0) => {
        self.validate_basic_structure_v0(platform_version)
    }
    Some(version) => Err(Error::Execution(
        ExecutionError::UnknownVersionMismatch {
            method: "identity create transition: validate_basic_structure"
                .to_string(),
            known_versions: vec![0],
            received: version,
        }
    )),
    None => Err(Error::Execution(
        ExecutionError::VersionNotActive {
            method: "identity create transition: validate_basic_structure"
                .to_string(),
            known_versions: vec![0],
        }
    )),
}
```

Three arms instead of two:
- `Some(0)` -- the feature exists, use v0.
- `Some(version)` -- the feature exists but the version is unrecognized.
- `None` -- the feature does not exist in this protocol version.

The `VersionNotActive` error is different from `UnknownVersionMismatch`. It
means "this feature is legitimately not available," not "something went wrong."
This distinction matters for callers that need to handle graceful degradation.

## The Directory Convention

Versioned methods follow a strict directory layout. Let us use `grove_get_raw`
as the example:

```
packages/rs-drive/src/util/grove_operations/
  grove_get_raw/
    mod.rs          # dispatch method (the match statement)
    v0/
      mod.rs        # grove_get_raw_v0 implementation
```

The dispatch method lives in `grove_get_raw/mod.rs`. Each implementation
version gets its own subdirectory: `v0/mod.rs`, `v1/mod.rs`, etc. The dispatch
file declares the version modules:

```rust
// grove_get_raw/mod.rs
mod v0;
```

And each version module provides the actual implementation as a method on
`Drive`:

```rust
// grove_get_raw/v0/mod.rs

impl Drive {
    pub(super) fn grove_get_raw_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        // actual implementation
        match direct_query_type {
            DirectQueryType::StatelessDirectQuery { /* ... */ } => {
                // estimate costs
            }
            DirectQueryType::StatefulDirectQuery => {
                let CostContext { value, cost } =
                    self.grove.get_raw(path, key, transaction,
                                       &drive_version.grove_version);
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(Some(value.map_err(Error::from)?))
            }
        }
    }
}
```

Notice the visibility: `pub(super)`. The v0 function is only visible to its
parent module (the dispatch file). External code calls the public dispatch
method, never the versioned implementation directly.

The layout is the versioning contract made physical, and three rules follow
from it:

- **One directory per generation, always.** A behaviour change to a versioned
  method is a new `v1/` (or `v2/`, ...) directory with its own `mod.rs`, plus
  a new match arm. It is never an edit inside `v0/`. That includes edits that
  look harmless: threading a new parameter through `v0`, adding an
  `if platform_version.protocol_version >= 14` inside it, or computing a
  version-table gate that is always false for old versions. A shipped `vN/`
  stays byte-identical to what shipped, so a reviewer never has to prove that
  an in-place diff is inert for old blocks.
- **Inside a generation, a capability is a constant fact, not a check.** If
  `v1` admits a new keyword, `v1` admits it unconditionally
  (`Index::try_from_value_map(map, true)`). The decision of whether the
  keyword is allowed was made by the table that selected `v1`. Old
  generations cannot reach the new path at all, so there is nothing for them
  to check.
- **Start the new generation as a copy of the old one.** Duplicate `v0/` into
  `v1/`, rename the function, make the change, and move the tests that
  exercise the new behaviour across. Duplication between generations is the
  accepted cost; a shared helper with a flag is the thing it replaces.

When new behaviour lives in a helper reached from several generations (a
value walker, a property-reference resolver), the helper itself becomes a
versioned method: an `OptionalFeatureVersion` slot in the tables (`None` for
versions that predate the feature, `Some(0)` to dispatch to `_v0`), and the
helper takes `&PlatformVersion`. A `bool` on a shared context struct is the
wrong shape, because it moves the version decision from the tables to whoever
set the flag. Per-generation grammar constants may live on a generation-owned
struct; feature gates reachable from more than one generation may not.

Tests live with the generation they test: a `#[cfg(test)] mod tests` at the
bottom of `vN/mod.rs`, or `vN/tests/` when it grows. The dispatcher's `mod.rs`
may carry end-to-end tests that need every generation.

For state transitions in Drive ABCI, the same pattern applies but with trait
implementations:

```
packages/rs-drive-abci/src/execution/validation/state_transition/
  state_transitions/
    identity_create/
      mod.rs                  # dispatch traits and match statements
      basic_structure/
        mod.rs                # just declares v0
        v0/
          mod.rs              # BasicStructureValidationV0 implementation
      advanced_structure/
        mod.rs
        v0/
          mod.rs
      state/
        mod.rs
        v0/
          mod.rs
```

## The Error Types

There are two `UnknownVersionMismatch` error variants in the codebase -- one
for Drive and one for Drive ABCI -- but they have the same shape:

```rust
// packages/rs-drive/src/error/drive.rs

#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    #[error("drive unknown version on {method}, received: {received}")]
    UnknownVersionMismatch {
        method: String,
        known_versions: Vec<FeatureVersion>,
        received: FeatureVersion,
    },

    #[error("{method} not active for drive version")]
    VersionNotActive {
        method: String,
        known_versions: Vec<FeatureVersion>,
    },
    // ...
}
```

```rust
// packages/rs-drive-abci/src/error/execution.rs

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("platform unknown version on {method}, received: {received}")]
    UnknownVersionMismatch {
        method: String,
        known_versions: Vec<FeatureVersion>,
        received: FeatureVersion,
    },

    #[error("{method} not active for drive version")]
    VersionNotActive {
        method: String,
        known_versions: Vec<FeatureVersion>,
    },
    // ...
}
```

Both carry three pieces of information:
- **`method`**: A human-readable name identifying which dispatch failed.
- **`known_versions`**: The versions this binary knows how to handle.
- **`received`**: The version number that was actually in the platform version.

This makes the error message self-diagnosing. If you see "drive unknown version
on update_contract, received: 2, known versions: [0, 1]", you immediately
know that the binary is too old to handle the active protocol version.

## How to Add a New Version: Step by Step

Let us walk through the exact steps to add a v1 implementation of a method
that currently only has v0. We will use a fictional example:
`my_grove_operation`.

### Step 1: Write the new implementation

Create the v1 module:

```
my_grove_operation/
  mod.rs          # existing dispatch
  v0/
    mod.rs        # existing v0
  v1/
    mod.rs        # NEW: v1 implementation
```

```rust
// my_grove_operation/v1/mod.rs

impl Drive {
    pub(super) fn my_grove_operation_v1(
        &self,
        // same signature as v0, or possibly different
    ) -> Result<SomeResult, Error> {
        // new implementation with bug fix or feature
    }
}
```

### Step 2: Update the dispatch

In `my_grove_operation/mod.rs`, declare the new module and add the match arm:

```rust
mod v0;
mod v1;  // NEW

impl Drive {
    pub fn my_grove_operation(
        &self,
        // ...
        drive_version: &DriveVersion,
    ) -> Result<SomeResult, Error> {
        match drive_version.grove_methods.basic.my_grove_operation {
            0 => self.my_grove_operation_v0(/* ... */),
            1 => self.my_grove_operation_v1(/* ... */),  // NEW
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "my_grove_operation".to_string(),
                known_versions: vec![0, 1],  // UPDATED
                received: version,
            })),
        }
    }
}
```

### Step 3: Bump the slot in the unreleased protocol version's tables

The new arm is dead until a table selects it. Which table you edit depends on
whether the unreleased protocol version already owns one.

At the time of writing the latest released version is 13 and version 14 is in
development. `PLATFORM_V14` already references `DRIVE_VERSION_V9`, which was
created for version 14, so a version-14 change edits `v9.rs` in place:

```rust
// drive_versions/v9.rs  (existing file, amended)

pub const DRIVE_VERSION_V9: DriveVersion = DriveVersion {
    // ...
    grove_methods: DRIVE_GROVE_METHOD_VERSIONS_V2, // changed in v9: my_grove_operation v1
    // ...
};
```

`DRIVE_GROVE_METHOD_VERSIONS_V1` is referenced by released versions, so it
cannot be edited. Create the next one with struct update syntax:

```rust
// drive_grove_method_versions/v2.rs  (NEW file)

/// Differs from v1 in one slot: `basic.my_grove_operation` is 1 rather
/// than 0. v1 of the operation <what changed and why>.
pub const DRIVE_GROVE_METHOD_VERSIONS_V2: DriveGroveMethodVersions =
    DriveGroveMethodVersions {
        basic: DriveGroveBasicMethodVersions {
            my_grove_operation: 1,
            ..DRIVE_GROVE_METHOD_VERSIONS_V1.basic
        },
        ..DRIVE_GROVE_METHOD_VERSIONS_V1
    };
```

Had `DRIVE_VERSION_V9` also been shared with a released version, the same
logic would apply one level up: a new `drive_versions/v10.rs` pointing at
`DRIVE_GROVE_METHOD_VERSIONS_V2`, and `PLATFORM_V14` pointing at
`DRIVE_VERSION_V10`. The rule at every level is the same: **edit in place if
the constant belongs only to the unreleased version; create the next constant
if a released version references it.**

### Step 4: Annotate the platform version file

Add or extend the `// changed:` comment on the affected slot of the unreleased
`PLATFORM_V*`, and add a numbered item to the file's doc comment describing
the consensus change. That doc comment is the release changelog for the
protocol version.

### Step 5: If this is the first change after a release, create the version

Only the *first* consensus change after a release creates a new protocol
version. If version 14 had already shipped, the change above would start
version 15:

```rust
// version/v15.rs  (NEW file, copied from v14.rs)

pub const PROTOCOL_VERSION_15: ProtocolVersion = 15;

/// v15 hosts one consensus change so far:
///
/// 1. **my_grove_operation v1**: ...
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V10, // changed: my_grove_operation v1
    // ... everything else unchanged from V14
};
```

Then register it:

```rust
// version/mod.rs
pub mod v15;
pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_15;

// version/protocol_version.rs
pub const PLATFORM_VERSIONS: &[PlatformVersion] = &[
    PLATFORM_V1,
    // ...
    PLATFORM_V14,
    PLATFORM_V15,  // NEW
];

pub const LATEST_PLATFORM_VERSION: &PlatformVersion = &PLATFORM_V15;
```

A test in `system_limits/mod.rs` asserts that `PLATFORM_VERSIONS.len()`
equals `LATEST_VERSION`, so a version that is declared but not registered
fails the test run rather than silently resolving to the previous one. Every
subsequent change destined for version 15 amends `v15.rs` and the constants it
introduced, as in step 3.

### Step 6: Write tests

Test both generations through the dispatcher, and put each test with the
generation it exercises:

```rust
// my_grove_operation/v1/mod.rs
#[cfg(test)]
mod tests {
    #[test]
    fn should_apply_new_behaviour() {
        let platform_version = PlatformVersion::latest();
        // ... drive.my_grove_operation(..., &platform_version.drive)
    }
}

// my_grove_operation/v0/mod.rs
#[cfg(test)]
mod tests {
    #[test]
    fn should_keep_old_behaviour() {
        // frozen generation: pin the last protocol version that selected it
        let platform_version = PlatformVersion::get(13).expect("known version");
        // ...
    }
}
```

The current generation tests against `PlatformVersion::latest()` so it keeps
tracking the tip; the moment `v1` is introduced is the moment `v0`'s tests get
pinned to an explicit version. Do not write a test that merely asserts the
table slot's value (`assert_eq!(PLATFORM_V14.drive.grove_methods.basic.my_grove_operation, 1)`);
it restates the literal and cannot fail without the edit being deliberate. A
behaviour test that runs both versions through the dispatcher pins the gate
meaningfully.

This is a lot of steps, but each one is mechanical and the compiler guides you
through most of it. If you add a field to a version struct and forget to set it
in one of the fourteen platform version constants, the build fails.

## Passing Version References

A subtle but important convention is *which* version reference a function
receives. There are three patterns:

**`&PlatformVersion`** -- used by high-level code that might need any part of
the version tree. State transition processing, block execution, and similar
entry points take this.

**`&DriveVersion`** -- used by mid-level Drive code that only needs drive-
specific versions. The caller extracts `&platform_version.drive` once.

**`&GroveVersion`** -- used by the lowest-level GroveDB operations. Extracted
from `&drive_version.grove_version`.

This layering avoids passing the entire `PlatformVersion` into the deepest
functions. It also makes the dependency explicit: a function taking
`&DriveVersion` cannot accidentally use a DPP version number.

## The Version Flow in Block Processing

Here is how the version flows through a real execution path:

```
Block arrives from Tenderdash
    |
    v
PlatformState has the current protocol_version (e.g., 14)
    |
    v
PlatformVersion::get(14) -> &PLATFORM_V14
    |
    v
process_raw_state_transitions(&platform_version)
    |
    v
validate_state_for_identity_create_transition()
    reads: platform_version.drive_abci.validation_and_processing
           .state_transitions.identity_create_state_transition.state
    dispatches to: validate_state_v0()
    |
    v
drive.update_contract(&platform_version)
    reads: platform_version.drive.methods.contract.update.update_contract
    dispatches to: update_contract_v1()
    |
    v
drive.grove_get_raw(&platform_version.drive)
    reads: drive_version.grove_methods.basic.grove_get_raw
    dispatches to: grove_get_raw_v0()
```

The protocol version number enters at the top and the correct implementation
is selected at every level. No function chooses its own version -- it is always
determined by the version reference passed from above.

## The First Block of a New Protocol Version

The version flow above assumes the protocol version is already known. The
switch itself happens in `run_block_proposal`
(`packages/rs-drive-abci/src/execution/engine/run_block_proposal/mod.rs`).
On the first block of an epoch, if the protocol version the network locked in
during the previous epoch differs from the one in consensus, the block runs
under the new version:

```rust
// abbreviated
let block_platform_version = if epoch_info.is_epoch_change_but_not_genesis()
    && platform_state.next_epoch_protocol_version()
        != platform_state.current_protocol_version_in_consensus()
{
    let next_protocol_version = platform_state.next_epoch_protocol_version();

    // We should panic if this node is not supported a new protocol version
    let Ok(next_platform_version) = PlatformVersion::get(next_protocol_version) else {
        panic!("Failed to upgrade the network protocol version {next_protocol_version}. ...");
    };

    let old_protocol_version = block_platform_state.current_protocol_version_in_consensus();
    block_platform_state.set_current_protocol_version_in_consensus(next_protocol_version);

    // This is for events like adding stuff to the root tree, or making structural changes/fixes
    self.perform_events_on_first_block_of_protocol_change(
        platform_state, &block_info, transaction, old_protocol_version, next_platform_version,
    )?;

    next_platform_version
} else {
    last_committed_platform_version
};
```

Three things to take from this:

- **Epoch info is computed with the old version**, before the switch. The new
  version applies to everything after it.
- **A binary that does not know the new version panics** with an upgrade
  message. That is deliberate: a node that cannot run the agreed protocol must
  stop rather than produce a divergent state root.
- **`perform_events_on_first_block_of_protocol_change` is where state
  migrations live.** Anything a new protocol version needs done to the tree
  once, before its first state transition runs, goes here: creating a new
  root-tree subtree, rewriting a system contract, back-filling a sum tree.

The migration hook is itself a versioned method
(`drive_abci.methods.protocol_upgrade.perform_events_on_first_block_of_protocol_change`:
`None` in the earliest method tables, `Some(0)` once the first migration
existed, `Some(1)` in the two most recent tables). Its `v0` is a ladder of
guarded rungs, one per protocol version that needed a migration:

```rust
// packages/rs-drive-abci/src/execution/platform_events/protocol_upgrade/
//   perform_events_on_first_block_of_protocol_change/v0/mod.rs

if previous_protocol_version < 4 && platform_version.protocol_version >= 4 {
    self.transition_to_version_4(platform_state, block_info, transaction, platform_version)?;
}
if previous_protocol_version < 6 && platform_version.protocol_version >= 6 {
    self.transition_to_version_6(block_info, transaction, platform_version)?;
}
// ... 8, 9, 11, 12, 13 ...
if previous_protocol_version < 14 && platform_version.protocol_version >= 14 {
    self.transition_to_version_14(block_info, transaction, platform_version)?;
}
```

The guard shape matters. A node can cross more than one protocol version in a
single switch (a network that skipped a version, or a devnet started at an old
one), and the `previous < N && new >= N` form runs every rung it crossed, in
order. A rung written as `new == N` would be skipped by such a node, and its
state tree would be missing a subtree every other node has.

`v1` exists because the migrations write system contracts through a path that
bypasses the drive operation batch's cache invalidation; it runs the same
ladder and then refreshes the cached contract definitions so that a validator
with a warm cache and one with a cold cache serialize the same bytes. Read its
doc comment before touching the hook: it is a worked example of a
consensus-critical cache bug.

To add a migration for a new protocol version: add a rung at the bottom of the
ladder in the current generation of the hook, guarded by that version, with a
`transition_to_version_N` helper next to the others. A failure in a rung
returns `Err` from block processing on every node, so a rung either succeeds
deterministically or halts the network. The version 8 rung's `or_else` that
logs and continues is the exception for a migration that does not touch the
state structure, not a pattern to copy.

Whole new state transition kinds are gated separately, by the `is_allowed`
stage of the validation pipeline reading the constants in
`feature_initial_protocol_versions.rs`
(`ADDRESS_FUNDS_INITIAL_PROTOCOL_VERSION = 11`,
`SHIELDED_POOL_INITIAL_PROTOCOL_VERSION = 12`). A transition submitted before
its initial version is rejected with a `StateTransitionNotActiveError` rather
than an unknown-version dispatch error.

## Rules

**Do:**
- Always include the method name in the `UnknownVersionMismatch` error. Use
  the same string format you see in existing code: the plain method name for
  Drive methods (`"grove_get_raw"`), and a descriptive path for ABCI methods
  (`"identity create transition: validate_basic_structure"`).
- Keep the `known_versions` vector in the error arm up to date. When you add
  version 2, the vector should be `vec![0, 1, 2]`.
- Make versioned implementation methods `pub(super)` -- visible to the dispatch
  module but not to external code.
- Put every new generation in its own `vN/` directory, started as a copy of
  the previous one, with its tests inside it.
- Bump the slot in the unreleased protocol version's tables only: amend a
  constant that only the unreleased version references, create the next
  constant when a released version references it.
- Test the current generation against `PlatformVersion::latest()` and pin the
  previous generation's tests to the last protocol version that selected it.
- Put one-time state changes a protocol version needs in a guarded rung of
  `perform_events_on_first_block_of_protocol_change`.

**Do not:**
- Never call a versioned implementation directly (e.g., `grove_get_raw_v0`).
  Always go through the dispatch method. Direct calls bypass version control
  and break determinism.
- Never add a version to the match without also adding the corresponding
  `FeatureVersion` field value in the version constants. The dispatch will
  never be reached if no platform version sets that number.
- Never use `_ =>` as the catch-all arm in a version dispatch. Always use
  `version =>` so the variable is available for the error message. And never
  silently ignore unknown versions -- always return an error.
- Never modify a shipped generation, not even by threading a parameter or a
  version check through it. If v0 takes five parameters and v1 needs six,
  that is fine -- v0 keeps its original signature and body forever.
- Never gate new behaviour with a `bool` on a shared context struct. A helper
  reached from several generations gets an `OptionalFeatureVersion` slot and
  takes `&PlatformVersion`.
- Never write a test that only asserts a table slot's value. Test the
  behaviour through the dispatcher on both sides of the gate.
