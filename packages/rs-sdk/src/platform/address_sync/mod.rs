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
//! The behavior depends on the `last_sync_timestamp` parameter passed to
//! [`sync_address_balances`]:
//!
//! - **`None`** — Full tree scan, then incremental catch-up from the tree
//!   snapshot to chain tip.
//! - **`Some(timestamp)`** — Incremental-only from
//!   [`AddressProvider::last_sync_height`] (unless the elapsed time exceeds
//!   [`AddressSyncConfig::full_rescan_after_time_s`], in which case a full
//!   scan runs).
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
mod types;

pub use provider::AddressProvider;
pub use types::{
    AddressFunds, AddressIndex, AddressKey, AddressSyncConfig, AddressSyncMetrics,
    AddressSyncResult, LeafBoundaryKey,
};

use crate::error::Error;
use crate::platform::trunk_branch_sync::{
    self, BranchQueryParams, KeyLeafTracker, TrunkBranchSyncOps, TrunkQueryResponse,
};
use crate::platform::Fetch;
use crate::sync::retry;
use crate::Sdk;
use dapi_grpc::platform::v0::{
    get_addresses_branch_state_request, get_addresses_branch_state_response,
    get_recent_address_balance_changes_request,
    get_recent_compacted_address_balance_changes_request, GetAddressesBranchStateRequest,
    GetRecentAddressBalanceChangesRequest, GetRecentCompactedAddressBalanceChangesRequest, Proof,
};
use dpp::balances::credits::{BlockAwareCreditOperation, CreditOperation};
use dpp::prelude::AddressNonce;
use dpp::version::PlatformVersion;
use drive::drive::{Drive, RootTree};
use drive::grovedb::{Element, GroveBranchQueryResult, GroveTrunkQueryResult};
use drive_proof_verifier::types::{
    PlatformAddressTrunkState, RecentAddressBalanceChanges, RecentCompactedAddressBalanceChanges,
};
use rs_dapi_client::{
    DapiRequest, ExecutionError, ExecutionResponse, InnerInto, IntoInner, RequestSettings,
};
use std::collections::HashMap;
use tracing::{debug, trace};

/// Server limit for compacted address balance changes per request.
const COMPACTED_BATCH_LIMIT: usize = 25;

/// The subtree key for recent (per-block) address balances storage.
/// Mirrors `drive::drive::saved_block_transactions::queries::ADDRESS_BALANCES_KEY_U8`
/// which is gated behind the `server` feature.
const ADDRESS_BALANCES_KEY_U8: u8 = b'm';

// ── Context type for the shared algorithm ────────────────────────────

/// Mutable context carried through the trunk/branch tree scan for addresses.
///
/// This bundles the provider, the key-to-index lookup, and the result into a
/// single struct so it can serve as `TrunkBranchSyncOps::Context`.
struct AddressSyncContext<'a, P: AddressProvider> {
    provider: &'a mut P,
    key_to_index: &'a mut HashMap<AddressKey, AddressIndex>,
    result: &'a mut AddressSyncResult,
}

// SAFETY: P: AddressProvider is Send (required by trait bound), and HashMap/AddressSyncResult are Send.
unsafe impl<P: AddressProvider> Send for AddressSyncContext<'_, P> {}

// ── TrunkBranchSyncOps implementation ────────────────────────────────

/// Marker type for address sync operations, parameterized over the provider.
struct AddressOps<P>(std::marker::PhantomData<P>);

impl<P: AddressProvider> TrunkBranchSyncOps for AddressOps<P> {
    type Context<'a>
        = AddressSyncContext<'a, P>
    where
        P: 'a;

    /// Address branch queries need no extra config beyond what's in the
    /// standard parameters (key, depth, hash, checkpoint_height).
    type BranchQueryConfig = ();

    async fn execute_trunk_query(
        sdk: &Sdk,
        settings: RequestSettings,
        context: &mut Self::Context<'_>,
    ) -> Result<TrunkQueryResponse, Error> {
        let (trunk_state, metadata) =
            PlatformAddressTrunkState::fetch_with_metadata(sdk, (), Some(settings)).await?;

        context.result.metrics.trunk_queries += 1;

        let trunk_state = trunk_state.ok_or_else(|| {
            Error::InvalidProvedResponse("Trunk query returned no state".to_string())
        })?;

        context.result.metrics.total_elements_seen += trunk_state.elements.len();

        trace!(
            "Trunk query returned {} elements, {} leaf_keys",
            trunk_state.elements.len(),
            trunk_state.leaf_keys.len()
        );

        Ok(TrunkQueryResponse {
            trunk: trunk_state.into_inner(),
            height: metadata.height,
            block_time_ms: metadata.time_ms,
        })
    }

