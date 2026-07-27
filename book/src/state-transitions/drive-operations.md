# Drive Operations: From Action to Storage

By this point in the pipeline, a state transition has been validated and transformed
into a `StateTransitionAction`. But the action is still an abstract description of
*what* should change. The final step is translating it into concrete storage mutations
that get applied atomically to GroveDB, the platform's Merkle tree database. This
translation happens through a three-tier pipeline of progressively lower-level
operation types.

## The Three-Tier Pipeline

The pipeline looks like this:

```
StateTransitionAction
    |
    | DriveHighLevelOperationConverter::into_high_level_drive_operations()
    v
Vec<DriveOperation>
    |
    | DriveLowLevelOperationConverter::into_low_level_drive_operations()
    v
Vec<LowLevelDriveOperation>
    |
    | apply_batch_low_level_drive_operations()
    v
GroveDB (applied atomically)
```

Each tier exists for a reason:

- **StateTransitionAction** speaks the language of the protocol: "create this identity,"
  "store this document," "transfer these credits."
- **DriveOperation** speaks the language of Drive's domain model: "insert a document
  into this contract's document type tree," "update an identity's balance."
- **LowLevelDriveOperation** speaks the language of GroveDB: "insert this key-value
  pair at this path," "delete this element."

This layering allows each tier to be tested independently and evolved separately.
A change to how documents are indexed in GroveDB only affects the
`DriveLowLevelOperationConverter` implementation for `DocumentOperationType` -- it does
not ripple up to the action layer.

## Tier 1: DriveHighLevelOperationConverter

The first conversion step is defined in
`packages/rs-drive/src/state_transition_action/action_convert_to_operations/mod.rs`:

```rust
pub trait DriveHighLevelOperationConverter {
    fn into_high_level_drive_operations<'a>(
        self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error>;
}
```

The `StateTransitionAction` enum implements this trait by dispatching to each variant:

```rust
impl DriveHighLevelOperationConverter for StateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            StateTransitionAction::DataContractCreateAction(action) => {
                action.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::DataContractUpdateAction(action) => {
                action.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::BatchAction(action) => {
                action.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::IdentityCreateAction(action) => {
                action.into_high_level_drive_operations(epoch, platform_version)
            }
            // ... all other variants
        }
    }
}
```

Each action type knows how to decompose itself into the appropriate `DriveOperation`
variants. A single action often produces *multiple* drive operations. For example,
`IdentityCreateAction` might produce:

1. An `IdentityOperation` to insert the identity
2. An `IdentityOperation` to set the initial balance
3. Multiple `IdentityOperation`s to add each public key
4. A `SystemOperation` to update system credit tracking

For the `BatchTransitionAction`, there is an additional layer of delegation. The batch
contains multiple `BatchedTransitionAction` items, each of which implements
`DriveHighLevelBatchOperationConverter`:

```rust
pub trait DriveHighLevelBatchOperationConverter {
    fn into_high_level_batch_drive_operations<'a>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error>;
}
```

Notice the extra `owner_id` parameter -- batch operations need to know which identity
owns the documents being created or modified.

## Tier 2: The DriveOperation Enum

The `DriveOperation` enum represents domain-level storage operations. It is defined in
`packages/rs-drive/src/util/batch/drive_op_batch/mod.rs`:

```rust
pub enum DriveOperation<'a> {
    DataContractOperation(DataContractOperationType<'a>),
    DocumentOperation(DocumentOperationType<'a>),
    TokenOperation(TokenOperationType),
    WithdrawalOperation(WithdrawalOperationType),
    IdentityOperation(IdentityOperationType),
    PrefundedSpecializedBalanceOperation(PrefundedSpecializedBalanceOperationType),
    SystemOperation(SystemOperationType),
    GroupOperation(GroupOperationType),
    AddressFundsOperation(AddressFundsOperationType),
    GroveDBOperation(QualifiedGroveDbOp),
    GroveDBOpBatch(GroveDbOpBatch),
}
```

Each variant wraps a type-specific operation enum. For example, `DocumentOperationType`
includes operations like `AddDocument`, `UpdateDocument`, `DeleteDocument` -- each
carrying the document data, contract reference, and storage flags needed for insertion.

The last two variants -- `GroveDBOperation` and `GroveDBOpBatch` -- are escape hatches
for when higher-level abstractions are not needed. They wrap raw GroveDB operations
directly.

