# Platform Version

## The Problem: Deterministic Upgrades in a Distributed System

Dash Platform is a replicated state machine. Every masternode in the network
processes the same transactions and must arrive at the *exact same state*. If
even one node computes a fee differently, serializes a document with one extra
byte, or validates a field that others skip, the chain forks.

Now imagine you need to ship a bug fix. In a normal application you deploy the
new binary and move on. In a blockchain you have a harder constraint: **not
every node upgrades at the same moment.** Some masternodes will still be running
the old code when the new protocol version activates. The system needs a way to
say "at protocol version N, use *these exact behaviors*" -- and it needs to be
impossible for a developer to accidentally mix behaviors from different versions.

The answer is the `PlatformVersion` struct: a single, massive, immutable
snapshot that pins *every versioned behavior in the entire platform* to a
concrete value.

## The PlatformVersion Struct

Open `packages/rs-platform-version/src/version/protocol_version.rs` and you
will find the heart of the system:

```rust
#[derive(Clone, Debug)]
pub struct PlatformVersion {
    pub protocol_version: ProtocolVersion,
    pub dpp: DPPVersion,
    pub drive: DriveVersion,
    pub drive_abci: DriveAbciVersion,
    pub consensus: ConsensusVersions,
    pub fee_version: FeeVersion,
    pub system_data_contracts: SystemDataContractVersions,
    pub system_limits: SystemLimits,
}
```

Where `ProtocolVersion` is simply:

```rust
pub type ProtocolVersion = u32;
```

Every field inside `PlatformVersion` is itself a version struct -- and those
structs contain more version structs, all the way down to individual method
version numbers. We will explore that nesting in the next chapter. For now,
the key insight is that `PlatformVersion` is the **root** of a tree. Given a
single protocol version number (like `7`), you can resolve the exact version
of every method, every fee parameter, every system limit, and every data
contract schema across the entire platform.

Think of it like a lockfile in a package manager. `Cargo.lock` pins every
transitive dependency to a specific version so that builds are reproducible.
`PlatformVersion` does the same thing for runtime behavior: it pins every
function version so that execution is deterministic.

## The Version Array

Each protocol version gets its own constant, defined in a separate file. At
the time of writing, the platform has fourteen versions:

```rust
// packages/rs-platform-version/src/version/mod.rs

pub type ProtocolVersion = u32;

pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_14;
pub const INITIAL_PROTOCOL_VERSION: ProtocolVersion = 1;
pub const ALL_VERSIONS: RangeInclusive<ProtocolVersion> = 1..=LATEST_VERSION;
```

These fourteen snapshots are collected into a single static array in
`protocol_version.rs`:

```rust
pub const PLATFORM_VERSIONS: &[PlatformVersion] = &[
    PLATFORM_V1,
    PLATFORM_V2,
    PLATFORM_V3,
    PLATFORM_V4,
    PLATFORM_V5,
    PLATFORM_V6,
    PLATFORM_V7,
    PLATFORM_V8,
    PLATFORM_V9,
    PLATFORM_V10,
    PLATFORM_V11,
    PLATFORM_V12,
    PLATFORM_V13,
    PLATFORM_V14,
];

pub const LATEST_PLATFORM_VERSION: &PlatformVersion = &PLATFORM_V14;
pub const DESIRED_PLATFORM_VERSION: &PlatformVersion = LATEST_PLATFORM_VERSION;
```

The array is indexed by protocol version number minus one (since versions are
1-indexed). `PLATFORM_V1` sits at index 0, `PLATFORM_V14` at index 13. This
simple layout is what makes the `get` function so fast.

One file, one protocol version. `v14.rs` was created when the first consensus
change after version 13 shipped needed somewhere to live, and every later
change destined for version 14 amends that same file. The day version 14 is
released the file freezes: from then on it is part of the chain's historical
record, and the next consensus change creates `v15.rs`. There is never a
`v14.rs` that means one thing on a node built last month and another on a node
built today.

## What a Version Snapshot Looks Like

