# Finalize Tasks

Most operations on Dash Platform follow a straightforward path: convert high-level operations to low-level ones, apply them atomically, calculate fees. But some operations need something to happen *after* the batch has been successfully committed. That is what finalize tasks are for.

## The Problem: Post-Apply Side Effects

Consider what happens when a data contract is updated. The updated contract is written to GroveDB as part of the atomic batch. But Drive also caches contracts in memory for fast access. After the batch is applied, that cache entry is stale -- it still holds the old version of the contract.

You cannot refresh the cache *before* the batch is applied, because applying might fail (GroveDB could reject the batch due to a consistency error). And you cannot refresh it *during* the apply, because the batch application is a single atomic operation on GroveDB. You need a post-apply callback: "if the batch succeeds, do this."

That is exactly what `DriveOperationFinalizeTask` provides.

## The DriveOperationFinalizeTask Enum

Defined in `packages/rs-drive/src/util/batch/drive_op_batch/finalize_task.rs`:

```rust
pub enum DriveOperationFinalizeTask {
    RefreshDataContractCache { contract_id: Identifier },
}
```

Currently there is only one variant: `RefreshDataContractCache`. When a data contract is created or updated, this task is registered. After the batch is applied successfully, it re-seeds Drive's in-memory cache from what state now holds for the contract, reading through the transaction the batch was applied in.

The execution delegates to `Drive::refresh_data_contract_cache_from_state`:

```rust
impl DriveOperationFinalizeTask {
    pub fn execute(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match self {
            DriveOperationFinalizeTask::RefreshDataContractCache { contract_id } => drive
                .refresh_data_contract_cache_from_state(
                    contract_id.to_buffer(),
                    transaction,
                    platform_version,
                ),
        }
    }
}
```

The task takes the transaction because what it seeds must be what *this transaction* holds, not committed state. Inside a block, the batch was applied in the block transaction and the update is not committed yet: the refresh reads the updated contract through that transaction, marks the contract as modified in the block, and seeds the block cache with it. Outside a block (a caller that passed no transaction, so the batch committed on its own) the refresh only evicts the superseded copy, and the next reader reloads it from committed state.

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

    // Step 4: Execute finalize tasks AFTER a successful apply, and only when
    // the batch was applied rather than estimated. They read through the
    // caller's transaction; `caller_transaction` is `None` exactly when the
    // batch committed on its own just above.
    if apply {
        for task in finalize_tasks {
            task.execute(self, caller_transaction, platform_version)?;
        }
    }

    // Step 5: Calculate fees
    Drive::calculate_fee(/* ... */)
}
```

The ordering is critical:

1. **Collect finalize tasks first.** This happens before `into_low_level_drive_operations` because that method *consumes* the `DriveOperation` (it takes `self`, not `&self`). After conversion, the original operation is gone.

2. **Apply the batch.** If this fails, we return the error immediately. The finalize tasks never execute.

3. **Execute finalize tasks only on success.** By the time we reach step 4, we know the batch was applied successfully. Now it is safe to refresh caches and perform other side effects. An estimation-only call (`apply == false`) writes nothing, so it runs no finalize tasks either.

## The Cache Refresh Pattern

Why a refresh rather than a plain eviction? Drive maintains an in-memory cache of frequently-accessed data contracts, and two kinds of reader share it: block execution, which reads through the block transaction on the consensus thread, and the query threads, which read committed state with no transaction, concurrently, and populate the global half of the cache with what they read.

Without any refresh, here is what would go wrong:

1. Block N: Contract "foo" is at version 3 in GroveDB and cached.
2. Block N+1: A state transition updates "foo" to version 4 in GroveDB.
3. Block N+1: Without a refresh, the block still reads version 3 from the cache.
4. Block N+1: Document validation uses the stale version 3 schema, potentially accepting invalid documents.

A plain eviction is not enough, because of the query threads. Between the eviction and the block's commit, a query can read committed state (still version 3) and put version 3 back into the global cache. A transactional read that missed the block cache and fell back to the global cache would then be handed version 3 again, while a validator whose cache happened to be cold reads version 4 from the transaction: the two validators execute the rest of the block against different contracts and compute different app hashes.

The refresh closes this in two ways, both implemented in `DataContractCache` (`packages/rs-drive/src/cache/data_contract.rs`):

- It marks the contract as **modified in the block**. From then until the block cache is cleared or promoted, a transactional read of that contract is served from the block cache or from state through the transaction, never from the global cache. This holds even if the seeded block-cache entry is evicted, and it is what the proposer relies on when it rolls a transition back: the rollback drops the modified contracts from the block cache, and the next read goes to the rolled-back transaction.
- The block cache is **promoted into the global cache only after the block transaction is committed**, by the `finalize_block` handler, and every committed-state read carries a `CommittedGeneration` snapshot taken before it read state. A read that straddles a commit cannot publish what it read, so a query thread descheduled across the commit cannot clobber the promoted definition with the pre-block one.

## When to Use Finalize Tasks

Finalize tasks are the right tool when you need to perform side effects that:

1. **Must not happen if the batch fails.** If you refresh a cache before the apply and the apply fails, you have seeded a definition the transaction does not hold.

2. **Are not idempotent with respect to partial application.** A cache refresh is fine to do after the apply because the cache will self-heal on the next access. But if your side effect were "send a network message," you would want to be very sure the batch actually committed.

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
- Execute finalize tasks only after confirming the batch was applied successfully.
- Keep finalize task execution fast. They run synchronously in the block processing pipeline.
- Read through the transaction the batch was applied in. What a task seeds must be what that transaction holds, not committed state.

**Do not:**
- Put business logic in finalize tasks. They are for side effects like cache management, not for state mutations. State mutations belong in the batch itself.
- Execute finalize tasks if the batch application returns an error. The whole point is that they only run on success.
- Rely on finalize tasks for exactly-once side effects. If the process crashes between the apply and the finalize task, the task will not run; the cache refresh survives this because Drive's caches are rebuilt from disk on restart.
- Introduce finalize tasks with external side effects (like network calls) without careful consideration of failure modes. Keep them fast, local, and idempotent.
