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

### Step 3: Create a new subsystem version constant

If this is the first change in this subsystem version, create a new constant.
For example, if grove method versions were at V1:

```rust
// drive_grove_method_versions/v2.rs  (NEW file)

pub const DRIVE_GROVE_METHOD_VERSIONS_V2: DriveGroveMethodVersions =
    DriveGroveMethodVersions {
        basic: DriveGroveBasicMethodVersions {
            my_grove_operation: 1,  // CHANGED from 0 to 1
            grove_get_raw: 0,       // unchanged
            grove_delete: 0,        // unchanged
            // ... all other fields unchanged
        },
        // ... rest unchanged
    };
```

### Step 4: Create a new DriveVersion constant

Create a new `DriveVersion` that references the updated subsystem version:

```rust
// drive_versions/v7.rs  (NEW file)

pub const DRIVE_VERSION_V7: DriveVersion = DriveVersion {
    grove_methods: DRIVE_GROVE_METHOD_VERSIONS_V2,  // CHANGED
    // ... everything else unchanged from V6
};
```

### Step 5: Create a new PlatformVersion

Create the new platform version snapshot that references the new drive version:

```rust
// version/v13.rs  (NEW file)

pub const PROTOCOL_VERSION_13: ProtocolVersion = 13;

pub const PLATFORM_V13: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_13,
    drive: DRIVE_VERSION_V7,  // CHANGED
    // ... everything else unchanged from V12
};
```

### Step 6: Register the new version

Add `PLATFORM_V13` to the `PLATFORM_VERSIONS` array and update the version
constants:

```rust
// version/mod.rs
pub mod v13;

// version/protocol_version.rs
pub const PLATFORM_VERSIONS: &[PlatformVersion] = &[
    PLATFORM_V1,
    // ...
    PLATFORM_V12,
    PLATFORM_V13,  // NEW
];

pub const LATEST_PLATFORM_VERSION: &PlatformVersion = &PLATFORM_V13;
```

### Step 7: Write tests

Test both the old and new behavior:

```rust
#[test]
fn test_my_grove_operation_v0() {
    let platform_version = PlatformVersion::first();
    // ... assert v0 behavior
}

#[test]
fn test_my_grove_operation_v1() {
    let platform_version = PlatformVersion::latest();
    // ... assert v1 behavior
}
```

This is a lot of steps, but each one is mechanical and the compiler guides you
through most of it. If you add a field to a version struct and forget to set it
in one of the twelve (now thirteen) platform version constants, the build fails.

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
PlatformState has the current protocol_version (e.g., 12)
    |
    v
PlatformVersion::get(12) -> &PLATFORM_V12
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
- Keep v0 code intact when adding v1. Never modify an existing version's
  implementation. Copy it, rename it, and make your changes in the new version.

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
- Never change the signature of an existing version's function after it has
  been released to the network. If v0 takes five parameters and v1 needs six,
  that is fine -- v0 keeps its original signature forever.