Here is the very first version, `PLATFORM_V1`, slightly abbreviated:

```rust
// packages/rs-platform-version/src/version/v1.rs

pub const PROTOCOL_VERSION_1: ProtocolVersion = 1;

pub const PLATFORM_V1: PlatformVersion = PlatformVersion {
    protocol_version: 1,
    drive: DRIVE_VERSION_V1,
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V1,
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V1,
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V1,
        query: DRIVE_ABCI_QUERY_VERSIONS_V1,
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V1,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V1,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V1,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V1,
        state_transitions: STATE_TRANSITION_VERSIONS_V1,
        contract_versions: CONTRACT_VERSIONS_V1,
        document_versions: DOCUMENT_VERSIONS_V1,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V1,
        token_versions: TOKEN_VERSIONS_V1,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V1,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V1,
    fee_version: FEE_VERSION1,
    system_limits: SYSTEM_LIMITS_V1,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 0,
    },
};
```

Now compare with `PLATFORM_V14`, the latest at the time of writing. By
convention, each sub-constant slot that was bumped carries a trailing
`// changed:` comment saying what changed. The `protocol_version` field is the
snapshot's identity and is never annotated. One bumped slot in this snapshot,
`validation` (`DPP_VALIDATION_VERSIONS_V4` to `V5`), is missing its comment,
which is exactly the omission the convention exists to prevent:

```rust
// packages/rs-platform-version/src/version/v14.rs

pub const PLATFORM_V14: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_14,
    drive: DRIVE_VERSION_V9, // changed: drive document method versions v4 (v2 index walkers, detect_ranked_mode slot)
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V10, // changed: records the per-block total credits history
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V10, // changed: contested-index cross-check + refersTo validation
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3, // changed: prune bound for the total credits history
        query: DRIVE_ABCI_QUERY_VERSIONS_V3, // changed: ranked + boolean-HAVING routing gate
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V5,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V3, // changed: documentIndexOnlyDelete joins the wire
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V1,
        state_transitions: STATE_TRANSITION_VERSIONS_V3,
        contract_versions: CONTRACT_VERSIONS_V6, // changed: v3 document meta-schema (ranked, refersTo, requiredSince, timeRange)
        document_versions: DOCUMENT_VERSIONS_V4, // changed: document serialization format 3
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V2,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V3, // changed: daily_withdrawal_limit v2
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V3, // changed: DashPay v2 profile payment address fields
    fee_version: FEE_VERSION2,
    system_limits: SYSTEM_LIMITS_V4, // changed: relative daily withdrawal limit + time-range overlap cap
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};
```

Notice how only some subsystem versions change between V1 and V14. The ABCI
structure versions and checkpoint parameters are still at V1 because nothing
in them ever changed. The ABCI method versions, on the other hand, went from
V1 to V10 -- ten revisions of the block processing logic -- and the query
versions from V1 to V3.

This is the power of the snapshot model: **each subsystem version evolves at
its own pace.** A new protocol version does not require bumping everything. You
only change the sub-constants that actually differ.

Two annotation conventions make a version file reviewable:

- **The file's doc comment is the changelog.** `v14.rs` opens with a numbered
  list of every consensus change the version hosts, each with a paragraph on
  what it does and why it needed a protocol version. A change landing in the
  unreleased version adds an item there.
- **Every bumped slot gets a `// changed:` comment** saying what the new
  sub-constant does differently. A reviewer reading `PLATFORM_V14` top to
  bottom sees every behaviour difference from `PLATFORM_V13` without opening
  another file.

## What Belongs in the Snapshot

The snapshot holds three kinds of values, and the third is the one people
forget:

1. **Method versions.** `FeatureVersion` numbers that select a `v0`, `v1`, ...
   implementation. The next two chapters are about these.
2. **Format bounds.** `FeatureVersionBounds` for serialized structures: which
   structure versions a node accepts and which one it writes.
