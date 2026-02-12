# Batch Operations

Individual grove operations are the atoms. Batch operations are the molecules. Dash Platform never applies a single database write in isolation -- every state transition (a document creation, a contract update, a balance transfer) results in a *batch* of operations that are applied atomically. Either they all succeed, or none of them do. This chapter covers how that batching works at every level of the stack.

## The Three Levels of Abstraction

Drive has three layers of operation abstraction, each serving a different purpose:

1. **`DriveOperation`** -- High-level, domain-aware operations like "add this document" or "apply this contract."
2. **`LowLevelDriveOperation`** -- Individual grove operations, cost calculations, and function costs.
3. **`GroveDbOpBatch`** -- The final flat list of `QualifiedGroveDbOp` items that GroveDB applies atomically.

The flow is always top-down: `DriveOperation` -> `LowLevelDriveOperation` -> `GroveDbOpBatch`.

## DriveOperation: The High-Level API

The `DriveOperation` enum lives in `packages/rs-drive/src/util/batch/drive_op_batch/mod.rs` and represents every kind of operation the platform can perform:

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

Each variant wraps a domain-specific operation type. For example, a `DocumentOperationType` might be an `AddDocument`, `UpdateDocument`, or `DeleteDocument`. A `DataContractOperationType` might be `ApplyContract`. And so on.

The last two variants -- `GroveDBOperation` and `GroveDBOpBatch` -- are escape hatches for when code already has raw GroveDB operations and just wants to include them in the batch.

## The DriveLowLevelOperationConverter Trait

Every `DriveOperation` variant knows how to convert itself into a list of low-level operations. This is defined by the `DriveLowLevelOperationConverter` trait:

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

The `estimated_costs_only_with_layer_info` parameter is key. When it is `None`, the converter performs actual operations (stateful mode). When it is `Some(HashMap)`, the converter only estimates costs and fills in layer information for GroveDB's cost estimation (stateless mode).

The `DriveOperation` enum implements this trait by dispatching to each variant:

```rust
impl DriveLowLevelOperationConverter for DriveOperation<'_> {
    fn into_low_level_drive_operations(
        self, drive: &Drive,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>
        >,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match self {
            DriveOperation::DataContractOperation(op) =>
                op.into_low_level_drive_operations(
                    drive, estimated_costs_only_with_layer_info,
                    block_info, transaction, platform_version,
                ),
            DriveOperation::DocumentOperation(op) =>
                op.into_low_level_drive_operations(
                    drive, estimated_costs_only_with_layer_info,
                    block_info, transaction, platform_version,
                ),
            // ... each variant delegates to its own converter
            DriveOperation::GroveDBOperation(op) =>
                Ok(vec![GroveOperation(op)]),
            DriveOperation::GroveDBOpBatch(operations) =>
                Ok(operations.operations.into_iter()
                    .map(GroveOperation).collect()),
        }
    }
}
```

## The apply_drive_operations Flow

The centerpiece of the batch system is `Drive::apply_drive_operations`. From `packages/rs-drive/src/util/batch/drive_op_batch/drive_methods/apply_drive_operations/v0/mod.rs`:

```rust
impl Drive {
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
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let mut finalize_tasks: Vec<DriveOperationFinalizeTask> = Vec::new();

        for drive_op in operations {
            // Collect finalize tasks before conversion
            if let Some(tasks) = drive_op.finalization_tasks(platform_version)? {
                finalize_tasks.extend(tasks);
            }

            // Convert high-level to low-level operations
            low_level_operations.append(
                &mut drive_op.into_low_level_drive_operations(
                    self,
                    &mut estimated_costs_only_with_layer_info,
                    block_info,
                    transaction,
                    platform_version,
                )?
            );
        }

        let mut cost_operations = vec![];

        // Apply the batch atomically
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            low_level_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;

        // Execute post-commit finalize tasks
        for task in finalize_tasks {
            task.execute(self, platform_version);
        }

        // Calculate total fee from accumulated costs
        Drive::calculate_fee(
            None,
            Some(cost_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )
    }
}
```

Let us trace the flow step by step:

1. **Mode selection.** If `apply` is true, `estimated_costs_only_with_layer_info` is `None`, triggering stateful execution. If false, it is `Some(HashMap)`, triggering cost estimation only.

2. **Finalize task collection.** Before converting each operation, we collect any finalize tasks it declares. These are post-commit callbacks (covered in the [Finalize Tasks](finalize-tasks.md) chapter).

