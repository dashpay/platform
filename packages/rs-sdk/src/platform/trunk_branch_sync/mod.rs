//! Shared trunk/branch tree-scan algorithm for privacy-preserving synchronization.
//!
//! This module extracts the common iterative pattern used by both nullifier sync
//! and address balance sync.  Both follow the same algorithm:
//!
//! 1. **Trunk query** -- fetch the top-level snapshot of a Merkle tree.
//! 2. **Process trunk result** -- classify each target key as *found*, *absent*,
//!    or *needs deeper query* (traced to a leaf subtree).
//! 3. **Iterative branch queries** -- for leaves that still have unresolved keys,
//!    apply privacy adjustment, batch branch queries with concurrency control,
//!    and process results.  Repeat until all keys are resolved or the iteration
//!    limit is reached.
//!
//! Concrete sync modules implement [`TrunkBranchSyncOps`] to plug in their
//! specific trunk/branch request construction, result processing, and depth
//! limits, then call [`run_full_tree_scan`] to execute the shared algorithm.
//!
//! The [`KeyLeafTracker`] and [`get_privacy_adjusted_leaves`] helpers are also
//! exported from here for use by both modules.

mod tracker;

pub use tracker::KeyLeafTracker;

use crate::error::Error;
use crate::Sdk;
use dpp::version::PlatformVersion;
use drive::grovedb::{
    calculate_max_tree_depth_from_count, GroveBranchQueryResult, GroveTrunkQueryResult, LeafInfo,
};
use futures::stream::{FuturesUnordered, StreamExt};
use rs_dapi_client::RequestSettings;
use std::collections::BTreeSet;
use tracing::{debug, warn};

/// Alias for leaf boundary keys used in trunk/branch queries.
pub type LeafBoundaryKey = Vec<u8>;

// ── Trait ────────────────────────────────────────────────────────────

/// Operations that parameterize the trunk/branch tree-scan algorithm.
///
/// Each concrete sync module (nullifiers, address balances) implements this
/// trait to supply its specific query construction, result processing, and
/// depth limits.
///
/// # Associated types
///
/// - [`Context`](Self::Context) carries module-specific mutable state (result
///   sets, providers, lookup maps) through the algorithm.
/// - [`BranchQueryConfig`](Self::BranchQueryConfig) holds immutable parameters
///   needed for branch query construction that must be sent to async tasks
///   (e.g. `pool_type` / `pool_identifier` for nullifiers).
#[allow(async_fn_in_trait)]
pub trait TrunkBranchSyncOps {
    /// Module-specific mutable context carried through the scan.
    ///
    /// For nullifiers: holds `(&[NullifierKey], &mut NullifierSyncResult)`.
    /// For addresses: holds `(&mut P, &mut HashMap, &mut AddressSyncResult)`.
    type Context<'a>: Send
    where
        Self: 'a;

    /// Immutable configuration extracted from the context before spawning
    /// parallel branch queries.  Must be cheaply cloneable and `Send + Sync`.
    ///
    /// For nullifiers: `(u32, Option<[u8; 32]>)` (pool_type, pool_identifier).
    /// For addresses: `()`.
    type BranchQueryConfig: Clone + Send + Sync + 'static;

    // ── Trunk query ──────────────────────────────────────────────

    /// Execute the trunk query and return `(trunk_result, checkpoint_height, block_time_ms)`.
    async fn execute_trunk_query(
        sdk: &Sdk,
        settings: RequestSettings,
        context: &mut Self::Context<'_>,
    ) -> Result<(GroveTrunkQueryResult, u64, u64), Error>;

    // ── Trunk result processing ──────────────────────────────────

    /// Process the trunk query result: classify each target key as found,
    /// absent, or needing a deeper branch query.
    ///
    /// Implementations iterate over their target keys and call the
    /// appropriate methods on `tracker` for keys that trace to leaf subtrees.
    fn process_trunk_result(
        trunk_result: &GroveTrunkQueryResult,
        context: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) -> Result<(), Error>;

    // ── Branch query config ──────────────────────────────────────