    fn process_trunk_result(
        trunk_result: &GroveTrunkQueryResult,
        context: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) -> Result<(), Error> {
        let pending: Vec<(AddressIndex, AddressKey)> = context.provider.pending_addresses();

        for (index, key) in pending {
            if let Some(element) = trunk_result.elements.get(&key) {
                let funds = AddressFunds::try_from(element)?;
                context.result.found.insert((index, key.clone()), funds);
                context.provider.on_address_found(index, &key, funds);
            } else if let Some((leaf_key, info)) = trunk_result.trace_key_to_leaf(&key) {
                tracker.add_key(key, leaf_key, info);
            } else {
                // Key is proven absent
                context.result.absent.insert((index, key.clone()));
                context.provider.on_address_absent(index, &key);
            }
        }

        Ok(())
    }

    fn branch_query_config(_context: &Self::Context<'_>) -> Self::BranchQueryConfig {}

    async fn execute_single_branch_query(
        sdk: &Sdk,
        _config: &Self::BranchQueryConfig,
        params: BranchQueryParams,
        settings: RequestSettings,
        platform_version: &PlatformVersion,
    ) -> Result<GroveBranchQueryResult, Error> {
        execute_address_branch_query(sdk, params, settings, platform_version).await
    }

    fn process_branch_result(
        branch_result: &GroveBranchQueryResult,
        queried_leaf_key: &[u8],
        context: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) -> Result<(), Error> {
        let target_keys = tracker.keys_for_leaf(queried_leaf_key);

        for target_key in target_keys {
            let index = context.key_to_index.get(&target_key).copied().unwrap_or(0);

            if let Some(element) = branch_result.elements.get(&target_key) {
                let funds = AddressFunds::try_from(element)?;
                context
                    .result
                    .found
                    .insert((index, target_key.clone()), funds);
                context.provider.on_address_found(index, &target_key, funds);
                tracker.key_found(&target_key);
            } else if let Some((new_leaf_key, info)) = branch_result.trace_key_to_leaf(&target_key)
            {
                tracker.update_leaf(&target_key, new_leaf_key, info);
            } else {
                // Key is proven absent
                context.result.absent.insert((index, target_key.clone()));
                context.provider.on_address_absent(index, &target_key);
                tracker.key_found(&target_key); // Remove from tracking
            }
        }
        Ok(())
    }

    fn depth_limits(platform_version: &PlatformVersion) -> (u8, u8) {
        (
            platform_version
                .drive
                .methods
                .address_funds
                .address_funds_query_min_depth,
            platform_version
                .drive
                .methods
                .address_funds
                .address_funds_query_max_depth,
        )
    }

    fn after_branch_iteration(
        trunk_result: &GroveTrunkQueryResult,
        context: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) {
        // Check if provider has extended pending addresses (gap limit behavior)
        for (index, key) in context.provider.pending_addresses() {
            if !context.key_to_index.contains_key(&key) {
                context.key_to_index.insert(key.clone(), index);
                // New key needs to be traced - it will be picked up in next iteration
                if let Some((leaf_key, info)) = trunk_result.trace_key_to_leaf(&key) {
                    tracker.add_key(key, leaf_key, info);
                }
            }
        }
    }

    fn on_branch_query(context: &mut Self::Context<'_>) {
        context.result.metrics.branch_queries += 1;
    }

    fn on_branch_failure(_context: &mut Self::Context<'_>) {
        // Address sync did not track branch failures in the original code.
    }

    fn on_elements_seen(context: &mut Self::Context<'_>, count: usize) {
        context.result.metrics.total_elements_seen += count;
    }

    fn on_iteration(context: &mut Self::Context<'_>, iteration: usize) {
        context.result.metrics.iterations = iteration;
    }

    fn set_checkpoint_height(context: &mut Self::Context<'_>, height: u64) {
        context.result.checkpoint_height = height;
    }
}

// ── Public API ───────────────────────────────────────────────────────

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
        // Full tree scan via the shared algorithm
        let mut context = AddressSyncContext {
            provider,
            key_to_index: &mut key_to_index,
            result: &mut result,
        };
        let (scan_height, block_time_ms) = trunk_branch_sync::run_full_tree_scan::<AddressOps<P>>(
            sdk,
            config.min_privacy_count,
            config.max_iterations,
            config.max_concurrent_requests,
            config.request_settings,
            &mut context,
        )
        .await?;
        // Seed timestamp from the trunk query (may be updated by incremental phase)
        result.new_sync_timestamp = block_time_ms / 1000;
        scan_height
    };

    // Incremental catch-up from catch_up_from to chain tip.
    // Queries recent first to get a proof, then checks if the boundary height
    // still exists in the recent tree. If it does, compacted is skipped.
    // If not (compaction detected), compacted is fetched first, then recent applied.
    let last_known_recent_block = provider.last_known_recent_block_height();
    incremental_catch_up(
        sdk,
        &key_to_index,
        catch_up_from,
        last_known_recent_block,
        provider,
        &mut result,
        config.request_settings,
    )
    .await?;

    // Set highest found index from provider
    result.highest_found_index = provider.highest_found_index();

    Ok(result)
}

