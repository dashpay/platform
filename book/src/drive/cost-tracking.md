# Cost Tracking

Dash Platform is a fee-based system. Every operation -- reading a document, inserting a key, hashing a value -- has a cost measured in platform credits. This chapter explains how Drive tracks those costs from individual operations all the way through to the final fee result.

## The Problem: Why Explicit Cost Tracking?

Some systems use gas metering: you start with a budget, and every opcode decrements it. Dash Platform takes a different approach. Instead of metering, it *accumulates* the costs of operations as they execute and then calculates the total fee at the end.

This matters because:

1. **Costs depend on what actually happened.** A GroveDB insert into a deep tree costs more than one into a shallow tree because of the Merkle proof updates involved. You cannot know this in advance -- you have to do the insert and measure.
2. **Storage fees and processing fees are different.** Storage fees are based on bytes added and removed. Processing fees cover computation: seeks, hash operations, byte loading. They need to be calculated separately.
3. **Refunds are possible.** When data is removed from storage, the original storage fee can be partially refunded. This requires knowing *when* the data was originally stored (which epoch), making the calculation depend on historical state.

## LowLevelDriveOperation: The Cost Carrier

The `LowLevelDriveOperation` enum is the vehicle that carries cost information through the system. Defined in `packages/rs-drive/src/fees/op.rs`:

```rust
pub enum LowLevelDriveOperation {
    GroveOperation(QualifiedGroveDbOp),
    FunctionOperation(FunctionOp),
    CalculatedCostOperation(OperationCost),
    PreCalculatedFeeResult(FeeResult),
}
```

Four variants, each representing a different kind of cost:

### GroveOperation

A raw GroveDB operation (insert, delete, get, etc.) that has not yet been executed. When a batch is built up (as described in the [Batch Operations](batch-operations.md) chapter), individual grove operations accumulate as this variant. They carry no cost yet -- the cost is determined when the batch is applied to GroveDB.

### CalculatedCostOperation

An `OperationCost` from a GroveDB operation that has already been executed (or estimated). This is what gets pushed onto the `drive_operations` vector by the grove operation wrappers:

```rust
fn push_drive_operation_result<T>(
    cost_context: CostContext<Result<T, GroveError>>,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
) -> Result<T, Error> {
    let CostContext { value, cost } = cost_context;
    if !cost.is_nothing() {
        drive_operations.push(CalculatedCostOperation(cost));
    }
    value.map_err(Error::from)
}
```

The `OperationCost` (from `grovedb_costs`) tracks:
- `seek_count`: Number of disk seeks performed
- `storage_cost`: Bytes added, replaced, and removed (with per-epoch tracking for refunds)
- `storage_loaded_bytes`: Bytes read from storage
- `hash_node_calls`: Number of hash operations for Merkle tree updates

### FunctionOperation

Represents the cost of a pure computation like hashing. Defined as:

```rust
pub struct FunctionOp {
    pub(crate) hash: HashFunction,
    pub(crate) rounds: u32,
}
```

With supported hash functions:

```rust
pub enum HashFunction {
    Sha256RipeMD160,
    Sha256,
    Sha256_2,  // Double SHA-256
    Blake3,
}
```

Each hash function has a base cost and a per-block cost. The total cost of a `FunctionOp` is:

```rust
impl FunctionOp {
    fn cost(&self, fee_version: &FeeVersion) -> Credits {
        let block_cost = (self.rounds as u64)
            .saturating_mul(self.hash.block_cost(fee_version));
        self.hash.base_cost(fee_version).saturating_add(block_cost)
    }
}
```

