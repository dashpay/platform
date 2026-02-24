//! Nullifier synchronization using trunk/branch chunk queries with incremental catch-up.
//!
//! This module provides privacy-preserving nullifier status checking for wallets.
//! It combines two strategies:
//!
//! 1. **Tree scan** (trunk/branch): Privacy-preserving bulk query of the nullifier
//!    Merkle tree. Used for initial sync or when the last sync is stale.
//!
//! 2. **Incremental catch-up** (compacted + recent blocks): Fetches nullifier
//!    changes block-by-block from a known height to chain tip. Fast for frequent
//!    re-syncs.
//!
//! # Sync Modes
//!
//! The behavior depends on the `last_sync_timestamp` parameter passed to
//! [`sync_nullifiers`]:
//!
//! - **`None`** — Full tree scan, then incremental catch-up from the tree
//!   snapshot to chain tip.
//! - **`Some(timestamp)`** — Incremental-only from `last_sync_height`
//!   (unless the elapsed time exceeds
//!   [`NullifierSyncConfig::full_rescan_after_time_s`], in which case a full
//!   scan runs).
//!
//! # Example
//!
//! ```rust,ignore
//! use dash_sdk::Sdk;
//!
//! let nullifiers: Vec<[u8; 32]> = vec![/* ... */];
//!
//! // First sync — full tree scan + catch-up
//! let result = sdk.sync_nullifiers(&nullifiers, None, None, None).await?;
//! let saved_height = result.new_sync_height;       // store for next call
//! let saved_timestamp = result.new_sync_timestamp;  // store for next call
//!
//! // Subsequent sync — incremental only (unless too old per full_rescan_after_time_s)
//! let result = sdk.sync_nullifiers(&nullifiers, None, Some(saved_height), Some(saved_timestamp)).await?;
//! let saved_height = result.new_sync_height;
//! let saved_timestamp = result.new_sync_timestamp;
//! ```

mod provider;
mod types;

pub use provider::NullifierProvider;
pub use types::{NullifierKey, NullifierSyncConfig, NullifierSyncMetrics, NullifierSyncResult};

use crate::error::Error;
use crate::platform::address_sync::tracker::KeyLeafTracker;
use crate::platform::Fetch;
use crate::sync::retry;
use crate::Sdk;
use dapi_grpc::platform::v0::{
    get_nullifiers_branch_state_request, get_nullifiers_branch_state_response,
    get_recent_compacted_nullifier_changes_request, get_recent_nullifier_changes_request,
    GetNullifiersBranchStateRequest, GetRecentCompactedNullifierChangesRequest,
    GetRecentNullifierChangesRequest,
};
use drive::drive::Drive;
use drive::grovedb::{
    calculate_max_tree_depth_from_count, GroveBranchQueryResult, GroveTrunkQueryResult, LeafInfo,
};
use drive_proof_verifier::types::{
    NullifiersTrunkQuery, NullifiersTrunkState, RecentCompactedNullifierChanges,
    RecentNullifierChanges,
};
use futures::stream::{FuturesUnordered, StreamExt};
use rs_dapi_client::{
    DapiRequest, ExecutionError, ExecutionResponse, InnerInto, IntoInner, RequestSettings,
};
use std::collections::{BTreeSet, HashSet};
use tracing::{debug, trace, warn};

use dpp::version::PlatformVersion;

type LeafBoundaryKey = Vec<u8>;

/// Server limit for compacted nullifier changes per request.
const COMPACTED_BATCH_LIMIT: usize = 25;
/// Server limit for recent nullifier changes per request.
const RECENT_BATCH_LIMIT: usize = 100;