// ── Incremental catch-up (address-specific) ──────────────────────────

/// Perform incremental block-based catch-up using recent + (optionally) compacted
/// address balance changes RPCs.
///
/// The function queries recent changes **first** to obtain a GroveDB proof, then
/// uses [`Drive::verify_key_exists_as_boundary`] to check whether the boundary
/// height still exists in the recent address balances tree. If it does, the
/// compacted phase is skipped entirely -- this is the common hot path for
/// frequent 15-second re-syncs where only the recent zone has changed.
///
/// When `last_known_recent_block > 0`, the recent query uses **exclusive start**
/// (`RangeAfter`) with that height. This causes the height to appear as a
/// **boundary node** in the proof, which `key_exists_as_boundary` can detect.
/// When `last_known_recent_block == 0`, the query falls back to inclusive start
/// (`RangeFrom` on `start_height`) and the boundary check is skipped (compacted
/// phase always runs).
///
/// - **Phase 1 (recent, always first)**: Single non-paginated query via
///   `fetch_with_metadata_and_proof`. The recent (uncompacted) zone covers at
///   most 64 blocks, well under the server limit of 100 entries per request.
///
/// - **Phase 2 (compacted, conditional)**: Paginated query (25-entry batches)
///   covering the compacted range. Only runs when the boundary check detects that
///   the cursor height was compacted away.
///
/// - **Phase 3 (apply recent)**: The recent results fetched in Phase 1 are applied
///   after the (optional) compacted phase.
///
/// Updates `result.found` with new balance values and sets `result.new_sync_height`
/// and `result.last_known_recent_block`.
async fn incremental_catch_up<P: AddressProvider>(
    sdk: &Sdk,
    key_to_index: &HashMap<AddressKey, AddressIndex>,
    start_height: u64,
    last_known_recent_block: u64,
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
    let mut observed_tip_height = start_height;

    // Phase 1 — Query recent changes first (with proof for compaction detection)
    //
    // When we have a last_known_recent_block from a previous sync, use
    // exclusive start (RangeAfter) so the boundary node appears in the proof.
    // Otherwise fall back to inclusive start (RangeFrom) on start_height.
    let use_exclusive_start = last_known_recent_block > 0;
    let recent_query_height = if use_exclusive_start {
        last_known_recent_block
    } else {
        start_height
    };

    let recent_request = GetRecentAddressBalanceChangesRequest {
        version: Some(get_recent_address_balance_changes_request::Version::V0(
            get_recent_address_balance_changes_request::GetRecentAddressBalanceChangesRequestV0 {
                start_height: recent_query_height,
                prove: true,
                start_height_exclusive: use_exclusive_start,
            },
        )),
    };

    let (recent_changes, recent_metadata, recent_proof) =
        match RecentAddressBalanceChanges::fetch_with_metadata_and_proof(
            sdk,
            recent_request,
            Some(settings),
        )
        .await
        {
            Ok(result) => result,
            Err(e) => {
                // First query failed — server may not support incremental
                // RPCs or may not have data for this height. Treat as
                // "no incremental data available" rather than hard error.
                debug!(
                    "Recent address balance changes query failed (non-fatal): {}",
                    e
                );
                result.new_sync_height = current_height.max(observed_tip_height);
                return Ok(());
            }
        };

    // The recent query succeeded — any subsequent query failures are real errors.
    let had_successful_query = true;

    result.new_sync_timestamp = recent_metadata.time_ms / 1000;
    result.metrics.recent_queries += 1;

    if recent_metadata.height > observed_tip_height {
        observed_tip_height = recent_metadata.height;
    }

    // Phase 2 — Determine whether compacted phase is needed
    //
    // When we used exclusive start (RangeAfter), the boundary height
    // (last_known_recent_block) appears as a boundary node in the proof.
    // We can check if it still exists to determine if compaction happened.
    //
    // When we used inclusive start (RangeFrom) or start_height == 0,
    // we cannot perform the boundary check — always run compacted.
    let need_compacted = if !use_exclusive_start || start_height == 0 {
        // No prior recent block or first incremental — always check compacted
        true
    } else {
        match check_compaction_from_proof(&recent_proof, last_known_recent_block, sdk.version()) {
            Ok(cursor_exists) => {
                if cursor_exists {
                    debug!(
                        "Address sync: last_known_recent_block {} exists as boundary in recent proof — skipping compacted phase",
                        last_known_recent_block
                    );
                    false
                } else {
                    debug!(
                        "Address sync: last_known_recent_block {} not found as boundary — running compacted phase",
                        last_known_recent_block
                    );
                    true
                }
            }
            Err(e) => {
                // On error, be conservative and query compacted
                debug!(
                    "Address sync: boundary check failed ({}), falling back to compacted phase",
                    e
                );
                true
            }
        }
    };

    // Phase 2b — Compacted (historical) catch-up (conditional)
    if need_compacted {
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
                        // First compacted query failed — treat as non-fatal.
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
            result.metrics.compacted_queries += 1;
            // Track the platform's chain tip from response metadata
            if metadata.height > observed_tip_height {
                observed_tip_height = metadata.height;
            }

            if entries.is_empty() {
                break;
            }

            let entry_count = entries.len();

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
    }

    // Phase 3 — Apply held recent results
    //
    // The recent results were fetched in Phase 1 but held back so that
    // compacted entries (if any) are applied first in correct height order.
    // We also track the highest block_height from recent entries to set
    // last_known_recent_block for the next sync.
    let mut highest_recent_block: u64 = 0;

    if let Some(changes) = recent_changes {
        let entries = changes.into_inner();

        for entry in &entries {
            // Track the highest block height in recent entries
            if entry.block_height > highest_recent_block {
                highest_recent_block = entry.block_height;
            }

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
    }

    result.new_sync_height = current_height.max(observed_tip_height);
    // Store the highest block from the recent entries so the next sync can
    // use RangeAfter(this_height) for compaction detection.
    // This MUST be a block that was actually in the recent tree entries —
    // not the metadata tip — because the boundary check needs the key to
    // exist in the tree. When the recent tree is empty (no address activity),
    // this stays at 0 and the next sync falls back to RangeFrom (inclusive).
    result.last_known_recent_block = highest_recent_block;
    Ok(())
}

/// Check whether a boundary height still exists as a boundary key in the
/// recent address balances tree by inspecting the GroveDB proof.
///
/// The `boundary_height` must have been used as the exclusive start of a
/// `RangeAfter` query, so it appears as a boundary node in the proof.
///
/// Returns `true` if the key exists as a boundary element (meaning it has NOT
/// been compacted away), `false` if it has been compacted away or was never
/// present.
fn check_compaction_from_proof(
    proof: &Proof,
    boundary_height: u64,
    platform_version: &PlatformVersion,
) -> Result<bool, Error> {
    let path: [&[u8]; 2] = [
        &[RootTree::SavedBlockTransactions as u8],
        &[ADDRESS_BALANCES_KEY_U8],
    ];

    Drive::verify_key_exists_as_boundary(
        &proof.grovedb_proof,
        &path,
        &boundary_height.to_be_bytes(),
        platform_version,
    )
    .map_err(Error::Drive)
}

// ── Branch query helper (address-specific) ───────────────────────────

/// Execute a single address branch query with retry logic.
///
/// If proof verification fails, the request will be retried with a different node
/// according to the retry settings.
async fn execute_address_branch_query(
    sdk: &Sdk,
    params: BranchQueryParams,
    settings: RequestSettings,
    platform_version: &PlatformVersion,
) -> Result<GroveBranchQueryResult, Error> {
    let BranchQueryParams {
        key,
        depth,
        expected_hash,
        checkpoint_height,
    } = params;

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

// ── Element conversion ───────────────────────────────────────────────

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

// ── SDK integration ──────────────────────────────────────────────────

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

    #[test]
    fn test_sync_mode_decision_no_timestamp() {
        // No timestamp → full scan needed
        let config = AddressSyncConfig::default();
        let last_sync_timestamp: Option<u64> = None;
        let needs_full_scan = match last_sync_timestamp {
            Some(ts) if config.full_rescan_after_time_s > 0 => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                let elapsed = now.saturating_sub(ts);
                elapsed >= config.full_rescan_after_time_s
            }
            _ => true,
        };
        assert!(needs_full_scan);
    }

    #[test]
    fn test_sync_mode_decision_recent_timestamp() {
        // Recent timestamp → incremental only
        let config = AddressSyncConfig::default();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let last_sync_timestamp = Some(now - 10); // 10 seconds ago
        let needs_full_scan = match last_sync_timestamp {
            Some(ts) if config.full_rescan_after_time_s > 0 => {
                let elapsed = now.saturating_sub(ts);
                elapsed >= config.full_rescan_after_time_s
            }
            _ => true,
        };
        assert!(!needs_full_scan);
    }

    #[test]
    fn test_sync_mode_decision_stale_timestamp() {
        // Stale timestamp (8 days old) → full scan
        let config = AddressSyncConfig::default();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let last_sync_timestamp = Some(now - 8 * 24 * 60 * 60); // 8 days ago
        let needs_full_scan = match last_sync_timestamp {
            Some(ts) if config.full_rescan_after_time_s > 0 => {
                let elapsed = now.saturating_sub(ts);
                elapsed >= config.full_rescan_after_time_s
            }
            _ => true,
        };
        assert!(needs_full_scan);
    }

    #[test]
    fn test_check_compaction_from_proof_empty_proof() {
        // Empty/invalid proof should return an error (conservative fallback)
        let proof = dapi_grpc::platform::v0::Proof {
            grovedb_proof: vec![],
            quorum_hash: vec![],
            signature: vec![],
            round: 0,
            block_id_hash: vec![],
            quorum_type: 0,
        };
        let platform_version = PlatformVersion::latest();
        let result = check_compaction_from_proof(&proof, 100, platform_version);
        // Empty proof should error — triggering conservative compacted query
        assert!(result.is_err());
    }

    #[test]
    fn test_check_compaction_from_proof_invalid_proof() {
        // Garbage bytes should return an error
        let proof = dapi_grpc::platform::v0::Proof {
            grovedb_proof: vec![0xFF, 0xFE, 0xFD, 0xFC],
            quorum_hash: vec![],
            signature: vec![],
            round: 0,
            block_id_hash: vec![],
            quorum_type: 0,
        };
        let platform_version = PlatformVersion::latest();
        let result = check_compaction_from_proof(&proof, 100, platform_version);
        assert!(result.is_err());
    }

    #[test]
    fn test_result_new_sync_height_max() {
        // new_sync_height should be max of current and observed tip
        let mut result = AddressSyncResult::new();
        result.new_sync_height = 100;
        let observed_tip = 200u64;
        result.new_sync_height = result.new_sync_height.max(observed_tip);
        assert_eq!(result.new_sync_height, 200);
    }

    #[test]
    fn test_result_checkpoint_separate_from_sync_height() {
        let mut result = AddressSyncResult::new();
        result.checkpoint_height = 50;
        result.new_sync_height = 100;
        assert_ne!(result.checkpoint_height, result.new_sync_height);
        assert_eq!(result.checkpoint_height, 50);
        assert_eq!(result.new_sync_height, 100);
    }
}