3. **Protocol parameters.** Plain numbers the code reads at runtime: size
   caps, per-block limits, penalty amounts, fee rates, retention windows,
   expiry heights. `SystemLimits`, `FeeVersion`,
   `DriveAbciWithdrawalConstants`, `DriveAbciValidationConstants` and
   `PenaltyAmounts` are all tables of these.

The rule for the third kind: **a constant that might change in a future
protocol version is declared in the version tables, not in an implementation
file.** Code reads it through `platform_version`. A `const MAX_SOMETHING: u16 =
50;` at the top of a Drive module is a value that cannot change without
editing shipped code, which is exactly what the versioning system exists to
avoid. A field on `SystemLimits` can change at a protocol-version boundary
with the old value preserved for replay.

Two consequences:

- Changing a number never creates a new method version. You change the table,
  and the method that reads the table keeps its version. See
  [Non-Method Version Fields](feature-versions.md#non-method-version-fields)
  for the recipe.
- Constants that genuinely cannot change stay in code: key and hash lengths,
  encoding widths, tree key bytes, anything whose change would be a new
  storage format rather than a new parameter. The test is whether a future
  protocol version could plausibly want a different value. If it could, it
  belongs in the tables from the start; moving it later means touching shipped
  code.

## The Get Dispatch

The most important function on `PlatformVersion` is `get`:

```rust
impl PlatformVersion {
    pub fn get<'a>(version: ProtocolVersion) -> Result<&'a Self, PlatformVersionError> {
        if version > 0 {
            PLATFORM_VERSIONS.get(version as usize - 1).ok_or_else(|| {
                PlatformVersionError::UnknownVersionError(
                    format!("no platform version {version}")
                )
            })
        } else {
            Err(PlatformVersionError::UnknownVersionError(
                format!("no platform version {version}")
            ))
        }
    }
}
```

This is a simple array lookup. Protocol version 1 maps to index 0, version 14
to index 13. If the version number is out of range, you get a clear error. No
hash maps, no runtime registration, no dynamic dispatch -- just a static array
of compile-time constants.

There are also convenience methods:

```rust
impl PlatformVersion {
    pub fn first<'a>() -> &'a Self {
        PLATFORM_VERSIONS.first()
            .expect("expected to have a platform version")
    }

    pub fn latest<'a>() -> &'a Self {
        PLATFORM_VERSIONS.last()
            .expect("expected to have a platform version")
    }

    pub fn desired<'a>() -> &'a Self {
        DESIRED_PLATFORM_VERSION
    }
}
```

`first()` is used in tests that need to verify behavior under the initial
protocol. `latest()` is the default for new code. `desired()` returns the
version that nodes *want* to upgrade to -- it equals `latest()` during normal
operation but could theoretically differ during a staged rollout.

## Version-Aware Traits

The `rs-platform-version` crate also defines traits that thread the platform
version through standard Rust conversion patterns:

```rust
// packages/rs-platform-version/src/lib.rs

pub trait TryFromPlatformVersioned<T>: Sized {
    type Error;

    fn try_from_platform_versioned(
        value: T,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error>;
}

pub trait DefaultForPlatformVersion: Sized {
    type Error;

    fn default_for_platform_version(
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error>;
}
```

These are the versioned equivalents of `TryFrom` and `Default`. When you
convert a data structure, you pass the platform version so the implementation
can pick the right serialization format, the right field set, or the right
validation rules. There is also `FromPlatformVersioned` for infallible
conversions, and blanket `IntoPlatformVersioned` implementations that mirror
the standard library pattern.

## Mock Versions for Testing

The version system supports a `mock-versions` feature flag for tests:

```rust
#[cfg(feature = "mock-versions")]
pub static PLATFORM_TEST_VERSIONS: OnceLock<Vec<PlatformVersion>> = OnceLock::new();
```

When this feature is enabled, `PlatformVersion::get` checks for a special bit
in the version number. If set, it routes to the test version array instead of
the production one. This lets tests create synthetic platform versions with
specific behaviors without polluting the production constants:

```rust
#[cfg(feature = "mock-versions")]
{
    if version >> TEST_PROTOCOL_VERSION_SHIFT_BYTES > 0 {
        let test_version = version - (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES);
        let versions = PLATFORM_TEST_VERSIONS
            .get_or_init(|| vec![TEST_PLATFORM_V2, TEST_PLATFORM_V3]);
        return versions.get(test_version as usize - 2).ok_or(/* ... */);
    }
}
```

This is a clever design: tests can exercise version upgrade logic (like
"what happens when we transition from test version 2 to test version 3?")
without needing to create real protocol versions.

`TEST_PLATFORM_V4` (`version/mocks/v4_test.rs`) is the latest shipped tables
with one substitution: its fee schedule is `TEST_FEE_VERSION_DOUBLED_STORAGE`
(`version/mocks/fee_doubled_storage_test.rs`), the latest schedule with a new
`fee_version_number` and a doubled storage disk usage rate. No shipped schedule
carries a number other than 1, so this is the only fee generation that
exercises the registry lookup, the epoch-change hook, the saved-state round
trip and the history-driven refund path. Its number sits in the same shifted
range as the mock protocol versions (`(1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES)
+ 1`), and `FeeVersion::get` resolves that range through the test registry only
under `mock-versions`: a production node that finds such a number in its saved
state rejects it instead of falling back to a real schedule. When a new
protocol version ships, move the mock's base to its table so it keeps tracking
the latest behaviour.

## Why Immutable Snapshots?

You might wonder: why not use a mutable configuration object? Why not a
`HashMap<&str, u16>` that maps method names to versions?

Three reasons:

1. **Determinism.** A `const` value is baked into the binary at compile time.
   There is no way to accidentally modify it at runtime. Every node running the
   same binary with the same protocol version will use the exact same values.

2. **Exhaustiveness.** Because the version struct has named fields for every
   subsystem, adding a new versioned method forces you to set its version in
   *every* platform version constant. The compiler will refuse to compile if
   you forget one. A hash map cannot give you this guarantee.

3. **Performance.** Looking up a version number is a struct field access --
   zero overhead at runtime. The entire version tree lives in static memory.
   No allocations, no lookups, no indirection.

The cost is verbosity. Each new platform version file is large and repetitive.
But this is a deliberate trade-off: the system favors **correctness and
auditability** over conciseness. When you read `PLATFORM_V14`, you can see
every single version number in one place. There is no mystery about what
version 14 means.

## Rules

**Do:**
- Always pass `&PlatformVersion` (or `&DriveVersion`, etc.) to functions that
  have versioned behavior. Never hardcode a version number at a call site.
- Use `PlatformVersion::latest()` for tests of the current generation. Pin an
  explicit version with `PlatformVersion::get(n)` only for tests of a frozen,
  older generation, and use `PlatformVersion::first()` when you need the
  initial protocol behavior.
- Create `vN.rs` once, when the first consensus change after version N-1
  ships needs a home: copy the previous version file and change only the
  constants that differ. Every later change destined for version N amends
  that file: add a numbered item to its doc comment and a `// changed:`
  comment on each slot it bumps.
- Declare any constant that might change in a future protocol version in the
  version tables (`SystemLimits`, `FeeVersion`, the `*Constants` structs) and
  read it through `platform_version`. Keep only genuinely invariant values as
  `const` items in implementation files.

**Do not:**
- Never mutate platform version data at runtime. The constants are `const` for
  a reason.
- Never edit a released `vN.rs`, or any table constant it references. Once a
  protocol version has run on the network its snapshot is part of the chain's
  history; a change there makes a fresh node replay history differently from
  every node that was there at the time.
- Never add a new field to `PlatformVersion` without also updating every
  `PLATFORM_V*` constant. The compiler will enforce this, but be aware that
  the fix is updating fourteen files, not one.
- Never use `PlatformVersion::latest()` in consensus-critical code paths.
  Always use the version from the current platform state, obtained via
  `platform_state.current_platform_version()`. The "latest" version is what
  the *binary* supports; the *active* version is what the network has agreed
  upon -- and they may differ during an upgrade window.
