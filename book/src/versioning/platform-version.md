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
the time of writing, the platform has twelve versions:

```rust
// packages/rs-platform-version/src/version/mod.rs

pub type ProtocolVersion = u32;

pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_12;
pub const INITIAL_PROTOCOL_VERSION: ProtocolVersion = 1;
pub const ALL_VERSIONS: RangeInclusive<ProtocolVersion> = 1..=LATEST_VERSION;
```

These twelve snapshots are collected into a single static array in
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
];

pub const LATEST_PLATFORM_VERSION: &PlatformVersion = &PLATFORM_V12;
pub const DESIRED_PLATFORM_VERSION: &PlatformVersion = LATEST_PLATFORM_VERSION;
```

The array is indexed by protocol version number minus one (since versions are
1-indexed). `PLATFORM_V1` sits at index 0, `PLATFORM_V12` at index 11. This
simple layout is what makes the `get` function so fast.

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

Now compare with `PLATFORM_V12`, the latest at the time of writing:

```rust
// packages/rs-platform-version/src/version/v12.rs

pub const PLATFORM_V12: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_12,
    drive: DRIVE_VERSION_V6,          // was V1
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V7,   // was V1
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V7, // was V1
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2,     // was V1
        query: DRIVE_ABCI_QUERY_VERSIONS_V1,
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V2,  // was V1
        state_transitions: STATE_TRANSITION_VERSIONS_V3, // was V1
        contract_versions: CONTRACT_VERSIONS_V3,         // was V1
        document_versions: DOCUMENT_VERSIONS_V3,         // was V1
        // ... other fields, some still V1, some bumped
        methods: DPP_METHOD_VERSIONS_V2,                 // was V1
        factory_versions: DPP_FACTORY_VERSIONS_V1,
        // ...
    },
    fee_version: FEE_VERSION2,   // was VERSION1
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,  // was 0
    },
    // ...
};
```

Notice how only some subsystem versions change between V1 and V12. The query
versions stayed at V1 across all twelve protocol versions because the query
logic never changed. The ABCI method versions, on the other hand, went from
V1 all the way to V7 -- seven revisions of the block processing logic.

This is the power of the snapshot model: **each subsystem version evolves at
its own pace.** A new protocol version does not require bumping everything. You
only change the sub-constants that actually differ.

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

This is a simple array lookup. Protocol version 1 maps to index 0, version 12
to index 11. If the version number is out of range, you get a clear error. No
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
auditability** over conciseness. When you read `PLATFORM_V12`, you can see
every single version number in one place. There is no mystery about what
version 12 means.

## Rules

**Do:**
- Always pass `&PlatformVersion` (or `&DriveVersion`, etc.) to functions that
  have versioned behavior. Never hardcode a version number at a call site.
- Use `PlatformVersion::latest()` for tests that do not care about a specific
  version. Use `PlatformVersion::first()` when you need to test the initial
  protocol behavior.
- When creating a new platform version, copy the previous version file and
  change only the constants that differ. The compiler will catch any missing
  fields.

**Do not:**
- Never mutate platform version data at runtime. The constants are `const` for
  a reason.
- Never add a new field to `PlatformVersion` without also updating every
  `PLATFORM_V*` constant. The compiler will enforce this, but be aware that
  the fix is updating twelve files, not one.
- Never use `PlatformVersion::latest()` in consensus-critical code paths.
  Always use the version from the current platform state, obtained via
  `platform_state.current_platform_version()`. The "latest" version is what
  the *binary* supports; the *active* version is what the network has agreed
  upon -- and they may differ during an upgrade window.
