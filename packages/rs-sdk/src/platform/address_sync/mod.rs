//! Address balance synchronization using trunk/branch chunk queries with
//! incremental catch-up.
//!
//! This module provides privacy-preserving address balance synchronization for wallets.
//! It combines two strategies:
//!
//! 1. **Tree scan** (trunk/branch): Privacy-preserving bulk query of the address tree.
//!    Used for initial sync or when the last sync is stale.
//!
//! 2. **Incremental catch-up** (compacted + recent blocks): Fetches balance changes
//!    block-by-block from a known height to chain tip. Fast for frequent re-syncs.
//!
//! # Sync Modes
//!
//! The behavior depends on [`AddressSyncConfig::last_sync_height`]:
//!
//! - **`None` or `Some(0)`** — Full tree scan, then incremental catch-up from
//!   the tree snapshot to chain tip.
//! - **`Some(height)`** — Incremental-only from that height (unless the height
//!   is too old per `full_rescan_after_time_s`, in which case a full scan runs).
//!
//! # Example
//!
//! ```rust,ignore
//! use dash_sdk::{Sdk, platform::address_sync::{AddressProvider, AddressSyncConfig}};
//!
//! // First sync — full tree scan + catch-up
//! let result = sdk.sync_address_balances(&mut wallet, None, None).await?;
//! let saved_height = result.new_sync_height;       // store for provider.last_sync_height()
//! let saved_timestamp = result.new_sync_timestamp;  // store for last_sync_timestamp param
//!
//! // Subsequent sync — incremental only (unless too old per full_rescan_after_time_s)
//! let result = sdk.sync_address_balances(&mut wallet, None, Some(saved_timestamp)).await?;
//! let saved_height = result.new_sync_height;
//! let saved_timestamp = result.new_sync_timestamp;
//! ```

mod provider;
pub(crate) mod tracker;
mod types;

pub use provider::AddressProvider;
pub use types::{
    AddressFunds, AddressIndex, AddressKey, AddressSyncConfig, AddressSyncMetrics,
    AddressSyncResult, LeafBoundaryKey,
};

use crate::error::Error;
use crate::platform::Fetch;
use crate::sync::retry;
use crate::Sdk;
use dapi_grpc::platform::v0::{
    get_addresses_branch_state_request, get_addresses_branch_state_response,
    get_recent_address_balance_changes_request,
    get_recent_compacted_address_balance_changes_request, GetAddressesBranchStateRequest,
    GetRecentAddressBalanceChangesRequest, GetRecentCompactedAddressBalanceChangesRequest,
};
use dpp::balances::credits::{BlockAwareCreditOperation, CreditOperation};
use dpp::prelude::AddressNonce;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::{
    calculate_max_tree_depth_from_count, Element, GroveBranchQueryResult, GroveTrunkQueryResult,
    LeafInfo,
};
use drive_proof_verifier::types::{
    PlatformAddressTrunkState, RecentAddressBalanceChanges, RecentCompactedAddressBalanceChanges,
};
use futures::stream::{FuturesUnordered, StreamExt};
use rs_dapi_client::{
    DapiRequest, ExecutionError, ExecutionResponse, InnerInto, IntoInner, RequestSettings,
};
use std::collections::{BTreeSet, HashMap};
use tracing::{debug, trace, warn};
use tracker::KeyLeafTracker;

/// Server limit for compacted address balance changes per request.
const COMPACTED_BATCH_LIMIT: usize = 25;
/// Server limit for recent address balance changes per request.
const RECENT_BATCH_LIMIT: usize = 100;