/// Synchronize nullifier statuses using trunk/branch chunk queries with
/// incremental block-based catch-up.
///
/// See [module docs](self) for full description of sync modes.
///
/// # Arguments
/// - `sdk`: The SDK instance for making network requests.
/// - `provider`: An implementation of [`NullifierProvider`] that supplies nullifier keys.
/// - `config`: Optional configuration; uses defaults if `None`.
/// - `last_sync_height`: Optional block height from the previous sync's
///   [`NullifierSyncResult::new_sync_height`]. Used as the starting point for
///   incremental-only catch-up.
/// - `last_sync_timestamp`: Optional block time (Unix seconds) from the previous
///   sync's [`NullifierSyncResult::new_sync_timestamp`]. When provided together
///   with a non-zero [`NullifierSyncConfig::full_rescan_after_time_s`], the
///   function compares `now - last_sync_timestamp` to decide whether a full tree
///   rescan is needed or incremental-only catch-up suffices.
///   Pass `None` to always perform a full tree scan.
///
/// # Returns
/// - `Ok(NullifierSyncResult)`: Contains found (spent) and absent (unspent)
///   nullifiers, plus `new_sync_height` and `new_sync_timestamp` to persist
///   for the next call.
/// - `Err(Error)`: If the sync fails after exhausting retries.
pub async fn sync_nullifiers<P: NullifierProvider>(
    sdk: &Sdk,
    provider: &P,
    config: Option<NullifierSyncConfig>,
    last_sync_height: Option<u64>,
    last_sync_timestamp: Option<u64>,
) -> Result<NullifierSyncResult, Error> {
    let config = config.unwrap_or_default();
    let platform_version = sdk.version();

    let nullifiers = provider.nullifiers_to_check();

    let mut result = NullifierSyncResult::new();

    if nullifiers.is_empty() {
        return Ok(result);
    }

    // Decide whether to do a full tree scan or incremental-only.
    //
    // Incremental-only is chosen when ALL of these are true:
    //   1. last_sync_timestamp is provided
    //   2. full_rescan_after_time_s > 0
    //   3. elapsed time since last sync < full_rescan_after_time_s
    let needs_full_scan = match last_sync_timestamp {
        Some(last_ts) if config.full_rescan_after_time_s > 0 => {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let elapsed = now_secs.saturating_sub(last_ts);
            if elapsed >= config.full_rescan_after_time_s {
                debug!(
                    "Nullifier sync: full rescan needed (elapsed {}s >= threshold {}s)",
                    elapsed, config.full_rescan_after_time_s
                );
                true
            } else {
                false
            }
        }
        _ => true,
    };

    let catch_up_from = if !needs_full_scan {
        // Incremental-only mode — skip the tree scan
        let start_height = last_sync_height.unwrap_or(0);
        debug!(
            "Nullifier sync: incremental-only from height {}",
            start_height
        );
        start_height
    } else {
        // Full tree scan
        let (scan_height, block_time_ms) =
            full_tree_scan(sdk, &config, &nullifiers, &mut result, platform_version).await?;
        result.new_sync_timestamp = block_time_ms / 1000;
        scan_height
    };

    // Incremental catch-up from catch_up_from to chain tip
    let nullifier_set: HashSet<NullifierKey> = nullifiers.iter().copied().collect();
    incremental_catch_up(
        sdk,
        &nullifier_set,
        catch_up_from,
        &mut result,
        config.request_settings,
    )
    .await?;

    Ok(result)
}