The `DriveOperation` enum implements the `DriveLowLevelOperationConverter` trait to
convert itself into the next tier:

```rust
pub trait DriveLowLevelOperationConverter {
    fn into_low_level_drive_operations(
        self,
        drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error>;
}
```

Two important parameters here:

- **`estimated_costs_only_with_layer_info`**: When this is `Some`, the converter does
  not actually read from or write to GroveDB. Instead, it estimates the cost of the
  operations using layer information. This is used for fee estimation before execution.
- **`transaction`**: The GroveDB transaction handle. All reads and writes within a
  single block happen within one transaction, ensuring atomicity.

## Tier 3: LowLevelDriveOperation

The lowest tier is defined in `packages/rs-drive/src/fees/op.rs`:

```rust
pub enum LowLevelDriveOperation {
    GroveOperation(QualifiedGroveDbOp),
    FunctionOperation(FunctionOp),
    CalculatedCostOperation(OperationCost),
    PreCalculatedFeeResult(FeeResult),
}
```

At this level, there are only four kinds of things:

- **GroveOperation**: A concrete GroveDB operation -- insert, delete, or update a
  key-value pair at a specific path in the Merkle tree.
- **FunctionOperation**: A CPU-bound operation with a pre-defined cost (like hashing
  or signature verification). These do not touch storage but still cost processing fees.
- **CalculatedCostOperation**: A pre-computed cost that gets folded into the fee
  calculation.
- **PreCalculatedFeeResult**: An already-computed fee result, used when the fee for
  an operation was determined earlier in the pipeline.

The `GroveOperation` variant is where the rubber meets the road.
`QualifiedGroveDbOp` is GroveDB's own batch operation type -- it specifies a path
(a vector of byte-string segments navigating the tree), a key, and an operation
(insert element, delete, replace, etc.).

## Applying the Batch

The entire sequence -- from `DriveOperation` collection to GroveDB application --
is orchestrated by `Drive::apply_drive_operations`, defined in
`packages/rs-drive/src/util/batch/drive_op_batch/drive_methods/apply_drive_operations/v0/mod.rs`:

```rust
pub(crate) fn apply_drive_operations_v0(
    &self,
    operations: Vec<DriveOperation>,
    apply: bool,
    block_info: &BlockInfo,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
    previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
) -> Result<FeeResult, Error> {
    if operations.is_empty() {
        return Ok(FeeResult::default());
    }
    let mut low_level_operations = vec![];
    let mut estimated_costs_only_with_layer_info = if apply {
        None
    } else {
        Some(HashMap::new())
    };

    let mut finalize_tasks: Vec<DriveOperationFinalizeTask> = Vec::new();

    for drive_op in operations {
        if let Some(tasks) = drive_op.finalization_tasks(platform_version)? {
            finalize_tasks.extend(tasks);
        }
        low_level_operations.append(
            &mut drive_op.into_low_level_drive_operations(
                self, &mut estimated_costs_only_with_layer_info,
                block_info, transaction, platform_version,
            )?
        );
    }

    let mut cost_operations = vec![];
    self.apply_batch_low_level_drive_operations(
        estimated_costs_only_with_layer_info, transaction,
        low_level_operations, &mut cost_operations, &platform_version.drive,
    )?;

    for task in finalize_tasks {
        task.execute(self, platform_version);
    }

    Drive::calculate_fee(
        None, Some(cost_operations), &block_info.epoch,
        self.config.epochs_per_era, platform_version, previous_fee_versions,
    )
}
```

Let us break this down:

1. **Collect finalization tasks.** Some operations need post-processing. For example,
   `DataContractOperation` may produce a `RecordShieldedAnchor` finalization task that
   runs after the batch is committed. Tasks are collected first, executed last.

2. **Convert to low-level operations.** Each `DriveOperation` is expanded into one or
   more `LowLevelDriveOperation`s. A single document insertion might produce dozens of
   GroveDB operations (one for each index, plus the document itself, plus metadata).

3. **Apply the batch atomically.** `apply_batch_low_level_drive_operations` collects
   all `GroveOperation` items into a single GroveDB batch and applies them in one
   atomic write. This is critical -- if the node crashes mid-application, either all
   operations succeed or none do.

4. **Execute finalization tasks.** Post-commit callbacks run (e.g., updating caches).

