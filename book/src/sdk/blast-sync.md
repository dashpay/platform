# BLAST Sync

**B**lockchain **L**ayered **A**ddress **S**ync **T**ree (BLAST) is a privacy-preserving
synchronization algorithm used by the Dash Platform SDK. It allows wallets to discover
which of their keys exist in a server-side Merkle tree without revealing the specific
keys being queried.

BLAST is used for two distinct sync tasks:

- **Address balance sync**: Discovering which platform addresses have balances and what
  those balances are.
- **Nullifier sync**: Checking which nullifiers have been spent in the shielded pool.

Both follow the same trunk/branch tree-scan pattern, extracted into a shared generic
algorithm.

## The Problem

A wallet holds a set of keys (addresses or nullifiers) and needs to learn which ones
exist in a Merkle tree stored by Platform nodes. The naive approach -- querying each key
individually -- leaks the wallet's full key set to the server. Even batching the keys
into a single request reveals the exact set.

BLAST solves this by querying *subtrees* of the Merkle tree rather than individual keys.
The server returns a chunk of the tree that contains the target key along with many
other keys, making it impossible for the server to determine which specific key the
wallet cares about.

## Algorithm Overview

The sync has two phases: a **tree scan** for bulk discovery, and **incremental catch-up**
for staying current between scans.

### Phase 1: Tree Scan (Trunk/Branch)

The tree scan proceeds in three steps:

**Step 1 -- Trunk query.** The wallet requests the top-level snapshot of the Merkle tree.
The response contains:
- Elements at the trunk level (keys with their values)
- Leaf boundary keys pointing to subtrees below the trunk
- A Merkle proof covering the entire trunk

For each target key, the wallet classifies it as:
- **Found** -- the key appears directly in the trunk elements
- **Absent** -- the key is proven to not exist (no path to any subtree)
- **Needs deeper query** -- the key traces to a leaf subtree that must be fetched

```
                    ┌─────────────┐
                    │  Trunk Root │  ← Step 1: fetch entire trunk
                    └──────┬──────┘
                   ┌───────┼───────┐
                   ▼       ▼       ▼
                ┌─────┐ ┌─────┐ ┌─────┐
                │Leaf │ │Leaf │ │Leaf │  ← Step 2: classify target keys
                │  A  │ │  B  │ │  C  │
                └──┬──┘ └──┬──┘ └──┬──┘
                   ▼       ▼       ▼
                ┌─────┐ ┌─────┐ ┌─────┐
                │Branch│ │Branch│ │Branch│  ← Step 3: query leaves with
                │Query │ │Query │ │Query │     unresolved keys
                └──────┘ └──────┘ └──────┘
```

**Step 2 -- Privacy adjustment.** Before querying leaf subtrees, the algorithm checks
each leaf's element count. If a leaf contains fewer elements than `min_privacy_count`
(default: 32), the query is expanded to an ancestor subtree that meets the threshold.
This prevents the server from narrowing down which key the wallet is interested in.

```rust
// If a leaf has only 5 elements, the server could guess which one we want.
// Instead, find an ancestor with >= 32 elements.
if count < min_privacy_count {
    // Walk up the tree to find an ancestor with enough elements
    let (ancestor_key, ancestor_info) = trunk_result
        .get_ancestor(&leaf_key, min_privacy_count);
}
```

**Step 3 -- Iterative branch queries.** For each leaf (or privacy-adjusted ancestor),
the wallet sends a branch query specifying:
- The leaf boundary key
- The query depth (how many levels of the subtree to return)
- The expected root hash (for verification)

The server returns the subtree's elements and a Merk proof. The wallet verifies the
proof against the expected hash from the trunk, then classifies each target key again.
Keys that trace to even deeper subtrees are queued for the next iteration.

Branch queries run in parallel with configurable concurrency (`max_concurrent_requests`,
default: 10). The iteration continues until all keys are resolved or `max_iterations`
(default: 50) is reached.

### Phase 2: Incremental Catch-Up

After the tree scan produces a snapshot at some checkpoint height, the wallet needs to
catch up to the chain tip. This is done with two sub-phases:

**Compacted changes** -- Historical balance/nullifier changes aggregated across block
ranges. These cover the gap between the checkpoint height and recent history. Each
response covers a range of blocks and contains the net changes.

**Recent changes** -- Per-block changes for the most recent blocks. These provide
granular updates from where compacted changes left off to the chain tip.

```
  checkpoint_height                              chain_tip
        │                                            │
        ▼                                            ▼
  ──────┬────────────────────────────┬───────────────┤
        │   Compacted changes        │ Recent changes│
        │   (block ranges)           │ (per-block)   │
        └────────────────────────────┴───────────────┘
```