    /// Extract the branch query configuration from the context.
    ///
    /// Called once before the branch query loop begins. The returned value
    /// is cloned into each parallel branch query task.
    fn branch_query_config(context: &Self::Context<'_>) -> Self::BranchQueryConfig;

    // ── Branch query ─────────────────────────────────────────────

    /// Execute a single branch query and return the verified result.
    ///
    /// The `config` parameter carries module-specific data needed to
    /// construct the request (e.g. pool_type for nullifiers).
    #[allow(clippy::too_many_arguments)]
    async fn execute_single_branch_query(
        sdk: &Sdk,
        config: &Self::BranchQueryConfig,
        key: LeafBoundaryKey,
        depth: u32,
        expected_hash: [u8; 32],
        checkpoint_height: u64,
        settings: RequestSettings,
        platform_version: &PlatformVersion,
    ) -> Result<GroveBranchQueryResult, Error>;

    // ── Branch result processing ─────────────────────────────────

    /// Process a branch query result: for each target key in the queried
    /// leaf, classify it as found, absent, or needing a deeper query.
    fn process_branch_result(
        branch_result: &GroveBranchQueryResult,
        queried_leaf_key: &[u8],
        context: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) -> Result<(), Error>;

    // ── Depth limits ─────────────────────────────────────────────

    /// Return `(min_query_depth, max_query_depth)` from the platform version.
    fn depth_limits(platform_version: &PlatformVersion) -> (u8, u8);

    // ── Post-iteration hook ──────────────────────────────────────

    /// Called after each branch iteration completes.
    ///
    /// This allows the address sync module to extend pending addresses
    /// (gap-limit behavior) and add newly pending keys to the tracker.
    /// The default implementation does nothing.
    fn after_branch_iteration(
        _trunk_result: &GroveTrunkQueryResult,
        _context: &mut Self::Context<'_>,
        _tracker: &mut KeyLeafTracker,
    ) {
    }

    // ── Metrics hooks ────────────────────────────────────────────

    /// Increment the branch query count in the context's metrics.
    fn on_branch_query(context: &mut Self::Context<'_>);

    /// Record a branch query failure in the context's metrics.
    fn on_branch_failure(context: &mut Self::Context<'_>);

    /// Add to the total elements seen count in the context's metrics.
    fn on_elements_seen(context: &mut Self::Context<'_>, count: usize);

    /// Set the iteration count in the context's metrics.
    fn on_iteration(context: &mut Self::Context<'_>, iteration: usize);

    /// Set the checkpoint height in the context's result.
    fn set_checkpoint_height(context: &mut Self::Context<'_>, height: u64);
}

// ── Shared algorithm ─────────────────────────────────────────────────

/// Run the full trunk/branch tree scan.
///
/// Returns `(checkpoint_height, block_time_ms)` from the trunk query.
///
/// # Type Parameters
/// - `Ops`: The concrete sync operations (nullifier or address).
pub async fn run_full_tree_scan<Ops: TrunkBranchSyncOps>(
    sdk: &Sdk,
    min_privacy_count: u64,
    max_iterations: usize,
    max_concurrent: usize,
    request_settings: RequestSettings,
    context: &mut Ops::Context<'_>,
) -> Result<(u64, u64), Error> {
    let platform_version = sdk.version();

    // Step 1: Execute trunk query
    let (trunk_result, checkpoint_height, block_time_ms) =
        Ops::execute_trunk_query(sdk, request_settings, context).await?;
    Ops::set_checkpoint_height(context, checkpoint_height);

    // Step 2: Process trunk result
    let mut tracker = KeyLeafTracker::new();
    Ops::process_trunk_result(&trunk_result, context, &mut tracker)?;

    // Step 3: Iterative branch queries
    let (min_query_depth, max_query_depth) = Ops::depth_limits(platform_version);
    let branch_config = Ops::branch_query_config(context);

    let mut iterations = 0;
    while !tracker.is_empty() && iterations < max_iterations {
        iterations += 1;
        Ops::on_iteration(context, iterations);

        let leaves_to_query = get_privacy_adjusted_leaves(
            &tracker,
            &trunk_result,
            min_privacy_count,
            min_query_depth,
            max_query_depth,
        );

        if leaves_to_query.is_empty() {
            break;
        }

        debug!(
            "Iteration {}: querying {} leaves for {} remaining keys",
            iterations,
            leaves_to_query.len(),
            tracker.remaining_count()
        );

        let branch_results = execute_branch_queries_parallel::<Ops>(
            sdk,
            &branch_config,
            &leaves_to_query,
            checkpoint_height,
            context,
            max_concurrent,
            request_settings,
            platform_version,
        )
        .await?;

        for (leaf_key, branch_result) in branch_results {
            Ops::process_branch_result(&branch_result, &leaf_key, context, &mut tracker)?;
            Ops::on_elements_seen(context, branch_result.elements.len());
        }

        // Post-iteration hook (gap-limit extension for addresses)
        Ops::after_branch_iteration(&trunk_result, context, &mut tracker);
    }

    if iterations >= max_iterations {
        warn!(
            "Sync reached max iterations ({}) with {} keys remaining",
            max_iterations,
            tracker.remaining_count()
        );
    }

    Ok((checkpoint_height, block_time_ms))
}

// ── Privacy-adjusted leaves ──────────────────────────────────────────

/// Get privacy-adjusted leaves to query.
///
/// For leaves with element count below `min_privacy_count`, find an ancestor
/// in the trunk result that has sufficient count, so the server cannot
/// distinguish which specific key is being queried.
///
/// Returns `(leaf_key, leaf_info, clamped_depth)` tuples.
pub fn get_privacy_adjusted_leaves(
    tracker: &KeyLeafTracker,
    trunk_result: &GroveTrunkQueryResult,
    min_privacy_count: u64,
    min_query_depth: u8,
    max_query_depth: u8,
) -> Vec<(LeafBoundaryKey, LeafInfo, u8)> {
    let active_leaves = tracker.active_leaves();
    let mut result = Vec::new();
    let mut seen_ancestors: BTreeSet<LeafBoundaryKey> = BTreeSet::new();

    for (leaf_key, info) in active_leaves {
        let count = info.count.unwrap_or(0);
        let tree_depth = calculate_max_tree_depth_from_count(count);
        let clamped_depth = tree_depth.clamp(min_query_depth, max_query_depth);

        if count >= min_privacy_count {
            if seen_ancestors.insert(leaf_key.clone()) {
                result.push((leaf_key, info, clamped_depth));
            }
        } else if let Some((levels_up, ancestor_count, ancestor_key, ancestor_hash)) =
            trunk_result.get_ancestor(&leaf_key, min_privacy_count)
        {
            if seen_ancestors.insert(ancestor_key.clone()) {
                let ancestor_info = LeafInfo {
                    hash: ancestor_hash,
                    count: Some(ancestor_count),
                };
                let depth = tree_depth
                    .saturating_sub(levels_up)
                    .clamp(min_query_depth, max_query_depth);
                result.push((ancestor_key, ancestor_info, depth));
            }
        } else {
            // No suitable ancestor found, use the leaf anyway
            if seen_ancestors.insert(leaf_key.clone()) {
                result.push((leaf_key, info, clamped_depth));
            }
        }
    }

    result
}

// ── Parallel branch execution ────────────────────────────────────────

/// Execute branch queries in parallel with concurrency limiting.
///
/// The `branch_config` carries immutable data needed by each branch query
/// (cloned into each spawned future). The `context` is only used for metrics
/// updates in the main task after futures complete.
#[allow(clippy::too_many_arguments)]
async fn execute_branch_queries_parallel<Ops: TrunkBranchSyncOps>(
    sdk: &Sdk,
    branch_config: &Ops::BranchQueryConfig,
    leaves: &[(LeafBoundaryKey, LeafInfo, u8)],
    checkpoint_height: u64,
    context: &mut Ops::Context<'_>,
    max_concurrent: usize,
    settings: RequestSettings,
    platform_version: &PlatformVersion,
) -> Result<Vec<(LeafBoundaryKey, GroveBranchQueryResult)>, Error> {
    let mut futures = FuturesUnordered::new();
    let mut results = Vec::new();

    for (leaf_key, info, depth) in leaves.iter().cloned() {
        let sdk = sdk.clone();
        let expected_hash = info.hash;
        let depth_u32 = depth as u32;
        let config = branch_config.clone();

        futures.push(async move {
            Ops::execute_single_branch_query(
                &sdk,
                &config,
                leaf_key.clone(),
                depth_u32,
                expected_hash,
                checkpoint_height,
                settings,
                platform_version,
            )
            .await
            .map(|result| (leaf_key, result))
        });

        // Limit concurrency
        if futures.len() >= max_concurrent {
            if let Some(result) = futures.next().await {
                match result {
                    Ok((key, branch_result)) => {
                        Ops::on_branch_query(context);
                        results.push((key, branch_result));
                    }
                    Err(e) => {
                        warn!("Branch query failed: {:?}", e);
                        Ops::on_branch_failure(context);
                    }
                }
            }
        }
    }

    // Collect remaining futures
    while let Some(result) = futures.next().await {
        match result {
            Ok((key, branch_result)) => {
                Ops::on_branch_query(context);
                results.push((key, branch_result));
            }
            Err(e) => {
                warn!("Branch query failed: {:?}", e);
                Ops::on_branch_failure(context);
            }
        }
    }

    Ok(results)
}