5. **Calculate fees.** The cost of every operation (storage bytes written, bytes read,
   processing time) is tallied into a `FeeResult` that determines how many credits
   the user pays.

## The apply Parameter

Notice the `apply: bool` parameter. When `apply` is `false`, the entire pipeline runs
in **estimation mode**: operations are not actually written to GroveDB. Instead, the
`estimated_costs_only_with_layer_info` map is populated with what *would* be written,
and fees are estimated from that.

This is used during `check_tx` and fee estimation. The platform needs to know
approximately how much a transition will cost *before* actually applying it, both for
balance pre-checks and for returning fee estimates to clients.

## The Fee Calculation

At the end of `apply_drive_operations`, fees are calculated from the accumulated
cost operations:

```rust
Drive::calculate_fee(
    None,
    Some(cost_operations),
    &block_info.epoch,
    self.config.epochs_per_era,
    platform_version,
    previous_fee_versions,
)
```

The fee has two components:

- **Storage fee**: Proportional to the bytes written to disk. Stored bytes have an
  ongoing cost because they consume space in the state tree indefinitely (until deleted).
  When bytes are later removed, a portion of the storage fee is refunded.
- **Processing fee**: Proportional to the CPU work performed -- hashing, signature
  verification, tree traversal. This is ephemeral and not refundable.

The user's `user_fee_increase` (a percentage multiplier) applies to the processing fee,
allowing users to bid higher for priority.

## Finalization Tasks

Some `DriveOperation` variants carry finalization tasks -- callbacks that run after
the batch is committed. These are defined in
`packages/rs-drive/src/util/batch/drive_op_batch/finalize_task.rs`:

```rust
pub(crate) trait DriveOperationFinalizationTasks {
    fn finalization_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error>;
}
```

The most common finalization task is `RecordShieldedAnchor`, used by shielded
transaction operations to record Merkle tree anchors after the state changes are
committed. Finalization tasks are intentionally limited -- they must be deterministic
and must not fail, since they run after the batch is already committed.

## Putting It All Together: A Document Create

Let us trace a document creation through the entire three-tier pipeline:

1. **Action:** `BatchTransitionAction` containing a `DocumentAction::CreateAction` with
   the document data, its type, and the contract reference.

2. **DriveHighLevelOperationConverter:** The document create action produces a
   `DriveOperation::DocumentOperation(AddDocument { ... })` containing the owned
   document, contract info, document type info, and storage flags.

3. **DriveLowLevelOperationConverter:** The `AddDocument` operation produces multiple
   `LowLevelDriveOperation::GroveOperation` items:
   - Insert the serialized document at its primary key path
   - Insert index entries for each indexed property
   - Update the document type's document count
   - Record storage flags for fee tracking

4. **GroveDB batch:** All the `GroveOperation` items from all documents in the batch
   are collected into a single `GroveDbOpBatch` and applied atomically.

5. **Fee calculation:** The total bytes written, bytes read, and processing operations
   are summed to produce the `FeeResult`.

## Rules and Guidelines

**Do:**
- Implement `DriveHighLevelOperationConverter` for new action types. This is the
  contract between the validation layer and the storage layer.
- Keep `into_low_level_drive_operations` deterministic. Given the same inputs and
  state, it must always produce the same operations. Non-determinism causes consensus
  failures.
- Use the estimation mode (`apply = false`) for fee pre-checks. Do not skip fee
  estimation -- users need accurate cost information before committing to a transition.
- Test both the estimation path and the application path. They can diverge if the
  estimation layer info is stale or incomplete.

**Do not:**
- Access GroveDB directly from action conversion code. Always go through Drive's
  methods, which handle versioning, caching, and error translation.
- Produce side effects in `into_high_level_drive_operations`. This conversion must be
  pure -- it maps data, it does not read or write state.
- Assume a 1:1 mapping between actions and GroveDB operations. A single document
  create can produce 10+ GroveDB operations (one per index). A batch with 50 documents
  can produce hundreds.
- Forget finalization tasks when adding new operation types that require post-commit
  work. If your operation needs to update a cache or record an anchor, implement
  `DriveOperationFinalizationTasks`.
- Mix up `DriveOperation` (high-level, domain-aware) with `LowLevelDriveOperation`
  (low-level, GroveDB-aware). The naming can be confusing, but the distinction is
  important: the former knows about documents and identities, the latter knows about
  tree paths and elements.