On subsequent syncs, if the elapsed time since the last sync is within
`full_rescan_after_time_s` (default: 7 days), the tree scan is skipped entirely and
only the incremental catch-up runs. This makes frequent re-syncs very fast.

## The TrunkBranchSyncOps Trait

The shared algorithm is parameterized by the `TrunkBranchSyncOps` trait, defined in
`packages/rs-sdk/src/platform/trunk_branch_sync/mod.rs`. Each sync module implements
this trait to plug in its specific query construction, result processing, and
depth limits.

```rust
pub trait TrunkBranchSyncOps {
    /// Module-specific mutable state carried through the scan.
    type Context<'a>: Send where Self: 'a;

    /// Immutable config for parallel branch queries (cloned into each task).
    type BranchQueryConfig: Clone + Send + Sync + 'static;

    // Trunk
    async fn execute_trunk_query(sdk, settings, context)
        -> Result<(GroveTrunkQueryResult, u64, u64), Error>;
    fn process_trunk_result(trunk_result, context, tracker) -> Result<(), Error>;

    // Branch
    fn branch_query_config(context) -> Self::BranchQueryConfig;
    async fn execute_single_branch_query(sdk, config, key, depth, ...)
        -> Result<GroveBranchQueryResult, Error>;
    fn process_branch_result(branch_result, leaf_key, context, tracker)
        -> Result<(), Error>;

    // Limits and hooks
    fn depth_limits(platform_version) -> (u8, u8);
    fn after_branch_iteration(trunk_result, context, tracker) { }
    fn on_branch_query(context);
    fn on_branch_failure(context);
    fn on_elements_seen(context, count);
    fn on_iteration(context, iteration);
    fn set_checkpoint_height(context, height);
}
```

The two associated types deserve attention:

- **`Context<'a>`** is a GAT (generic associated type) that carries mutable state
  through the algorithm. For nullifiers, this holds the input keys and result sets.
  For addresses, it holds the address provider, key-to-index mapping, and result.

- **`BranchQueryConfig`** holds immutable parameters needed to construct branch
  queries that must be sent to async tasks. For nullifiers, this is
  `(pool_type, pool_identifier)`. For addresses, it is `()` since no extra parameters
  are needed.

The `after_branch_iteration` hook allows the address sync module to implement gap-limit
behavior: after each branch iteration, it checks if the provider has extended its
pending address list and adds newly pending keys to the tracker.

## KeyLeafTracker

The `KeyLeafTracker` (in `trunk_branch_sync/tracker.rs`) maintains the mapping between
target keys and the leaf subtrees they reside in. It supports:

- **Adding keys**: When a key traces to a leaf during trunk processing
- **Updating keys**: When a branch query reveals the key is in a deeper subtree
- **Removing keys**: When a key is found or proven absent
- **Reference counting**: Multiple target keys can map to the same leaf; the leaf
  stays active until all its keys are resolved

```rust
let mut tracker = KeyLeafTracker::new();

// After trunk query: key traces to leaf subtree
tracker.add_key(target_key, leaf_boundary_key, leaf_info);

// After branch query: key found in subtree
tracker.key_found(&target_key);

// After branch query: key in even deeper subtree
tracker.update_leaf(&target_key, deeper_leaf_key, deeper_info);

// Check what still needs querying
let active = tracker.active_leaves(); // leaves with unresolved keys
let remaining = tracker.remaining_count();
```

## Privacy-Adjusted Leaves

The `get_privacy_adjusted_leaves` function (in `trunk_branch_sync/mod.rs`) ensures
that branch queries do not leak information about which specific key is being looked up.

For each active leaf in the tracker:

1. If the leaf's element count >= `min_privacy_count`, query it directly.
2. If the count is too low, walk up the trunk to find an ancestor with sufficient
   count. The query depth is adjusted to account for the extra levels.
3. If no suitable ancestor exists (the entire tree is small), query the leaf anyway.

Duplicate ancestors are deduplicated -- if two target keys would both expand to the
same ancestor, only one query is made.

The query depth for each leaf is calculated from the element count and clamped to
platform-version-defined bounds:

```rust
let tree_depth = calculate_max_tree_depth_from_count(count);
let clamped_depth = tree_depth.clamp(min_query_depth, max_query_depth);
```

## Concrete Implementations

### Address Balance Sync

The address sync module (`platform/address_sync/`) implements `TrunkBranchSyncOps`
as `AddressOps<P>` where `P: AddressProvider`.