3. **Conversion.** Each `DriveOperation` is converted into zero or more `LowLevelDriveOperation` items. A single high-level operation like "add document" might produce dozens of low-level operations (insert the document itself, update each index, update the contract's document count, etc.).

4. **Batch application.** All low-level operations are applied as a single atomic batch through `apply_batch_low_level_drive_operations`.

5. **Finalization.** Post-commit tasks execute (like invalidating caches).

6. **Fee calculation.** The accumulated cost operations are converted into a `FeeResult`.

## GroveDbOpBatch: The Final Layer

Before operations hit GroveDB, they are split into two categories. From `packages/rs-drive/src/util/operations/apply_batch_low_level_drive_operations/v0/mod.rs`:

```rust
impl Drive {
    pub(crate) fn apply_batch_low_level_drive_operations_v0(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: Vec<LowLevelDriveOperation>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let (grove_db_operations, mut other_operations) =
            LowLevelDriveOperation::grovedb_operations_batch_consume_with_leftovers(
                batch_operations,
            );
        if !grove_db_operations.is_empty() {
            self.apply_batch_grovedb_operations(
                estimated_costs_only_with_layer_info,
                transaction,
                grove_db_operations,
                drive_operations,
                drive_version,
            )?;
        }
        drive_operations.append(&mut other_operations);
        Ok(())
    }
}
```

The `grovedb_operations_batch_consume_with_leftovers` method partitions the operations:
- `GroveOperation` variants become a `GroveDbOpBatch` that is applied atomically to GroveDB.
- Everything else (`CalculatedCostOperation`, `FunctionOperation`, `PreCalculatedFeeResult`) is kept as-is for fee calculation.

The `GroveDbOpBatch` itself is defined in `packages/rs-drive/src/util/batch/grovedb_op_batch/mod.rs`:

```rust
pub struct GroveDbOpBatch {
    pub(crate) operations: Vec<QualifiedGroveDbOp>,
}
```

It is a thin wrapper around a vector of `QualifiedGroveDbOp` -- GroveDB's native batch operation type. The wrapper provides convenience methods for building batches:

```rust
pub trait GroveDbOpBatchV0Methods {
    fn new() -> Self;
    fn push(&mut self, op: QualifiedGroveDbOp);
    fn add_insert_empty_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>);
    fn add_insert_empty_sum_tree(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>);
    fn add_delete(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>);
    fn add_insert(&mut self, path: Vec<Vec<u8>>, key: Vec<u8>, element: Element);
    fn verify_consistency_of_operations(&self) -> GroveDbOpConsistencyResults;
    fn contains<'c, P>(&self, path: P, key: &[u8]) -> Option<&GroveOp>;
    fn remove<'c, P>(&mut self, path: P, key: &[u8]) -> Option<GroveOp>;
    fn remove_if_insert(&mut self, path: Vec<Vec<u8>>, key: &[u8]) -> Option<GroveOp>;
}
```

The `verify_consistency_of_operations` method is particularly important -- it checks that the batch does not contain conflicting operations (like inserting and deleting the same key).

## Building a Batch: A Real Example

The test code in `drive_op_batch/mod.rs` shows how a typical batch is assembled:

```rust
let mut drive_operations = vec![];

// Step 1: Apply a contract
drive_operations.push(DataContractOperation(ApplyContract {
    contract: Cow::Borrowed(&contract),
    storage_flags: None,
}));

// Step 2: Add a document
drive_operations.push(DocumentOperation(AddDocument {
    owned_document_info: OwnedDocumentInfo {
        document_info: DocumentRefInfo((
            &document,
            StorageFlags::optional_default_as_cow(),
        )),
        owner_id: None,
    },
    contract_info: DataContractInfo::BorrowedDataContract(&contract),
    document_type_info: DocumentTypeInfo::DocumentTypeRef(document_type),
    override_document: false,
}));

// Step 3: Apply everything atomically
drive.apply_drive_operations(
    drive_operations,
    true,  // actually apply, not just estimate
    &BlockInfo::default(),
    Some(&db_transaction),
    platform_version,
    None,
)?;
```

The contract application and document insertion happen in the same atomic batch. If the document insert fails (perhaps due to a uniqueness constraint violation), the contract application is also rolled back. This all-or-nothing guarantee is fundamental to platform correctness.

## The Display Implementation

The `GroveDbOpBatch` has a custom `Display` implementation that produces human-readable output, mapping raw byte paths to meaningful names:

```rust
impl fmt::Display for GroveDbOpBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for op in &self.operations {
            let (path_string, known_path) = readable_path(&op.path);
            let (key_string, _) = readable_key_info(known_path, &op.key);
            writeln!(f, "   Path: {}", path_string)?;
            writeln!(f, "   Key: {}", key_string)?;
            // ... operation details
        }
    }
}
```

This translates paths like `[0x03]` into `Identities(3)` and keys like 32-byte arrays into `IdentityId(bs58::...)`. Invaluable for debugging.

## Rules and Guidelines

**Do:**
- Always use `apply_drive_operations` for applying batches. It handles the full pipeline: conversion, application, finalization, and fee calculation.
- Collect all operations for a state transition into a single `Vec<DriveOperation>` before applying.
- Use the `apply: false` flag for dry-run fee estimation before committing.

**Do not:**
- Apply operations one at a time. Always batch them for atomicity.
- Mix stateful and stateless operations in the same batch application pass.
- Forget to handle the `FeeResult` returned by `apply_drive_operations`. The fee system depends on it.
- Manually construct `GroveDbOpBatch` objects unless you are working at the lowest level. Prefer `DriveOperation` for business logic.
