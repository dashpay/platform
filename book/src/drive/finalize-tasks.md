# Finalize Tasks

Most operations on Dash Platform follow a straightforward path: convert high-level operations to low-level ones, apply them atomically, calculate fees. But some operations need something to happen *after* the batch has been successfully committed. That is what finalize tasks are for.

## The Problem: Post-Commit Side Effects

Consider what happens when a data contract is updated. The updated contract is written to GroveDB as part of the atomic batch. But Drive also caches contracts in memory for fast access. After the batch commits, that cache entry is stale -- it still holds the old version of the contract.

You cannot invalidate the cache *before* the commit, because the commit might fail (GroveDB could reject the batch due to a consistency error). And you cannot invalidate it *during* the commit, because the batch application is a single atomic operation on GroveDB. You need a post-commit callback: "if the batch succeeds, do this."

That is exactly what `DriveOperationFinalizeTask` provides.

## The DriveOperationFinalizeTask Enum

Defined in `packages/rs-drive/src/util/batch/drive_op_batch/finalize_task.rs`:

```rust
pub enum DriveOperationFinalizeTask {
    RemoveDataContractFromCache { contract_id: Identifier },
}
```

Currently there is only one variant: `RemoveDataContractFromCache`. When a data contract is updated, this task is registered. After the batch commits successfully, it removes the stale contract from Drive's in-memory cache, forcing the next access to reload from GroveDB.

The execution is straightforward:

```rust
impl DriveOperationFinalizeTask {
    pub fn execute(self, drive: &Drive, _platform_version: &PlatformVersion) {
        match self {
            DriveOperationFinalizeTask::RemoveDataContractFromCache { contract_id } => {
                drive.cache.data_contracts.remove(contract_id.to_buffer());
            }
        }
    }
}
```

## The DriveOperationFinalizationTasks Trait

Not every `DriveOperation` has finalize tasks. The trait that declares them is:

```rust
pub trait DriveOperationFinalizationTasks {
    fn finalization_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error>;
}
```

The return type is `Option<Vec<...>>` rather than just `Vec<...>`. This is a deliberate optimization -- since only one operation type currently has finalize tasks, returning `None` (rather than an empty `Vec`) avoids unnecessary heap allocations for the vast majority of operations.

The implementation on `DriveOperation` dispatches through versioning:

```rust
impl DriveOperationFinalizationTasks for DriveOperation<'_> {
    fn finalization_tasks(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .operations
            .finalization_tasks
        {
            0 => self.finalization_tasks_v0(platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveOperation.finalization_tasks".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
```

And the v0 implementation only checks data contract operations:

```rust
impl DriveOperation<'_> {
    fn finalization_tasks_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Vec<DriveOperationFinalizeTask>>, Error> {
        match self {
            DriveOperation::DataContractOperation(o) =>
                o.finalization_tasks(platform_version),
            _ => Ok(None),
        }
    }
}
```

Every other operation variant -- documents, identities, tokens, withdrawals -- returns `None`. Only data contract operations can produce finalize tasks.

## How Finalize Tasks Integrate with Batch Application

The integration point is in `apply_drive_operations_v0`, which we saw in the [Batch Operations](batch-operations.md) chapter. Here is the relevant excerpt:

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
    // ...

    let mut finalize_tasks: Vec<DriveOperationFinalizeTask> = Vec::new();

    for drive_op in operations {
        // Step 1: Collect finalize tasks BEFORE converting the operation
        if let Some(tasks) = drive_op.finalization_tasks(platform_version)? {
            finalize_tasks.extend(tasks);
        }

        // Step 2: Convert to low-level operations (consumes drive_op)
        low_level_operations.append(
            &mut drive_op.into_low_level_drive_operations(/* ... */)?
        );
    }

    // Step 3: Apply the batch atomically
    self.apply_batch_low_level_drive_operations(/* ... */)?;

    // Step 4: Execute finalize tasks AFTER successful commit
    for task in finalize_tasks {
        task.execute(self, platform_version);
    }

    // Step 5: Calculate fees
    Drive::calculate_fee(/* ... */)
}
```

The ordering is critical:

1. **Collect finalize tasks first.** This happens before `into_low_level_drive_operations` because that method *consumes* the `DriveOperation` (it takes `self`, not `&self`). After conversion, the original operation is gone.

2. **Apply the batch.** If this fails, we return the error immediately. The finalize tasks never execute.

3. **Execute finalize tasks only on success.** By the time we reach step 4, we know the batch committed successfully. Now it is safe to invalidate caches and perform other side effects.

## The Cache Invalidation Pattern

Why cache invalidation specifically? Drive maintains an in-memory cache of frequently-accessed data contracts:

```rust
drive.cache.data_contracts.remove(contract_id.to_buffer());
```

Without this invalidation, here is what would go wrong:

1. Block N: Contract "foo" is at version 3 in GroveDB and cached.
2. Block N+1: A state transition updates "foo" to version 4 in GroveDB.
3. Block N+1: Without cache invalidation, queries still return version 3 from the cache.
4. Block N+1: Document validation uses the stale version 3 schema, potentially accepting invalid documents.

By removing the contract from the cache after a successful update, the next access will read version 4 from GroveDB and re-populate the cache.

## When to Use Finalize Tasks

Finalize tasks are the right tool when you need to perform side effects that:

1. **Must not happen if the batch fails.** If you invalidate a cache before the commit and the commit fails, you have a warm-up penalty for no reason (and potentially incorrect behavior during the recovery window).

2. **Are not idempotent with respect to partial application.** Cache invalidation is fine to do after commit because the cache will self-heal on the next access. But if your side effect were "send a network message," you would want to be very sure the batch actually committed.

3. **Operate on data outside GroveDB.** GroveDB's atomic batch guarantees only cover GroveDB state. In-memory caches, external systems, and non-transactional state all need explicit post-commit handling.

## Extending Finalize Tasks

To add a new finalize task:

1. Add a variant to the `DriveOperationFinalizeTask` enum in `finalize_task.rs`.
2. Implement its execution in the `execute` method's match block.
3. In the relevant `DriveOperation` variant's `finalization_tasks` implementation, return the new task when appropriate.

The design is intentionally simple and extensible. The enum + trait pattern means new finalize tasks do not affect existing code paths.

## Rules and Guidelines

**Do:**
- Collect finalize tasks before consuming `DriveOperation` via `into_low_level_drive_operations`.
- Execute finalize tasks only after confirming the batch committed successfully.
- Keep finalize task execution fast. They run synchronously in the block processing pipeline.

**Do not:**
- Put business logic in finalize tasks. They are for side effects like cache management, not for state mutations. State mutations belong in the batch itself.
- Execute finalize tasks if the batch application returns an error. The whole point is that they only run on success.
- Rely on finalize tasks for correctness-critical behavior that must be exactly-once. If the process crashes between batch commit and finalize task execution, the finalize tasks will not run. They should always be "nice to have" optimizations (like cache invalidation) rather than required for correctness.
- Introduce finalize tasks with external side effects (like network calls) without careful consideration of failure modes. Keep them fast, local, and idempotent.