You can create a `FunctionOp` either by specifying the number of rounds directly or by providing the byte count (which calculates rounds based on the hash function's block size):

```rust
impl FunctionOp {
    pub fn new_with_round_count(hash: HashFunction, rounds: u32) -> Self {
        FunctionOp { hash, rounds }
    }

    pub fn new_with_byte_count(hash: HashFunction, byte_count: u16) -> Self {
        let blocks = byte_count / hash.block_size() + 1;
        let rounds = blocks + hash.rounds() - 1;
        FunctionOp { hash, rounds: rounds as u32 }
    }
}
```

### PreCalculatedFeeResult

A fee result that was already computed elsewhere and just needs to be included in the total. This is a pass-through -- no further calculation needed.

## BaseOp: Arithmetic Operation Costs

For simple computational operations (not storage-related), the `BaseOp` enum provides fixed costs:

```rust
pub enum BaseOp {
    Stop, Add, Mul, Sub, Div, Sdiv, Mod, Smod,
    Addmod, Mulmod, Signextend,
    Lt, Gt, Slt, Sgt, Eq, Iszero,
    And, Or, Xor, Not, Byte,
}

impl BaseOp {
    pub fn cost(&self) -> u64 {
        match self {
            BaseOp::Stop => 0,
            BaseOp::Add | BaseOp::Sub => 12,
            BaseOp::Mul | BaseOp::Div | BaseOp::Sdiv |
            BaseOp::Mod | BaseOp::Smod | BaseOp::Signextend => 20,
            BaseOp::Addmod | BaseOp::Mulmod => 32,
            BaseOp::Lt | BaseOp::Gt | BaseOp::Slt |
            BaseOp::Sgt | BaseOp::Eq | BaseOp::Iszero |
            BaseOp::And | BaseOp::Or | BaseOp::Xor |
            BaseOp::Not | BaseOp::Byte => 12,
        }
    }
}
```

These are EVM-inspired operation costs, adapted for the platform's fee model. Comparisons and bitwise operations cost 12 credits. Multiplication and division cost 20. Modular arithmetic costs 32.

## The consume_to_fees_v0 Pipeline

When all operations are collected, they are converted into fee results through `consume_to_fees_v0`:

```rust
pub fn consume_to_fees_v0(
    drive_operations: Vec<LowLevelDriveOperation>,
    epoch: &Epoch,
    epochs_per_era: u16,
    fee_version: &FeeVersion,
    previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
) -> Result<Vec<FeeResult>, Error> {
    drive_operations.into_iter().map(|operation| match operation {
        PreCalculatedFeeResult(f) => Ok(f),

        FunctionOperation(op) => Ok(FeeResult {
            processing_fee: op.cost(fee_version),
            ..Default::default()
        }),

        _ => {
            let cost = operation.operation_cost()?;

            // Storage fee: bytes added * rate per byte
            let storage_fee = cost.storage_cost.added_bytes as u64
                * fee_version.storage.storage_disk_usage_credit_per_byte;

            // Processing fee: seeks + loaded bytes + hash calls + ...
            let processing_fee = cost.ephemeral_cost(fee_version)?;

            // Refunds from removed data
            let (fee_refunds, removed_bytes_from_system) =
                match cost.storage_cost.removed_bytes {
                    NoStorageRemoval => (FeeRefunds::default(), 0),
                    BasicStorageRemoval(amount) => (FeeRefunds::default(), amount),
                    SectionedStorageRemoval(removal_per_epoch_by_identifier) => {
                        // Calculate epoch-aware refunds
                        (FeeRefunds::from_storage_removal(
                            removal_per_epoch_by_identifier,
                            epoch.index,
                            epochs_per_era,
                            previous_fee_versions,
                        )?, system_amount)
                    }
                };

            Ok(FeeResult {
                storage_fee,
                processing_fee,
                fee_refunds,
                removed_bytes_from_system,
            })
        }
    }).collect()
}
```

Each operation produces a `FeeResult` with four components:

- **`storage_fee`**: The cost of new bytes written to persistent storage.
- **`processing_fee`**: The ephemeral cost of computation and I/O.
- **`fee_refunds`**: Credits returned because previously-stored data was removed.
- **`removed_bytes_from_system`**: Bytes removed that were stored by the system (not any particular identity), so no refund is issued.

## Ephemeral Cost Calculation

The `ephemeral_cost` method on `OperationCost` computes the processing fee from the raw operation metrics:

```rust
impl DriveCost for OperationCost {
    fn ephemeral_cost(&self, fee_version: &FeeVersion) -> Result<Credits, Error> {
        let OperationCost {
            seek_count,
            storage_cost,
            storage_loaded_bytes,
            hash_node_calls,
        } = self;

        let seek_cost = (*seek_count as u64)
            .checked_mul(fee_version.storage.storage_seek_cost)?;

        let storage_added_bytes_ephemeral_cost = (storage_cost.added_bytes as u64)
            .checked_mul(fee_version.storage.storage_processing_credit_per_byte)?;

        let storage_replaced_bytes_ephemeral_cost = (storage_cost.replaced_bytes as u64)
            .checked_mul(fee_version.storage.storage_processing_credit_per_byte)?;

        let storage_loaded_bytes_cost = (*storage_loaded_bytes)
            .checked_mul(fee_version.storage.storage_load_credit_per_byte)?;

        let blake3_total = fee_version.hashing.blake3_base
            + fee_version.hashing.blake3_per_block;
        let hash_node_cost = blake3_total * (*hash_node_calls as u64);

        // Sum all costs with overflow checking
        seek_cost
            .checked_add(storage_added_bytes_ephemeral_cost)
            .and_then(|c| c.checked_add(storage_replaced_bytes_ephemeral_cost))
            .and_then(|c| c.checked_add(storage_loaded_bytes_cost))
            .and_then(|c| c.checked_add(hash_node_cost))
            .ok_or_else(|| get_overflow_error("ephemeral cost addition overflow"))
    }
}
```

Notice that every multiplication and addition uses checked arithmetic. In a fee system, overflows would be catastrophic -- an underflowed fee could let someone store unlimited data for free.

## Helper Methods on LowLevelDriveOperation

The `LowLevelDriveOperation` type provides several methods for working with collections of operations:

```rust
impl LowLevelDriveOperation {
    // Combine all CalculatedCostOperation costs into one
    pub fn combine_cost_operations(
        operations: &[LowLevelDriveOperation]
    ) -> OperationCost { ... }

    // Extract GroveOperation variants into a batch
    pub fn grovedb_operations_batch(
        operations: &[LowLevelDriveOperation]
    ) -> GroveDbOpBatch { ... }

    // Same but consuming the vector
    pub fn grovedb_operations_batch_consume(
        operations: Vec<LowLevelDriveOperation>
    ) -> GroveDbOpBatch { ... }

    // Partition: grove ops go to batch, rest stays as leftovers
    pub fn grovedb_operations_batch_consume_with_leftovers(
        operations: Vec<LowLevelDriveOperation>,
    ) -> (GroveDbOpBatch, Vec<LowLevelDriveOperation>) { ... }
}
```

The `grovedb_operations_batch_consume_with_leftovers` method is particularly important -- it is used during batch application to separate the grove operations (which go to GroveDB) from the cost operations (which go to fee calculation).

## Constructing Operations

`LowLevelDriveOperation` also has constructors for common operations:

```rust
impl LowLevelDriveOperation {
    pub fn for_known_path_key_empty_tree(
        path: Vec<Vec<u8>>, key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self { ... }

    pub fn for_known_path_key_empty_sum_tree(
        path: Vec<Vec<u8>>, key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self { ... }

    pub fn insert_for_known_path_key_element(
        path: Vec<Vec<u8>>, key: Vec<u8>, element: Element,
    ) -> Self {
        GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(
            path, key, element
        ))
    }

    pub fn replace_for_known_path_key_element(
        path: Vec<Vec<u8>>, key: Vec<u8>, element: Element,
    ) -> Self {
        GroveOperation(QualifiedGroveDbOp::replace_op(
            path, key, element
        ))
    }
}
```

These provide a cleaner API than constructing `QualifiedGroveDbOp` directly, and they handle storage flags properly.

## Rules and Guidelines

**Do:**
- Use checked arithmetic everywhere in fee calculations. `checked_mul`, `checked_add`, and friends.
- Construct `FunctionOp` with `new_with_byte_count` when you know the input size, and `new_with_round_count` when you know the rounds.
- Let operations accumulate in the `drive_operations` vector throughout the call chain.

**Do not:**
- Call `operation_cost()` on a `GroveOperation` -- it will return an error. Grove operations must be executed first; only `CalculatedCostOperation` carries a usable cost.
- Forget that storage fees and processing fees are calculated differently. Storage fees are proportional to bytes. Processing fees are a complex function of seeks, loads, hashes, and byte movements.
- Assume fee rates are constant. They are versioned through `FeeVersion` and can change between protocol versions.
- Ignore `removed_bytes_from_system`. This tracks bytes removed that belong to the system rather than a specific identity, affecting the refund calculation.