/// Perform the full trunk/branch tree scan.
///
/// Returns `(checkpoint_height, block_time_ms)` from the trunk query.
async fn full_tree_scan(
    sdk: &Sdk,
    config: &NullifierSyncConfig,
    nullifiers: &[NullifierKey],
    result: &mut NullifierSyncResult,
    platform_version: &PlatformVersion,
) -> Result<(u64, u64), Error> {
    // Step 1: Execute trunk query
    let (trunk_result, checkpoint_height, block_time_ms) =
        execute_trunk_query(sdk, config, config.request_settings, &mut result.metrics).await?;
    result.checkpoint_height = checkpoint_height;

    trace!(
        "Nullifier trunk query returned {} elements, {} leaf_keys",
        trunk_result.elements.len(),
        trunk_result.leaf_keys.len()
    );

    // Step 2: Process trunk result
    let mut tracker = KeyLeafTracker::new();
    process_trunk_result(&trunk_result, nullifiers, result, &mut tracker);

    // Step 3: Iterative branch queries
    let min_query_depth = platform_version
        .drive
        .methods
        .shielded
        .nullifiers_query_min_depth;
    let max_query_depth = platform_version
        .drive
        .methods
        .shielded
        .nullifiers_query_max_depth;

    let mut iterations = 0;
    while !tracker.is_empty() && iterations < config.max_iterations {
        iterations += 1;
        result.metrics.iterations = iterations;

        let leaves_to_query = get_privacy_adjusted_leaves(
            &tracker,
            &trunk_result,
            config.min_privacy_count,
            min_query_depth,
            max_query_depth,
        );

        if leaves_to_query.is_empty() {
            break;
        }

        debug!(
            "Iteration {}: querying {} leaves for {} remaining nullifiers",
            iterations,
            leaves_to_query.len(),
            tracker.remaining_count()
        );

        let branch_results = execute_branch_queries(
            sdk,
            config,
            &leaves_to_query,
            checkpoint_height,
            &mut result.metrics,
            config.max_concurrent_requests,
            config.request_settings,
            platform_version,
        )
        .await?;

        for (leaf_key, branch_result) in branch_results {
            process_branch_result(&branch_result, &leaf_key, result, &mut tracker);
        }
    }

    if iterations >= config.max_iterations {
        warn!(
            "Nullifier sync reached max iterations ({}) with {} keys remaining",
            config.max_iterations,
            tracker.remaining_count()
        );
    }

    Ok((checkpoint_height, block_time_ms))
}

/// Perform incremental block-based catch-up using compacted + recent nullifier
/// changes RPCs.
///
/// Updates `result.new_sync_height` and `result.new_sync_timestamp`.
async fn incremental_catch_up(
    sdk: &Sdk,
    nullifier_set: &HashSet<NullifierKey>,
    start_height: u64,
    result: &mut NullifierSyncResult,
    settings: RequestSettings,
) -> Result<(), Error> {
    let mut current_height = start_height;
    let mut had_successful_query = false;

    // Phase 1 — Compacted (historical) catch-up
    loop {
        let request = GetRecentCompactedNullifierChangesRequest {
            version: Some(
                get_recent_compacted_nullifier_changes_request::Version::V0(
                    get_recent_compacted_nullifier_changes_request::GetRecentCompactedNullifierChangesRequestV0 {
                        start_block_height: current_height,
                        prove: true,
                    },
                ),
            ),
        };

        let (changes, metadata): (Option<RecentCompactedNullifierChanges>, _) =
            match RecentCompactedNullifierChanges::fetch_with_metadata(sdk, request, Some(settings))
                .await
            {
                Ok(result) => result,
                Err(e) if !had_successful_query => {
                    debug!(
                        "Compacted nullifier changes query failed (non-fatal): {}",
                        e
                    );
                    break;
                }
                Err(e) => return Err(e),
            };

        let entries = match changes {
            Some(c) => c.into_inner(),
            None => break,
        };

        result.new_sync_timestamp = metadata.time_ms / 1000;

        if entries.is_empty() {
            break;
        }

        let entry_count = entries.len();
        result.metrics.compacted_queries += 1;
        had_successful_query = true;

        for entry in &entries {
            for nf_bytes in &entry.nullifiers {
                if nullifier_set.contains(nf_bytes) {
                    result.found.insert(*nf_bytes);
                }
            }
            if entry.end_block_height.saturating_add(1) > current_height {
                current_height = entry.end_block_height.saturating_add(1);
            }
        }

        if entry_count < COMPACTED_BATCH_LIMIT {
            break;
        }
    }

    // Phase 2 — Recent (per-block) changes
    loop {
        let request = GetRecentNullifierChangesRequest {
            version: Some(get_recent_nullifier_changes_request::Version::V0(
                get_recent_nullifier_changes_request::GetRecentNullifierChangesRequestV0 {
                    start_height: current_height,
                    prove: true,
                },
            )),
        };

        let (changes, metadata): (Option<RecentNullifierChanges>, _) =
            match RecentNullifierChanges::fetch_with_metadata(sdk, request, Some(settings)).await {
                Ok(result) => result,
                Err(e) if !had_successful_query => {
                    debug!("Recent nullifier changes query failed (non-fatal): {}", e);
                    break;
                }
                Err(e) => return Err(e),
            };

        let entries = match changes {
            Some(c) => c.into_inner(),
            None => break,
        };

        result.new_sync_timestamp = metadata.time_ms / 1000;

        if entries.is_empty() {
            break;
        }

        let entry_count = entries.len();
        result.metrics.recent_queries += 1;
        had_successful_query = true;

        for entry in &entries {
            for nf_bytes in &entry.nullifiers {
                if nullifier_set.contains(nf_bytes) {
                    result.found.insert(*nf_bytes);
                }
            }
            if entry.block_height.saturating_add(1) > current_height {
                current_height = entry.block_height.saturating_add(1);
            }
        }

        if entry_count < RECENT_BATCH_LIMIT {
            break;
        }
    }

    result.new_sync_height = current_height;
    Ok(())
}

