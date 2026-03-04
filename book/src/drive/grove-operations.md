# Grove Operations

Drive is the storage layer of Dash Platform, and GroveDB is the authenticated data structure (a Merkle tree of trees) that Drive uses under the hood. But Drive never talks to GroveDB directly in its business logic. Instead, every single GroveDB call is wrapped in a versioned `Drive` method that follows a consistent pattern. This chapter explains why that wrapper layer exists and how it works.

## The Problem: Raw GroveDB Is Too Low-Level

If you were to call GroveDB directly throughout Drive's codebase, you would face several problems:

1. **No cost tracking.** GroveDB operations return a `CostContext` that wraps both the result and an `OperationCost`. If you forget to capture that cost, the fee system breaks.
2. **No version dispatch.** Different protocol versions might need different behavior for the same logical operation (like how to handle estimated costs vs. actual costs).
3. **No consistent API.** Each caller would need to handle cost capture, error conversion, and version checking independently.

The grove operations layer solves all three problems by providing a single, consistent abstraction. Every grove operation is a method on `Drive` that takes a path, a key, some type information, a transaction, and the mutable `drive_operations` accumulator.

## The Module Structure

The grove operations live in `packages/rs-drive/src/util/grove_operations/`. Each operation is its own submodule:

```
grove_operations/
    mod.rs                    -- shared types and helpers
    grove_insert/
        mod.rs                -- version dispatcher
        v0/mod.rs             -- v0 implementation
    grove_get_raw/
        mod.rs                -- version dispatcher
        v0/mod.rs             -- v0 implementation
    grove_delete/
    grove_get/
    grove_get_raw_optional/
    grove_has_raw/
    batch_insert/
    batch_insert_empty_tree/
    batch_delete/
    ... (30+ more)
```

Each submodule follows the same structure: a `mod.rs` that dispatches on the version, and a `v0/mod.rs` (and potentially `v1/`, `v2/`, etc.) with the actual implementation.

## The drive_operations Accumulator Pattern

This is the most important pattern to understand. Almost every grove operation method accepts a mutable reference to a `Vec<LowLevelDriveOperation>`:

```rust
drive_operations: &mut Vec<LowLevelDriveOperation>
```

Instead of returning costs directly, the method *pushes* the cost of its GroveDB call onto this vector. The caller passes the same vector through multiple operations, accumulating all costs. Later, the batch application system processes this vector to calculate the total fee.

Why accumulate rather than execute immediately? Two reasons:

1. **Fee estimation.** When `apply` is false, Drive needs to estimate costs without actually writing to the database. The operations still accumulate cost information, but no state changes occur.
2. **Atomic batching.** Multiple operations can be collected and then applied as a single atomic batch. More on this in the [Batch Operations](batch-operations.md) chapter.

## A Concrete Example: grove_get_raw

Let us trace through a complete grove operation. The version dispatcher is in `packages/rs-drive/src/util/grove_operations/grove_get_raw/mod.rs`:

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
                path, key, direct_query_type,
                transaction, drive_operations, drive_version,
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

The dispatcher consults `drive_version.grove_methods.basic.grove_get_raw` to determine which implementation version to call. If the version is unknown, it returns an error immediately.

Now the v0 implementation, from `grove_get_raw/v0/mod.rs`:

```rust
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
        match direct_query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type,
                query_target,
            } => {
                let key_info_path = KeyInfoPath::from_known_owned_path(path.to_vec());
                let key_info = KeyInfo::KnownKey(key.to_vec());
                let cost = match query_target {
                    QueryTarget::QueryTargetTree(flags_size, tree_type) => {
                        GroveDb::average_case_for_get_tree(
                            &key_info_path, &key_info, flags_size,
                            tree_type, in_tree_type,
                            &drive_version.grove_version,
                        )
                    }
                    QueryTarget::QueryTargetValue(estimated_value_size) => {
                        GroveDb::average_case_for_get_raw(
                            &key_info_path, &key_info,
                            estimated_value_size, in_tree_type,
                            &drive_version.grove_version,
                        )
                    }
                }?;
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(None) // No actual data -- just cost estimation
            }
            DirectQueryType::StatefulDirectQuery => {
                let CostContext { value, cost } = self.grove.get_raw(
                    path, key, transaction,
                    &drive_version.grove_version,
                );
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(Some(value.map_err(Error::from)?))
            }
        }
    }
}
```

This reveals the dual nature of every grove operation: it can operate in **stateless** mode (for cost estimation) or **stateful** mode (for actual execution). In stateless mode, it calculates the average-case cost without touching the database and returns `None`. In stateful mode, it performs the actual GroveDB read and returns the element.

