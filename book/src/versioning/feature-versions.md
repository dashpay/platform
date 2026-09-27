# Feature Versions

## The Problem: Granularity

The previous chapter showed how `PlatformVersion` is an immutable snapshot of
the entire platform's behavior at a given protocol version. But a snapshot is
only useful if it can describe behavior at a fine enough granularity.

Consider the Drive storage layer. It has dozens of grove operations, hundreds
of document methods, contract methods, identity methods, and more. When you
fix a bug in `update_contract`, you need to bump *that one method's* version
without affecting `insert_contract` or `prove_contract`. The system needs a
way to assign a version number to individual methods and then compose those
numbers into larger subsystem snapshots.

This is where `FeatureVersion` and the nested version structs come in.

## The FeatureVersion Type

At the very bottom of the version tree is a single type, defined in the
external `versioned-feature-core` crate:

```rust
// versioned-feature-core/src/lib.rs

pub type FeatureVersion = u16;
pub type OptionalFeatureVersion = Option<u16>;
```

That is it. A `FeatureVersion` is a `u16` -- a number that says "use version N
of this particular function." The value `0` means "use the v0 implementation,"
`1` means "use v1," and so on.

`OptionalFeatureVersion` is `Option<u16>`. It represents a feature that did not
exist in earlier protocol versions. When the value is `None`, the feature is
not active -- calling it returns a `VersionNotActive` error. When it is
`Some(0)`, the feature exists and should use its v0 implementation.

There is also a bounds type for serialization format versions:

```rust
#[derive(Clone, Debug, Default)]
pub struct FeatureVersionBounds {
    pub min_version: FeatureVersion,
    pub max_version: FeatureVersion,
    pub default_current_version: FeatureVersion,
}
```

This is used when a field can accept a *range* of versions -- for example, a
data contract serialization format where the system can read versions 0 through
2 but writes version 2 by default.

## Version Structs: The Middle of the Tree

Between the top-level `PlatformVersion` and the leaf-level `FeatureVersion`
numbers sit dozens of intermediate structs. These structs group related method
versions together, forming a hierarchy that mirrors the codebase's module
structure.

Let us trace a path from the top down.

### Level 1: PlatformVersion

```rust
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

### Level 2: DriveVersion

The `drive` field contains `DriveVersion`, which groups all storage layer
versions:

```rust
// packages/rs-platform-version/src/version/drive_versions/mod.rs