The `AddressProvider` trait is implemented by wallets to supply:
- The list of pending addresses to check
- Callbacks when addresses are found or proven absent
- Gap-limit extension (generating new addresses when prior ones are found)
- Current balances for incremental-only mode

```rust
// First sync -- full tree scan + incremental catch-up
let result = sdk.sync_address_balances(&mut wallet, None, None).await?;

// Store for next call
let height = result.new_sync_height;
let timestamp = result.new_sync_timestamp;

// Subsequent sync -- incremental only if within 7-day threshold
let result = sdk.sync_address_balances(&mut wallet, None, Some(timestamp)).await?;
```

Address balance sync uses `ItemWithSumItem` GroveDB elements where the item value
contains the nonce (4 bytes big-endian) and the sum value contains the credit balance.

### Nullifier Sync

The nullifier sync module (`platform/nullifier_sync/`) implements `TrunkBranchSyncOps`
as `NullifierOps`.

Nullifier sync differs from address sync in several ways:
- Target keys are fixed 32-byte arrays (`[u8; 32]`)
- Branch queries carry extra config: `(pool_type, pool_identifier)` to identify the
  shielded pool
- No gap-limit behavior (the `after_branch_iteration` hook is not overridden)
- Branch query failures are tracked in metrics

```rust
let nullifiers: Vec<[u8; 32]> = vec![/* ... */];

// First sync -- full tree scan + incremental catch-up
let result = sdk.sync_nullifiers(&nullifiers, None, None, None).await?;

// Store for next call
let height = result.new_sync_height;
let timestamp = result.new_sync_timestamp;

// Subsequent sync -- incremental only if within 7-day threshold
let result = sdk.sync_nullifiers(&nullifiers, None, Some(height), Some(timestamp)).await?;
```

Found nullifiers indicate spent notes; absent nullifiers indicate unspent notes.

## Sync Mode Decision

Both sync modules use the same logic to decide between full scan and incremental-only:

| `last_sync_timestamp` | Elapsed time | Mode |
|----------------------|--------------|------|
| `None` | -- | Full tree scan + catch-up |
| `Some(ts)` | < `full_rescan_after_time_s` | Incremental only |
| `Some(ts)` | >= `full_rescan_after_time_s` | Full tree scan + catch-up |

The default `full_rescan_after_time_s` is 604800 (7 days). Setting it to 0 forces a
full tree scan on every call.

## Configuration

Both modules expose configuration structs with sensible defaults:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `min_privacy_count` | 32 | Minimum elements in a queried subtree |
| `max_concurrent_requests` | 10 | Parallel branch queries |
| `max_iterations` | 50 | Safety limit for branch iteration depth |
| `full_rescan_after_time_s` | 604800 | Seconds before forcing a full rescan |

## Module Structure

```
packages/rs-sdk/src/platform/
├── trunk_branch_sync/
│   ├── mod.rs        # TrunkBranchSyncOps trait, run_full_tree_scan(),
│   │                 #   get_privacy_adjusted_leaves(), parallel execution
│   └── tracker.rs    # KeyLeafTracker with reference counting
├── address_sync/
│   ├── mod.rs        # AddressOps<P> impl, sync_address_balances(),
│   │                 #   incremental_catch_up()
│   ├── provider.rs   # AddressProvider trait
│   └── types.rs      # AddressSyncConfig, AddressSyncResult, AddressFunds
└── nullifier_sync/
    ├── mod.rs        # NullifierOps impl, sync_nullifiers(),
    │                 #   incremental_catch_up()
    ├── provider.rs   # NullifierProvider trait
    └── types.rs      # NullifierSyncConfig, NullifierSyncResult
```

## Rules

**Do:**

- Use `sdk.sync_address_balances()` or `sdk.sync_nullifiers()` as the entry points.
- Persist `new_sync_height` and `new_sync_timestamp` from the result and pass them
  back on the next sync call. This enables incremental-only mode.
- Implement `AddressProvider` to integrate with your wallet's key derivation and
  storage.
- Set `min_privacy_count` high enough that individual key lookups cannot be
  distinguished. The default of 32 is a reasonable minimum.

**Don't:**

- Query individual keys directly via the trunk/branch RPCs -- use the sync functions
  which handle privacy adjustment, iteration, and proof verification.
- Set `max_iterations` too low -- complex trees may need many rounds. The default of
  50 handles trees with millions of entries.
- Ignore the `full_rescan_after_time_s` threshold -- without periodic full rescans,
  the incremental phase could miss changes that occurred before the last known height.
- Skip the incremental catch-up phase -- the tree scan snapshot may be slightly stale
  (the trunk is captured at a specific block height), and the catch-up brings it
  current.