## The DirectQueryType Enum

The `DirectQueryType` enum, defined in `packages/rs-drive/src/util/grove_operations/mod.rs`, controls this dual behavior:

```rust
pub enum DirectQueryType {
    StatelessDirectQuery {
        in_tree_type: TreeType,
        query_target: QueryTarget,
    },
    StatefulDirectQuery,
}
```

- **`StatelessDirectQuery`**: Used for fee estimation. Provides the tree type and query target so the system can calculate costs without reading from disk. The `QueryTarget` specifies whether we are querying for a tree (with flags) or a value (with an estimated size).

- **`StatefulDirectQuery`**: Used for actual execution. The system reads from GroveDB and returns real data.

There is also a more general `QueryType` enum that adds reference size estimation:

```rust
pub enum QueryType {
    StatelessQuery {
        in_tree_type: TreeType,
        query_target: QueryTarget,
        estimated_reference_sizes: Vec<u32>,
    },
    StatefulQuery,
}
```

And a `QueryTarget` enum that specifies what kind of element we expect to find:

```rust
pub enum QueryTarget {
    QueryTargetTree(FlagsLen, TreeType),
    QueryTargetValue(u32),  // estimated value size in bytes
}
```

## Another Example: grove_insert

Inserts follow the same pattern. From `packages/rs-drive/src/util/grove_operations/grove_insert/v0/mod.rs`:

```rust
impl Drive {
    pub(super) fn grove_insert_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        element: Element,
        transaction: TransactionArg,
        options: Option<InsertOptions>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let cost_context = self.grove.insert(
            path, key, element, options, transaction,
            &drive_version.grove_version,
        );
        push_drive_operation_result(cost_context, drive_operations)
    }
}
```

This is simpler than the get because inserts are always stateful -- you cannot "estimate" an insert by not doing it. The `push_drive_operation_result` helper extracts the cost from GroveDB's `CostContext` and pushes it onto the operations vector:

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

Notice the `is_nothing()` check -- if an operation has zero cost (which can happen), we skip pushing to avoid cluttering the vector.

## Batch Apply Types

For operations that work in batch mode (building up a batch of operations to apply atomically), there are corresponding apply-type enums. For example:

```rust
pub enum BatchDeleteApplyType {
    StatelessBatchDelete {
        in_tree_type: TreeType,
        estimated_key_size: u32,
        estimated_value_size: u32,
    },
    StatefulBatchDelete {
        is_known_to_be_subtree_with_sum: Option<MaybeTree>,
    },
}

pub enum BatchInsertTreeApplyType {
    StatelessBatchInsertTree {
        in_tree_type: TreeType,
        tree_type: TreeType,
        flags_len: FlagsLen,
    },
    StatefulBatchInsertTree,
}
```

These follow the same stateless/stateful split. The stateless variants carry enough information to estimate costs without touching the database, while the stateful variants trigger actual operations. Each batch apply type can be converted to a `DirectQueryType` for use with the lower-level grove operations.

## The GroveDBToUse Enum

A recent addition supports querying different GroveDB instances:

```rust
pub enum GroveDBToUse {
    Current,
    LatestCheckpoint,
    Checkpoint(u64),
}
```

This enables queries against historical checkpoints -- useful for proof generation and state verification at specific block heights.

## Method Signature Conventions

Across all grove operations, you will notice a consistent parameter ordering:

```
&self, path, key, [element], [query_type], transaction, drive_operations, drive_version
```

1. `&self` -- the Drive instance (which holds the GroveDB handle)
2. Path -- where in the tree
3. Key -- which element at that path
4. Element -- the data to write (for inserts/replaces)
5. Query type -- stateless vs. stateful
6. Transaction -- the GroveDB transaction context
7. `drive_operations` -- the mutable cost accumulator
8. `drive_version` -- for version dispatching

This consistency makes the codebase navigable even though there are 30+ different grove operations.

## Rules and Guidelines

**Do:**
- Always use the grove operation wrappers on `Drive`. Never call `self.grove.insert()` or `self.grove.get()` directly in business logic.
- Pass the `drive_operations` vector through every call chain. It is how costs propagate upward.
- Use `StatelessDirectQuery` for fee estimation and `StatefulDirectQuery` for actual execution.

**Do not:**
- Ignore the cost returned by GroveDB operations. The `push_drive_operation_result` helper exists for this reason.
- Mix stateful and stateless queries in a single estimation pass. Pick one mode and stick with it.
- Create new grove operations without following the `mod.rs` + `v0/mod.rs` dispatcher pattern. Consistency is critical.