// ── Tree scan helpers ────────────────────────────────────────────────

/// Execute the trunk query and return the verified result.
///
/// Returns `(trunk_result, checkpoint_height, block_time_ms)`.
async fn execute_trunk_query(
    sdk: &Sdk,
    config: &NullifierSyncConfig,
    settings: RequestSettings,
    metrics: &mut NullifierSyncMetrics,
) -> Result<(GroveTrunkQueryResult, u64, u64), Error> {
    let trunk_query = NullifiersTrunkQuery {
        pool_type: config.pool_type,
        pool_identifier: config.pool_identifier.clone(),
    };

    let (trunk_state, metadata) =
        NullifiersTrunkState::fetch_with_metadata(sdk, trunk_query, Some(settings)).await?;

    metrics.trunk_queries += 1;

    let trunk_state = trunk_state.ok_or_else(|| {
        Error::InvalidProvedResponse("Nullifier trunk query returned no state".to_string())
    })?;

    metrics.total_elements_seen += trunk_state.elements.len();

    Ok((trunk_state.into_inner(), metadata.height, metadata.time_ms))
}

/// Process the trunk query result.
fn process_trunk_result(
    trunk_result: &GroveTrunkQueryResult,
    nullifiers: &[NullifierKey],
    result: &mut NullifierSyncResult,
    tracker: &mut KeyLeafTracker,
) {
    for key in nullifiers {
        let key_vec = key.to_vec();

        if trunk_result.elements.contains_key(&key_vec) {
            // Nullifier found in tree — the note is spent
            result.found.insert(*key);
        } else if let Some((leaf_key, info)) = trunk_result.trace_key_to_leaf(&key_vec) {
            // Not in trunk elements, but traces to a leaf subtree
            tracker.add_key(key_vec, leaf_key, info);
        } else {
            // Proven absent — the note is unspent
            result.absent.insert(*key);
        }
    }
}

/// Get privacy-adjusted leaves to query.
///
/// For leaves with count below min_privacy_count, find an ancestor with sufficient count.
fn get_privacy_adjusted_leaves(
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

/// Execute branch queries in parallel.
async fn execute_branch_queries(
    sdk: &Sdk,
    config: &NullifierSyncConfig,
    leaves: &[(LeafBoundaryKey, LeafInfo, u8)],
    checkpoint_height: u64,
    metrics: &mut NullifierSyncMetrics,
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
        let pool_type = config.pool_type;
        let pool_identifier = config.pool_identifier.clone();

        futures.push(async move {
            execute_single_branch_query(
                &sdk,
                pool_type,
                pool_identifier.as_deref(),
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
                        metrics.branch_queries += 1;
                        results.push((key, branch_result));
                    }
                    Err(e) => {
                        warn!("Nullifier branch query failed: {:?}", e);
                    }
                }
            }
        }
    }

    // Collect remaining futures
    while let Some(result) = futures.next().await {
        match result {
            Ok((key, branch_result)) => {
                metrics.branch_queries += 1;
                results.push((key, branch_result));
            }
            Err(e) => {
                warn!("Nullifier branch query failed: {:?}", e);
            }
        }
    }

    Ok(results)
}