#[derive(Clone, Debug, Default)]
pub struct DriveVersion {
    pub structure: DriveStructureVersion,
    pub methods: DriveMethodVersions,
    pub grove_methods: DriveGroveMethodVersions,
    pub grove_version: GroveVersion,
}
```

### Level 3: DriveMethodVersions

The `methods` field expands into every category of Drive operation:

```rust
#[derive(Clone, Debug, Default)]
pub struct DriveMethodVersions {
    pub initialization: DriveInitializationMethodVersions,
    pub credit_pools: DriveCreditPoolMethodVersions,
    pub protocol_upgrade: DriveProtocolUpgradeVersions,
    pub prefunded_specialized_balances: DrivePrefundedSpecializedMethodVersions,
    pub balances: DriveBalancesMethodVersions,
    pub document: DriveDocumentMethodVersions,
    pub vote: DriveVoteMethodVersions,
    pub contract: DriveContractMethodVersions,
    pub fees: DriveFeesMethodVersions,
    pub estimated_costs: DriveEstimatedCostsMethodVersions,
    pub asset_lock: DriveAssetLockMethodVersions,
    pub verify: DriveVerifyMethodVersions,
    pub identity: DriveIdentityMethodVersions,
    pub token: DriveTokenMethodVersions,
    pub platform_system: DrivePlatformSystemMethodVersions,
    pub operations: DriveOperationsMethodVersion,
    pub batch_operations: DriveBatchOperationsMethodVersion,
    pub fetch: DriveFetchMethodVersions,
    pub prove: DriveProveMethodVersions,
    pub state_transitions: DriveStateTransitionMethodVersions,
    pub platform_state: DrivePlatformStateMethodVersions,
    pub group: DriveGroupMethodVersions,
    pub address_funds: DriveAddressFundsMethodVersions,
    pub saved_block_transactions: DriveSavedBlockTransactionsMethodVersions,
}
```

### Level 4: Individual Method Categories

Each category struct contains `FeatureVersion` fields for individual methods.
For example, the contract method versions:

```rust
#[derive(Clone, Debug, Default)]
pub struct DriveContractMethodVersions {
    pub prove: DriveContractProveMethodVersions,
    pub apply: DriveContractApplyMethodVersions,
    pub insert: DriveContractInsertMethodVersions,
    pub update: DriveContractUpdateMethodVersions,
    pub costs: DriveContractCostsMethodVersions,
    pub get: DriveContractGetMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveContractUpdateMethodVersions {
    pub update_contract: FeatureVersion,
    pub update_description: FeatureVersion,
    pub update_keywords: FeatureVersion,
}
```

So the full path to read "which version of `update_contract` should I use?" is:

```rust
platform_version.drive.methods.contract.update.update_contract
```

That is a five-level deep field access, and it resolves to a plain `u16`.

## The Grove Methods Branch

Let us trace a different path. The `grove_methods` field on `DriveVersion`
holds versions for low-level GroveDB operations:

```rust
#[derive(Clone, Debug, Default)]
pub struct DriveGroveMethodVersions {
    pub basic: DriveGroveBasicMethodVersions,
    pub batch: DriveGroveBatchMethodVersions,
    pub apply: DriveGroveApplyMethodVersions,
    pub costs: DriveGroveCostMethodVersions,
}
```

The `basic` struct is where individual grove operations live:

```rust
#[derive(Clone, Debug, Default)]
pub struct DriveGroveBasicMethodVersions {
    pub grove_insert: FeatureVersion,
    pub grove_insert_empty_tree: FeatureVersion,
    pub grove_insert_if_not_exists: FeatureVersion,
    pub grove_clear: FeatureVersion,
    pub grove_delete: FeatureVersion,
    pub grove_get_raw: FeatureVersion,
    pub grove_get_raw_optional: FeatureVersion,
    pub grove_get: FeatureVersion,
    pub grove_get_path_query: FeatureVersion,
    pub grove_get_proved_path_query: FeatureVersion,
    pub grove_get_sum_tree_total_value: FeatureVersion,
    pub grove_has_raw: FeatureVersion,
    // ... and many more
}
```

So the path for `grove_get_raw` is:

```rust
drive_version.grove_methods.basic.grove_get_raw
```

Notice something: grove operations take a `&DriveVersion` rather than
`&PlatformVersion`. This is a minor optimization -- when you are deep in the
Drive layer, you only need the drive-specific version numbers, not the entire
platform snapshot. The caller extracts `&platform_version.drive` once and
passes it down.

## The DPP Branch

The Dash Platform Protocol has its own deep tree. `DPPVersion` contains
fourteen sub-version structs:

```rust
#[derive(Clone, Debug, Default)]
pub struct DPPVersion {
    pub costs: DPPCostsVersions,
    pub validation: DPPValidationVersions,
    pub state_transition_serialization_versions: DPPStateTransitionSerializationVersions,
    pub state_transition_conversion_versions: DPPStateTransitionConversionVersions,
    pub state_transition_method_versions: DPPStateTransitionMethodVersions,
    pub state_transitions: DPPStateTransitionVersions,
    pub contract_versions: DPPContractVersions,
    pub document_versions: DPPDocumentVersions,
    pub identity_versions: DPPIdentityVersions,
    pub voting_versions: DPPVotingVersions,
    pub token_versions: DPPTokenVersions,
    pub asset_lock_versions: DPPAssetLockVersions,
    pub methods: DPPMethodVersions,
    pub factory_versions: DPPFactoryVersions,
}
```

And those go deeper. For example, `DPPContractVersions` contains not just
`FeatureVersion` values but also `FeatureVersionBounds` and further nesting:

```rust
#[derive(Clone, Debug, Default)]
pub struct DPPContractVersions {
    pub max_serialized_size: u32,
    pub contract_serialization_version: FeatureVersionBounds,
    pub contract_structure_version: FeatureVersion,
    pub created_data_contract_structure: FeatureVersion,
    pub config: FeatureVersionBounds,
    pub methods: DataContractMethodVersions,
    pub document_type_versions: DocumentTypeVersions,
    pub token_versions: TokenVersions,
}
```

Notice `max_serialized_size: u32`. Not every field is a `FeatureVersion`. Some
are configuration values -- limits, thresholds, constants -- that change between
protocol versions. The version struct is flexible enough to hold both "which
implementation to use" and "what parameters to use."

## The Drive ABCI Branch

The `DriveAbciVersion` struct covers the application blockchain interface --
the layer that processes blocks, validates state transitions, and handles
protocol upgrades:

```rust
#[derive(Clone, Debug, Default)]
pub struct DriveAbciVersion {
    pub structs: DriveAbciStructureVersions,
    pub methods: DriveAbciMethodVersions,
    pub validation_and_processing: DriveAbciValidationVersions,
    pub withdrawal_constants: DriveAbciWithdrawalConstants,
    pub query: DriveAbciQueryVersions,
    pub checkpoints: DriveAbciCheckpointParameters,
}
```

The `validation_and_processing` field is where state transition validation
versions live. This is where `OptionalFeatureVersion` becomes important:

```rust
#[derive(Clone, Debug, Default)]
pub struct DriveAbciStateTransitionValidationVersion {
    pub basic_structure: OptionalFeatureVersion,
    pub advanced_structure: OptionalFeatureVersion,
    pub identity_signatures: OptionalFeatureVersion,
    pub nonce: OptionalFeatureVersion,
    pub state: FeatureVersion,
    pub transform_into_action: FeatureVersion,
}
```

`basic_structure` is `OptionalFeatureVersion` -- in some protocol versions,
basic structure validation may not exist for a particular state transition. The
dispatch code handles this with a three-arm match:

```rust
match platform_version
    .drive_abci
    .validation_and_processing
    .state_transitions
    .identity_create_state_transition
    .basic_structure
{
    Some(0) => self.validate_basic_structure_v0(platform_version),
    Some(version) => Err(Error::Execution(
        ExecutionError::UnknownVersionMismatch { /* ... */ }
    )),
    None => Err(Error::Execution(
        ExecutionError::VersionNotActive { /* ... */ }
    )),
}
```

Compare with `state` and `transform_into_action` which are plain
`FeatureVersion` -- those validations always exist, so there is no `None` arm.

## Non-Method Version Fields

Some version structs contain values that are not method versions at all, but
protocol parameters. `SystemLimits` is the main one:

```rust
// packages/rs-platform-version/src/version/system_limits/mod.rs

#[derive(Clone, Debug, Default)]
pub struct SystemLimits {
    pub estimated_contract_max_serialized_size: u16,
    pub max_field_value_size: u32,
    /// `None` preserves the behavior of protocol versions that predate this limit.
    pub max_document_value_depth: Option<u16>,
    pub max_state_transition_size: u64,
    pub max_transitions_in_documents_batch: u16,
    pub withdrawal_transactions_per_block_limit: u16,
    pub retry_signing_expired_withdrawal_documents_per_block_limit: u16,
    pub max_withdrawal_amount: u64,
    /// `None` for the protocol versions that predate the relative rule.
    pub daily_withdrawal_limit_percent: Option<u8>,
    pub max_daily_withdrawal_amount: Option<u64>,
    pub min_withdrawal_amount: u64,
    pub max_contract_group_size: u16,
    pub max_token_redemption_cycles: u32,
    pub max_shielded_transition_actions: u16,
    pub max_time_range_overlap_factor: Option<u64>,
}
```

There are four `SYSTEM_LIMITS_V*` constants, one for each protocol version at
which a limit changed. The `Option` fields show the idiom for a parameter that
did not exist before some version: `None` in the tables of the versions that
predate the rule, `Some(value)` from the version that introduced it. It is the
parameter-shaped twin of `OptionalFeatureVersion`.

The same shape recurs wherever a subsystem owns tunables:

```rust
// drive_abci_versions/drive_abci_withdrawal_constants/mod.rs
pub struct DriveAbciWithdrawalConstants {
    pub core_expiration_blocks: u32,
    pub cleanup_expired_locks_of_withdrawal_amounts_limit: u16,
    pub total_credits_history_prune_limit: u16,
}

// drive_abci_versions/drive_abci_validation_versions/mod.rs
pub struct PenaltyAmounts {
    pub identity_id_not_correct: u64,
    pub unique_key_already_present: u64,
    // ...
    pub shielded_proof_verification_failure: u64,
}

pub struct DriveAbciCoreChainLockMethodVersionsAndConstants {
    pub choose_quorum: FeatureVersion,
    pub verify_chain_lock: FeatureVersion,
    // ...
    pub recent_block_count_amount: u32,
}
```

The chain lock struct mixes method versions (`choose_quorum: FeatureVersion`)
with protocol constants (`recent_block_count_amount: u32`). This is perfectly
fine -- the version snapshot captures *all* protocol-specific values, whether
they control dispatch or configure behavior. Fee rates follow the same idea
one level up: `FeeVersion` is a table of numbers, and a fee change is a new
named `FEE_VERSION*` schedule referenced from the platform version, never an
inline override inside `PLATFORM_V*`.

### Changing a number

Because parameters live in tables, changing one is a table edit and not a
method version:

1. If the field does not exist yet, add it to the struct with a doc comment
   naming the method version that reads it.
2. Backfill every shipped table constant (and the mock tables under
   `version/mocks/`) with the old value, so released protocol versions keep
   their behaviour. The compiler forces this step.
3. Add the next table constant for the unreleased protocol version with the
   new value, and point that version's `PLATFORM_V*` at it. If the unreleased
   version already introduced a new table constant, edit that one in place
   instead.
4. Make the method read the field through `platform_version`. The method keeps
   its version number: shipped versions read their old value from their own
   table, so their behaviour is unchanged.

Do not create a `vN` method module whose entire body returns the new
constant. That hides a protocol parameter in an implementation file, which is
the situation the tables exist to prevent, and it leaves a dead module behind
every time the number moves. A new method version is warranted only when the
*logic* changes. `daily_withdrawal_limit` in `rs-dpp` is the reference case:
`v0` derives the limit from the current total credits, `v2` reads
`daily_withdrawal_limit_percent` and `max_daily_withdrawal_amount` from
`SystemLimits`. Raising the percentage later is a `SYSTEM_LIMITS_V5`, not a
`v3`.

## How Subsystem Version Constants Compose

Each subsystem version constant (like `DRIVE_VERSION_V1`) is assembled from
smaller constants:

```rust
// packages/rs-platform-version/src/version/drive_versions/v1.rs

pub const DRIVE_VERSION_V1: DriveVersion = DriveVersion {
    structure: DRIVE_STRUCTURE_V1,
    methods: DriveMethodVersions {
        initialization: DriveInitializationMethodVersions {
            create_initial_state_structure: 0,
        },
        credit_pools: CREDIT_POOL_METHOD_VERSIONS_V1,
        protocol_upgrade: DriveProtocolUpgradeVersions {
            clear_version_information: 0,
            fetch_versions_with_counter: 0,
            // ...
        },
        balances: DriveBalancesMethodVersions {
            add_to_system_credits: 0,
            remove_from_system_credits: 0,
            calculate_total_credits_balance: 0,
            // ...
        },
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V1,
        // ...
    },
    grove_methods: DRIVE_GROVE_METHOD_VERSIONS_V1,
    grove_version: GROVE_V1,
};
```

Notice the mix of inline construction and named constants. Small structs like
`DriveProtocolUpgradeVersions` are often written inline because all their
fields are `0` in every version. Larger, frequently-changing structs like
`DRIVE_CONTRACT_METHOD_VERSIONS_V1` get their own named constant so they can
be reused or overridden in later versions.

When `DRIVE_VERSION_V9` (used in `PLATFORM_V14`) needs new document method
versions, it references `DRIVE_DOCUMENT_METHOD_VERSIONS_V4` where V8
referenced V3, and annotates the slot:

```rust
// packages/rs-platform-version/src/version/drive_versions/v9.rs

pub const DRIVE_VERSION_V9: DriveVersion = DriveVersion {
    structure: DRIVE_STRUCTURE_V1,
    methods: DriveMethodVersions {
        // ...
        document: DRIVE_DOCUMENT_METHOD_VERSIONS_V4, // changed in v9: v2 index walkers + v1 update walker
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V3, // changed in v8: count-tree-aware contract-insertion cost estimation
        // ...
        verify: DRIVE_VERIFY_METHOD_VERSIONS_V2, // changed in v8: compacted address-balance proof envelope
        identity: DRIVE_IDENTITY_METHOD_VERSIONS_V2, // changed in v9: v1 withdrawal-by-transaction-index query builder
        // ...
    },
    grove_methods: DRIVE_GROVE_METHOD_VERSIONS_V1,
    // changed in v9: GROVE_V4 activates the indexed-tree batch cleanup
    grove_version: GROVE_V4,
};
```

The unchanged parts reference the same constants they always did. Only the
parts that actually changed get new constants. `grove_version` is how GroveDB
behaviour is versioned from Platform's side: a new `DRIVE_VERSION_V*` points
at a new `GROVE_V*`, and every GroveDB call receives it.

### Two ways to write a new table constant

Older table files are full copies of the previous version with the changed
slots edited. Newer ones use struct update syntax, so that the file *is* the
diff:

```rust
// packages/rs-platform-version/src/version/drive_abci_versions/drive_abci_query_versions/v3.rs

/// Differs from v2 in exactly one slot:
/// `document_query_helpers.compute_aggregate_mode_and_check_limit` is 2
/// rather than 1. That is the boolean-`HAVING` routing gate. ...
pub const DRIVE_ABCI_QUERY_VERSIONS_V3: DriveAbciQueryVersions = DriveAbciQueryVersions {
    document_query_helpers: DriveAbciDocumentQueryHelperVersions {
        compute_aggregate_mode_and_check_limit: 2,
    },
    ..DRIVE_ABCI_QUERY_VERSIONS_V2
};
```

It nests, so one changed leaf deep in a table still reads as one line:

```rust
// packages/rs-platform-version/src/version/dpp_versions/dpp_validation_versions/v5.rs

pub const DPP_VALIDATION_VERSIONS_V5: DPPValidationVersions = DPPValidationVersions {
    document_type: DocumentTypeValidationVersions {
        validate_update: 1,
        ..DPP_VALIDATION_VERSIONS_V4.document_type
    },
    ..DPP_VALIDATION_VERSIONS_V4
};
```

Use the struct update form for new table constants. Either way, the doc
comment on the constant names the slots that differ from the previous table
and why, in the same spirit as the `// changed:` comments on `PLATFORM_V*`.

### When to create a new table constant

Table constants mark the boundaries between released protocol versions. Create
`DRIVE_ABCI_QUERY_VERSIONS_V4` when the constant you would otherwise edit is
referenced by a *released* `PLATFORM_V*`. If the unreleased protocol version
already introduced a new constant for that table, a second feature landing in
the same protocol version amends it in place. Otherwise the crate accumulates
table versions that no protocol version references, and the numbering stops
saying anything about what shipped when.

## The File Layout

The version structs follow a consistent directory layout in
`packages/rs-platform-version/src/version/`:

```
version/
  mod.rs                        # ProtocolVersion type alias, LATEST_VERSION, module declarations
  protocol_version.rs           # PlatformVersion struct, PLATFORM_VERSIONS array, get()
  consensus_versions.rs         # ConsensusVersions (Tenderdash consensus version)
  feature_initial_protocol_versions.rs  # first protocol version of whole new transition kinds
  v1.rs .. v14.rs               # PLATFORM_V* snapshot constants, one per protocol version
  drive_versions/
    mod.rs                      # DriveVersion, DriveMethodVersions, etc.
    v1.rs .. v9.rs              # DRIVE_VERSION_V* constants
    drive_grove_method_versions/
      mod.rs                    # DriveGroveMethodVersions struct
      v1.rs                     # DRIVE_GROVE_METHOD_VERSIONS_V1
    drive_contract_method_versions/
      mod.rs                    # DriveContractMethodVersions struct
      v1.rs .. v3.rs            # versioned constants
    drive_document_method_versions/
      mod.rs
      v1.rs .. v4.rs
    drive_verify_method_versions/
      mod.rs
      v1.rs, v2.rs
    ...
  drive_abci_versions/
    mod.rs                      # DriveAbciVersion struct
    drive_abci_method_versions/
      mod.rs                    # DriveAbciMethodVersions and sub-structs
      v1.rs .. v10.rs           # versioned constants
    drive_abci_validation_versions/
      mod.rs                    # DriveAbciValidationVersions, PenaltyAmounts, validation constants
      v1.rs .. v10.rs
    drive_abci_query_versions/
      mod.rs
      v0.rs .. v3.rs
    drive_abci_withdrawal_constants/
      mod.rs                    # DriveAbciWithdrawalConstants (parameters, not method versions)
      v1.rs .. v3.rs
    drive_abci_structure_versions/      v1.rs
    drive_abci_checkpoint_parameters/   v1.rs
  dpp_versions/
    mod.rs                      # DPPVersion struct
    dpp_contract_versions/
      mod.rs                    # DPPContractVersions struct
      v1.rs .. v6.rs
    dpp_validation_versions/
      mod.rs
      v1.rs .. v5.rs
    ...
  fee/
    mod.rs                      # FeeVersion struct, FEE_VERSIONS
    v1.rs, v2.rs                # FEE_VERSION* schedules
    storage/, signature/, processing/, ...   # per-group tables, each with its own v*.rs
  system_limits/
    mod.rs                      # SystemLimits struct
    v1.rs .. v4.rs
  system_data_contract_versions/
    mod.rs
    v1.rs .. v3.rs
  mocks/
    v2_test.rs, v3_test.rs      # TEST_PLATFORM_V* behind the mock-versions feature
```

The pattern is: `mod.rs` defines the struct, and `v*.rs` files define the
concrete constants. The struct definition is the *schema*. The version files
are the *data*. Note that each table's `v*` numbering is its own:
`DRIVE_VERSION_V9` is what `PLATFORM_V14` uses, and `SYSTEM_LIMITS_V4`
likewise. The number says how many times that table has changed, not which
protocol version it belongs to.

## Rules

**Do:**
- When adding a new method to Drive, DPP, or Drive ABCI, add a corresponding
  `FeatureVersion` field to the appropriate version struct. Then set its value
  in every `v*.rs` constant -- the compiler will force you.
- Use `OptionalFeatureVersion` for features that are being introduced in a
  non-initial protocol version. Set them to `None` in earlier versions and
  `Some(0)` in the version that introduces the feature. Use an `Option` field
  the same way for a parameter that did not exist before some version.
- Declare every protocol parameter that might change (limits, penalties,
  retention windows, fee rates) as a table field and read it through
  `platform_version`. Change a number by adding a table version, not a method
  version.
- Write new table constants with struct update syntax (`..PREVIOUS`) and a doc
  comment naming the slots that differ and why.
- Group related methods into their own sub-struct when the parent struct
  grows too large. Follow the existing naming pattern:
  `Drive<Category>MethodVersions`.

**Do not:**
- Never use a raw `u16` where you mean `FeatureVersion`. The type alias exists
  for readability and future-proofing -- if we ever need to change the
  underlying type, the alias is the single point of change.
- Never put runtime-computed values into a version struct. Every field must be
  a compile-time constant. This is what makes the snapshot deterministic.
- Never reuse a version constant with different semantics. If
  `DRIVE_CONTRACT_METHOD_VERSIONS_V1` means something, creating a V2 that
  changes one field is correct. Silently modifying V1 is not -- it would change
  the behavior of every platform version that references it.
- Never create a new table constant for a second change landing in the same
  unreleased protocol version. Table versions mark released boundaries; amend
  the unreleased version's own constant in place.
- Never leave a `const` in an implementation file that a future protocol
  version might want to change. Move it to the tables the first time it is
  touched.