/// Synchronize address balances using trunk/branch chunk queries with
/// incremental block-based catch-up.
///
/// See [module docs](self) for full description of sync modes.
///
/// # Arguments
/// - `sdk`: The SDK instance for making network requests.
/// - `provider`: An implementation of [`AddressProvider`] that supplies addresses.
/// - `config`: Optional configuration; uses defaults if `None`.
/// - `last_sync_timestamp`: Optional block time (Unix seconds) from the previous
///   sync's [`AddressSyncResult::new_sync_timestamp`]. When provided together
///   with a non-zero [`full_rescan_after_time_s`](AddressSyncConfig::full_rescan_after_time_s),
///   the function compares `now - last_sync_timestamp` to decide whether a full
///   tree rescan is needed or incremental-only catch-up suffices.
///   Pass `None` to always perform a full tree scan.
///
/// # Returns
/// - `Ok(AddressSyncResult)`: Contains found addresses with balances/nonces,
///   absent addresses, plus `new_sync_height` and `new_sync_timestamp` to
///   persist for the next call.
/// - `Err(Error)`: If the sync fails after exhausting retries.
pub async fn sync_address_balances<P: AddressProvider>(
    sdk: &Sdk,
    provider: &mut P,
    config: Option<AddressSyncConfig>,
    last_sync_timestamp: Option<u64>,
) -> Result<AddressSyncResult, Error> {
    let config = config.unwrap_or_default();
    let platform_version = sdk.version();

    // Build the index -> key map for looking up indices when we find keys
    let mut key_to_index: HashMap<AddressKey, AddressIndex> = HashMap::new();
    for (index, key) in provider.pending_addresses() {
        key_to_index.insert(key, index);
    }

    // Initialize result
    let mut result = AddressSyncResult::new();

    // If no pending addresses, return early
    if !provider.has_pending() {
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
                    "Address sync: full rescan needed (elapsed {}s >= threshold {}s)",
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
        // Incremental-only mode — skip the tree scan, seed result from current_balances
        let start_height = provider.last_sync_height();
        debug!(
            "Address sync: incremental-only from height {}",
            start_height
        );
        for (index, key, funds) in provider.current_balances() {
            result.found.insert((index, key), funds);
        }
        start_height
    } else {
        // Full tree scan
        let (scan_height, block_time_ms) = full_tree_scan(
            sdk,
            &config,
            provider,
            &mut key_to_index,
            &mut result,
            platform_version,
        )
        .await?;
        // Seed timestamp from the trunk query (may be updated by incremental phase)
        result.new_sync_timestamp = block_time_ms / 1000;
        scan_height
    };

    // Incremental catch-up from catch_up_from to chain tip
    incremental_catch_up(
        sdk,
        &key_to_index,
        catch_up_from,
        provider,
        &mut result,
        config.request_settings,
    )
    .await?;

    // Set highest found index from provider
    result.highest_found_index = provider.highest_found_index();

    Ok(result)
}

// ── Full tree scan ───────────────────────────────────────────────────

/// Perform the full trunk/branch tree scan.
///
/// Returns `(checkpoint_height, block_time_ms)` from the trunk query.
async fn full_tree_scan<P: AddressProvider>(
    sdk: &Sdk,
    config: &AddressSyncConfig,
    provider: &mut P,
    key_to_index: &mut HashMap<AddressKey, AddressIndex>,
    result: &mut AddressSyncResult,
    platform_version: &PlatformVersion,
) -> Result<(u64, u64), Error> {
    // Step 1: Execute trunk query
    let (trunk_result, checkpoint_height, block_time_ms) =
        execute_trunk_query(sdk, config.request_settings, &mut result.metrics).await?;
    result.checkpoint_height = checkpoint_height;

    trace!(
        "Trunk query returned {} elements, {} leaf_keys",
        trunk_result.elements.len(),
        trunk_result.leaf_keys.len()
    );

    // Step 2: Process trunk result
    let mut tracker = KeyLeafTracker::new();
    process_trunk_result(&trunk_result, provider, result, &mut tracker)?;

    // Step 3: Iterative branch queries
    let min_query_depth = platform_version
        .drive
        .methods
        .address_funds
        .address_funds_query_min_depth;
    let max_query_depth = platform_version
        .drive
        .methods
        .address_funds
        .address_funds_query_max_depth;

    let mut iterations = 0;
    while !tracker.is_empty() && iterations < config.max_iterations {
        iterations += 1;
        result.metrics.iterations = iterations;

        // Get leaves that need querying (apply privacy adjustment)
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
            "Iteration {}: querying {} leaves for {} remaining keys",
            iterations,
            leaves_to_query.len(),
            tracker.remaining_count()
        );

        // Execute branch queries in parallel
        let branch_results = execute_branch_queries(
            sdk,
            &leaves_to_query,
            checkpoint_height,
            &mut result.metrics,
            config.max_concurrent_requests,
            config.request_settings,
            platform_version,
        )
        .await?;

        // Process branch results
        for (leaf_key, branch_result) in branch_results {
            process_branch_result(
                &branch_result,
                &leaf_key,
                provider,
                key_to_index,
                result,
                &mut tracker,
            )?;
        }

        // Check if provider has extended pending addresses (gap limit behavior)
        for (index, key) in provider.pending_addresses() {
            if !key_to_index.contains_key(&key) {
                key_to_index.insert(key.clone(), index);
                // New key needs to be traced - it will be picked up in next iteration
                if let Some((leaf_key, info)) = trunk_result.trace_key_to_leaf(&key) {
                    tracker.add_key(key, leaf_key, info);
                }
            }
        }
    }

    if iterations >= config.max_iterations {
        warn!(
            "Address sync reached max iterations ({}) with {} keys remaining",
            config.max_iterations,
            tracker.remaining_count()
        );
    }

    Ok((checkpoint_height, block_time_ms))
}