/// Execute a single branch query with retry logic.
async fn execute_single_branch_query(
    sdk: &Sdk,
    pool_type: u32,
    pool_identifier: Option<&[u8]>,
    key: LeafBoundaryKey,
    depth: u32,
    expected_hash: [u8; 32],
    checkpoint_height: u64,
    settings: RequestSettings,
    platform_version: &PlatformVersion,
) -> Result<GroveBranchQueryResult, Error> {
    let pool_id_owned = pool_identifier.map(|p| p.to_vec());

    let request = GetNullifiersBranchStateRequest {
        version: Some(get_nullifiers_branch_state_request::Version::V0(
            get_nullifiers_branch_state_request::GetNullifiersBranchStateRequestV0 {
                pool_type,
                pool_identifier: pool_id_owned.clone().unwrap_or_default(),
                key: key.clone(),
                depth,
                checkpoint_height,
            },
        )),
    };

    let fut = |settings: RequestSettings| {
        let request = request.clone();
        let key = key.clone();
        let pool_id_owned = pool_id_owned.clone();
        async move {
            let ExecutionResponse {
                address,
                retries,
                inner: response,
            } = request
                .execute(sdk, settings)
                .await
                .map_err(|execution_error| execution_error.inner_into())?;

            // Extract merk proof
            let proof_bytes = match response.version {
                Some(get_nullifiers_branch_state_response::Version::V0(v0)) => v0.merk_proof,
                None => {
                    return Err(ExecutionError {
                        inner: Error::Proof(drive_proof_verifier::Error::EmptyVersion),
                        address: Some(address),
                        retries,
                    });
                }
            };

            // Verify the proof
            let branch_result = Drive::verify_nullifiers_branch_query(
                &proof_bytes,
                pool_type,
                pool_id_owned.as_deref(),
                key,
                depth as u8,
                expected_hash,
                platform_version,
            )
            .map_err(|e| ExecutionError {
                inner: e.into(),
                address: Some(address.clone()),
                retries,
            })?;

            Ok(ExecutionResponse {
                inner: branch_result,
                address,
                retries,
            })
        }
    };

    let settings = sdk.dapi_client_settings.override_by(settings);

    retry(sdk.address_list(), settings, fut).await.into_inner()
}

/// Process a branch query result for nullifier presence.
fn process_branch_result(
    branch_result: &GroveBranchQueryResult,
    queried_leaf_key: &[u8],
    result: &mut NullifierSyncResult,
    tracker: &mut KeyLeafTracker,
) {
    let target_keys = tracker.keys_for_leaf(queried_leaf_key);

    for target_key in target_keys {
        if branch_result.elements.contains_key(&target_key) {
            // Nullifier found — note is spent
            if let Ok(nf) = <[u8; 32]>::try_from(target_key.as_slice()) {
                result.found.insert(nf);
            }
            tracker.key_found(&target_key);
        } else if let Some((new_leaf_key, info)) = branch_result.trace_key_to_leaf(&target_key) {
            // Traces to a deeper leaf — need another iteration
            tracker.update_leaf(&target_key, new_leaf_key, info);
        } else {
            // Proven absent — note is unspent
            if let Ok(nf) = <[u8; 32]>::try_from(target_key.as_slice()) {
                result.absent.insert(nf);
            }
            tracker.key_found(&target_key); // Remove from tracking
        }
    }

    result.metrics.total_elements_seen += branch_result.elements.len();
}

// ── SDK integration ──────────────────────────────────────────────────