// ── Incremental catch-up ─────────────────────────────────────────────

/// Perform incremental block-based catch-up using compacted + recent address
/// balance changes RPCs.
///
/// Updates `result.found` with new balance values and sets `result.new_sync_height`.
async fn incremental_catch_up<P: AddressProvider>(
    sdk: &Sdk,
    key_to_index: &HashMap<AddressKey, AddressIndex>,
    start_height: u64,
    provider: &mut P,
    result: &mut AddressSyncResult,
    settings: RequestSettings,
) -> Result<(), Error> {
    // Build a reverse lookup from PlatformAddress bytes to (index, key) for
    // efficient matching against change entries.
    let address_key_lookup: HashMap<Vec<u8>, (AddressIndex, AddressKey)> = key_to_index
        .iter()
        .map(|(key, &index)| (key.clone(), (index, key.clone())))
        .collect();

    let mut current_height = start_height;
    let mut had_successful_query = false;

    // Phase 1 — Compacted (historical) catch-up
    loop {
        let request = GetRecentCompactedAddressBalanceChangesRequest {
            version: Some(
                get_recent_compacted_address_balance_changes_request::Version::V0(
                    get_recent_compacted_address_balance_changes_request::GetRecentCompactedAddressBalanceChangesRequestV0 {
                        start_block_height: current_height,
                        prove: true,
                    },
                ),
            ),
        };

        let (changes, metadata): (Option<RecentCompactedAddressBalanceChanges>, _) =
            match RecentCompactedAddressBalanceChanges::fetch_with_metadata(
                sdk,
                request,
                Some(settings),
            )
            .await
            {
                Ok(result) => result,
                Err(e) if !had_successful_query => {
                    // First query failed — server may not support incremental
                    // RPCs or may not have data for this height. Treat as
                    // "no incremental data available" rather than hard error.
                    debug!(
                        "Compacted address balance changes query failed (non-fatal): {}",
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
            for (platform_addr, credit_op) in &entry.changes {
                let addr_bytes = platform_addr.to_bytes();
                if let Some((index, key)) = address_key_lookup.get(&addr_bytes) {
                    let current_balance = result
                        .found
                        .get(&(*index, key.clone()))
                        .map(|f| f.balance)
                        .unwrap_or(0);

                    let new_balance = match credit_op {
                        BlockAwareCreditOperation::SetCredits(credits) => *credits,
                        BlockAwareCreditOperation::AddToCreditsOperations(operations) => {
                            let total_to_add: u64 = operations
                                .iter()
                                .filter(|(height, _)| **height >= current_height)
                                .map(|(_, credits)| *credits)
                                .fold(0u64, |acc, c| acc.saturating_add(c));
                            current_balance.saturating_add(total_to_add)
                        }
                    };

                    if new_balance != current_balance {
                        let nonce = result
                            .found
                            .get(&(*index, key.clone()))
                            .map(|f| f.nonce)
                            .unwrap_or(0);
                        let funds = AddressFunds {
                            nonce,
                            balance: new_balance,
                        };
                        result.found.insert((*index, key.clone()), funds);
                        provider.on_address_found(*index, key, funds);
                    }
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
        let request = GetRecentAddressBalanceChangesRequest {
            version: Some(
                get_recent_address_balance_changes_request::Version::V0(
                    get_recent_address_balance_changes_request::GetRecentAddressBalanceChangesRequestV0 {
                        start_height: current_height,
                        prove: true,
                    },
                ),
            ),
        };

        let (changes, metadata): (Option<RecentAddressBalanceChanges>, _) =
            match RecentAddressBalanceChanges::fetch_with_metadata(sdk, request, Some(settings))
                .await
            {
                Ok(result) => result,
                Err(e) if !had_successful_query => {
                    debug!(
                        "Recent address balance changes query failed (non-fatal): {}",
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
        result.metrics.recent_queries += 1;
        had_successful_query = true;

        for entry in &entries {
            for (platform_addr, credit_op) in &entry.changes {
                let addr_bytes = platform_addr.to_bytes();
                if let Some((index, key)) = address_key_lookup.get(&addr_bytes) {
                    let current_balance = result
                        .found
                        .get(&(*index, key.clone()))
                        .map(|f| f.balance)
                        .unwrap_or(0);

                    let new_balance = match credit_op {
                        CreditOperation::SetCredits(credits) => *credits,
                        CreditOperation::AddToCredits(credits) => {
                            current_balance.saturating_add(*credits)
                        }
                    };

                    if new_balance != current_balance {
                        let nonce = result
                            .found
                            .get(&(*index, key.clone()))
                            .map(|f| f.nonce)
                            .unwrap_or(0);
                        let funds = AddressFunds {
                            nonce,
                            balance: new_balance,
                        };
                        result.found.insert((*index, key.clone()), funds);
                        provider.on_address_found(*index, key, funds);
                    }
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
    settings: RequestSettings,
    metrics: &mut AddressSyncMetrics,
) -> Result<(GroveTrunkQueryResult, u64, u64), Error> {
    let (trunk_state, metadata) =
        PlatformAddressTrunkState::fetch_with_metadata(sdk, (), Some(settings)).await?;

    metrics.trunk_queries += 1;

    let trunk_state = trunk_state
        .ok_or_else(|| Error::InvalidProvedResponse("Trunk query returned no state".to_string()))?;

    metrics.total_elements_seen += trunk_state.elements.len();

    Ok((trunk_state.into_inner(), metadata.height, metadata.time_ms))
}

/// Process the trunk query result.
fn process_trunk_result<P: AddressProvider>(
    trunk_result: &GroveTrunkQueryResult,
    provider: &mut P,
    result: &mut AddressSyncResult,
    tracker: &mut KeyLeafTracker,
) -> Result<(), Error> {
    // Get pending addresses
    let pending: Vec<(AddressIndex, AddressKey)> = provider.pending_addresses();

    for (index, key) in pending {
        // Check if found in elements
        if let Some(element) = trunk_result.elements.get(&key) {
            let funds = AddressFunds::try_from(element)?;
            result.found.insert((index, key.clone()), funds);
            provider.on_address_found(index, &key, funds);
        } else {
            // Trace to leaf
            if let Some((leaf_key, info)) = trunk_result.trace_key_to_leaf(&key) {
                tracker.add_key(key, leaf_key, info);
            } else {
                // Key is proven absent (not in elements and no leaf subtree)
                result.absent.insert((index, key.clone()));
                provider.on_address_absent(index, &key);
            }
        }
    }

    Ok(())
}

/// Get privacy-adjusted leaves to query.
///
/// For leaves with count below min_privacy_count, find an ancestor with sufficient count.
/// Returns a `Vec` of `(LeafBoundaryKey, LeafInfo, u8)` tuples where the `u8` is the query depth
/// clamped to [`min_query_depth`, `max_query_depth`] to stay within platform limits.
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
        // Clamp to allowed depth range
        let clamped_depth = tree_depth.clamp(min_query_depth, max_query_depth);

        if count >= min_privacy_count {
            // Leaf has sufficient privacy, use it directly
            if seen_ancestors.insert(leaf_key.clone()) {
                result.push((leaf_key, info, clamped_depth));
            }
        } else {
            // Need to find an ancestor with more elements
            if let Some((levels_up, _count, ancestor_key, ancestor_hash)) =
                trunk_result.get_ancestor(&leaf_key, min_privacy_count)
            {
                if seen_ancestors.insert(ancestor_key.clone()) {
                    let ancestor_info = LeafInfo {
                        hash: ancestor_hash,
                        count: Some(_count),
                    };
                    // Reduce depth based on how many levels up we went, then clamp
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
    }

    result
}

/// Execute branch queries in parallel.
async fn execute_branch_queries(
    sdk: &Sdk,
    leaves: &[(LeafBoundaryKey, LeafInfo, u8)],
    checkpoint_height: u64,
    metrics: &mut AddressSyncMetrics,
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

        futures.push(async move {
            execute_single_branch_query(
                &sdk,
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
                        warn!("Branch query failed: {:?}", e);
                        // Continue with other queries
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
                warn!("Branch query failed: {:?}", e);
            }
        }
    }

    Ok(results)
}

/// Execute a single branch query with retry logic.
///
/// If proof verification fails, the request will be retried with a different node
/// according to the retry settings.
async fn execute_single_branch_query(
    sdk: &Sdk,
    key: LeafBoundaryKey,
    depth: u32,
    expected_hash: [u8; 32],
    checkpoint_height: u64,
    settings: RequestSettings,
    platform_version: &PlatformVersion,
) -> Result<GroveBranchQueryResult, Error> {
    let request = GetAddressesBranchStateRequest {
        version: Some(get_addresses_branch_state_request::Version::V0(
            get_addresses_branch_state_request::GetAddressesBranchStateRequestV0 {
                key: key.clone(),
                depth,
                checkpoint_height,
            },
        )),
    };

    let fut = |settings: RequestSettings| {
        let request = request.clone();
        let key = key.clone();
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
                Some(get_addresses_branch_state_response::Version::V0(v0)) => v0.merk_proof,
                None => {
                    return Err(ExecutionError {
                        inner: Error::Proof(drive_proof_verifier::Error::EmptyVersion),
                        address: Some(address),
                        retries,
                    });
                }
            };

            // Verify the proof
            let branch_result = Drive::verify_address_funds_branch_query(
                &proof_bytes,
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

/// Process a branch query result.
fn process_branch_result<P: AddressProvider>(
    branch_result: &GroveBranchQueryResult,
    queried_leaf_key: &[u8],
    provider: &mut P,
    key_to_index: &HashMap<AddressKey, AddressIndex>,
    result: &mut AddressSyncResult,
    tracker: &mut KeyLeafTracker,
) -> Result<(), Error> {
    // Get all target keys that were in this leaf's subtree
    let target_keys = tracker.keys_for_leaf(queried_leaf_key);

    for target_key in target_keys {
        let index = key_to_index.get(&target_key).copied().unwrap_or(0);

        // Check if found in elements
        if let Some(element) = branch_result.elements.get(&target_key) {
            let funds = AddressFunds::try_from(element)?;
            result.found.insert((index, target_key.clone()), funds);
            provider.on_address_found(index, &target_key, funds);
            tracker.key_found(&target_key);
        } else {
            // Try to trace to a deeper leaf
            if let Some((new_leaf_key, info)) = branch_result.trace_key_to_leaf(&target_key) {
                tracker.update_leaf(&target_key, new_leaf_key, info);
            } else {
                // Key is proven absent
                result.absent.insert((index, target_key.clone()));
                provider.on_address_absent(index, &target_key);
                tracker.key_found(&target_key); // Remove from tracking
            }
        }
    }

    result.metrics.total_elements_seen += branch_result.elements.len();
    Ok(())
}

impl TryFrom<&Element> for AddressFunds {
    type Error = Error;

    /// Convert a GroveDB element into address funds (nonce and balance).
    ///
    /// The address funds tree stores the nonce as the item value and the balance as the sum item.
    fn try_from(element: &Element) -> Result<Self, Self::Error> {
        if let Element::ItemWithSumItem(nonce_bytes, balance, _) = element {
            let nonce_bytes: [u8; 4] = nonce_bytes.as_slice().try_into().map_err(|_| {
                Error::InvalidProvedResponse(
                    "address funds nonce must be exactly 4 bytes".to_string(),
                )
            })?;
            let nonce = AddressNonce::from_be_bytes(nonce_bytes);
            let balance: u64 = (*balance).try_into().map_err(|_| {
                Error::InvalidProvedResponse("address funds balance must fit into u64".to_string())
            })?;
            return Ok(AddressFunds { nonce, balance });
        }

        Err(Error::InvalidProvedResponse(
            "unexpected element type for address funds".to_string(),
        ))
    }
}

// SDK integration impl
impl Sdk {
    /// Synchronize address balances using privacy-preserving trunk/branch chunk
    /// queries with incremental block-based catch-up.
    ///
    /// This method discovers address balances for addresses supplied by the provider,
    /// using an iterative query process that fetches chunks of the address tree rather
    /// than individual addresses. This provides privacy by making it unclear which
    /// specific addresses are being queried.
    ///
    /// After the tree scan, incremental catch-up fetches balance changes from the
    /// checkpoint height to chain tip so the result is as fresh as possible.
    ///
    /// On subsequent calls, pass [`AddressSyncResult::new_sync_timestamp`] as
    /// `last_sync_timestamp` so the function can decide whether a full tree
    /// rescan is needed or incremental-only catch-up suffices. The provider
    /// should implement [`AddressProvider::last_sync_height`] (returning the
    /// stored [`AddressSyncResult::new_sync_height`]) and
    /// [`AddressProvider::current_balances`] to supply state from the previous
    /// sync.
    ///
    /// # Arguments
    /// - `provider`: An implementation of [`AddressProvider`] that supplies addresses
    ///   and handles callbacks when addresses are found or proven absent.
    /// - `config`: Optional configuration; uses defaults if `None`.
    /// - `last_sync_timestamp`: Optional block time (Unix seconds) from the
    ///   previous sync's [`AddressSyncResult::new_sync_timestamp`].
    ///   Pass `None` to always perform a full tree scan.
    ///
    /// # Returns
    /// - `Ok(AddressSyncResult)`: Contains found addresses with balances/nonces,
    ///   absent addresses, `new_sync_height` and `new_sync_timestamp` to store
    ///   for the next call.
    /// - `Err(Error)`: If the sync fails after exhausting retries.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use dash_sdk::{Sdk, platform::address_sync::{AddressProvider, AddressSyncConfig}};
    ///
    /// // First sync — full tree scan + catch-up (no timestamp)
    /// let result = sdk.sync_address_balances(&mut wallet, None, None).await?;
    /// let saved_height = result.new_sync_height;       // → provider.last_sync_height()
    /// let saved_timestamp = result.new_sync_timestamp;  // → last_sync_timestamp param
    ///
    /// // Subsequent sync — incremental only if within threshold
    /// let result = sdk.sync_address_balances(&mut wallet, None, Some(saved_timestamp)).await?;
    /// ```
    pub async fn sync_address_balances<P: AddressProvider>(
        &self,
        provider: &mut P,
        config: Option<AddressSyncConfig>,
        last_sync_timestamp: Option<u64>,
    ) -> Result<AddressSyncResult, Error> {
        sync_address_balances(self, provider, config, last_sync_timestamp).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_funds_from_element() {
        let item_with_sum_item = Element::ItemWithSumItem(vec![0, 0, 0, 5], 1000, None);
        let funds = AddressFunds::try_from(&item_with_sum_item).expect("valid funds element");
        assert_eq!(funds.balance, 1000);
        assert_eq!(funds.nonce, 5);

        let item = Element::Item(vec![1, 2, 3], None);
        let err = AddressFunds::try_from(&item).unwrap_err();
        assert!(matches!(err, Error::InvalidProvedResponse(_)));
    }

    #[test]
    fn test_default_config_values() {
        let config = AddressSyncConfig::default();
        assert_eq!(config.full_rescan_after_time_s, 7 * 24 * 60 * 60);
        assert_eq!(config.min_privacy_count, 32);
        assert_eq!(config.max_iterations, 50);
    }

    #[test]
    fn test_default_result_has_zero_new_sync_height() {
        let result = AddressSyncResult::new();
        assert_eq!(result.new_sync_height, 0);
        assert_eq!(result.checkpoint_height, 0);
    }

    #[test]
    fn test_metrics_total_includes_incremental() {
        let metrics = AddressSyncMetrics {
            trunk_queries: 1,
            branch_queries: 3,
            compacted_queries: 2,
            recent_queries: 1,
            ..Default::default()
        };
        assert_eq!(metrics.total_queries(), 7);
    }
}