impl Sdk {
    /// Synchronize nullifier statuses with incremental catch-up support.
    ///
    /// This is the main entry point for nullifier synchronization. It handles
    /// both full tree scans and incremental block-based catch-up, depending on
    /// the parameters.
    ///
    /// On subsequent calls, pass [`NullifierSyncResult::new_sync_height`] as
    /// `last_sync_height` and [`NullifierSyncResult::new_sync_timestamp`] as
    /// `last_sync_timestamp` so the function can decide whether a full tree
    /// rescan is needed or incremental-only catch-up suffices.
    ///
    /// # Arguments
    /// - `provider`: An implementation of [`NullifierProvider`] that supplies nullifier keys.
    /// - `config`: Optional configuration; uses defaults if `None`.
    /// - `last_sync_height`: Optional block height from the previous sync's
    ///   [`NullifierSyncResult::new_sync_height`]. Used as the starting point
    ///   for incremental-only catch-up.
    /// - `last_sync_timestamp`: Optional block time (Unix seconds) from the
    ///   previous sync's [`NullifierSyncResult::new_sync_timestamp`].
    ///   Pass `None` to always perform a full tree scan.
    ///
    /// # Returns
    /// - `Ok(NullifierSyncResult)`: Contains found (spent) and absent (unspent)
    ///   nullifiers, `new_sync_height` and `new_sync_timestamp` to store for
    ///   the next call.
    /// - `Err(Error)`: If the sync fails after exhausting retries.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use dash_sdk::Sdk;
    ///
    /// let sdk = Sdk::new(/* ... */);
    /// let nullifiers: Vec<[u8; 32]> = vec![/* known nullifiers */];
    ///
    /// // First call — full scan
    /// let result = sdk.sync_nullifiers(&nullifiers, None, None, None).await?;
    /// let height = result.new_sync_height;       // → last_sync_height param
    /// let timestamp = result.new_sync_timestamp;  // → last_sync_timestamp param
    ///
    /// // Next call — incremental only if within threshold
    /// let result = sdk.sync_nullifiers(&nullifiers, None, Some(height), Some(timestamp)).await?;
    /// ```
    pub async fn sync_nullifiers<P: NullifierProvider>(
        &self,
        provider: &P,
        config: Option<NullifierSyncConfig>,
        last_sync_height: Option<u64>,
        last_sync_timestamp: Option<u64>,
    ) -> Result<NullifierSyncResult, Error> {
        sync_nullifiers(
            self,
            provider,
            config,
            last_sync_height,
            last_sync_timestamp,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_provider() {
        let nullifiers: Vec<NullifierKey> = vec![[0u8; 32], [1u8; 32]];
        let result = nullifiers.nullifiers_to_check();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_btreeset_provider() {
        let mut set = BTreeSet::new();
        set.insert([0u8; 32]);
        set.insert([1u8; 32]);
        let result = set.nullifiers_to_check();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_slice_provider() {
        let nullifiers = [[0u8; 32], [1u8; 32]];
        let slice: &[NullifierKey] = &nullifiers;
        let result = slice.nullifiers_to_check();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_empty_provider_returns_empty() {
        let nullifiers: Vec<NullifierKey> = vec![];
        let result = nullifiers.nullifiers_to_check();
        assert!(result.is_empty());
    }

    #[test]
    fn test_default_config() {
        let config = NullifierSyncConfig::default();
        assert_eq!(config.min_privacy_count, 32);
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.pool_type, 0);
        assert!(config.pool_identifier.is_none());
        assert_eq!(config.full_rescan_after_time_s, 7 * 24 * 60 * 60);
    }

    #[test]
    fn test_result_default() {
        let result = NullifierSyncResult::new();
        assert!(result.found.is_empty());
        assert!(result.absent.is_empty());
        assert_eq!(result.checkpoint_height, 0);
        assert_eq!(result.new_sync_height, 0);
        assert_eq!(result.new_sync_timestamp, 0);
        assert_eq!(result.metrics.total_queries(), 0);
    }

    #[test]
    fn test_metrics_total_queries() {
        let mut metrics = NullifierSyncMetrics::default();
        metrics.trunk_queries = 1;
        metrics.branch_queries = 3;
        metrics.compacted_queries = 2;
        metrics.recent_queries = 1;
        assert_eq!(metrics.total_queries(), 7);
    }
}
