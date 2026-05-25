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
    AddressFunds, AddressIndex, AddressSyncConfig, AddressSyncMetrics, AddressSyncResult,
    AddressToBytes, LeafBoundaryKey,
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
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::{BlockAwareCreditOperation, CreditOperation, Credits};
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
use tracing::{debug, info, trace, warn};

/// One-shot warning threshold for the end-of-pass replay buffer
/// (`pending_unknown`). Cross-checked at the push site below.
const PENDING_UNKNOWN_WARN_THRESHOLD: usize = 1000;

/// Server limit for compacted address balance changes per request.
const COMPACTED_BATCH_LIMIT: usize = 25;

/// The subtree key for recent (per-block) address balances storage.
/// Mirrors `drive::drive::saved_block_transactions::queries::ADDRESS_BALANCES_KEY_U8`
/// which is gated behind the `server` feature.
const ADDRESS_BALANCES_KEY_U8: u8 = b'm';

// ── Context type for the shared algorithm ────────────────────────────

/// Mutable context carried through the trunk/branch tree scan for addresses.
///
/// This bundles the provider, the key-to-tag lookup, and the result into a
/// single struct so it can serve as `TrunkBranchSyncOps::Context`.
struct AddressSyncContext<'a, P: AddressProvider> {
    provider: &'a mut P,
    /// Keys are raw GroveDB bytes (produced by `AddressToBytes::to_bytes`),
    /// not the provider's address type — that avoids hashing the address
    /// representation twice per lookup.
    key_to_tag: &'a mut HashMap<Vec<u8>, (P::Tag, P::Address)>,
    result: &'a mut AddressSyncResult<P::Tag, P::Address>,
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

    async fn process_trunk_result(
        trunk_result: &GroveTrunkQueryResult,
        context: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) -> Result<(), Error> {
        // Materialize into a Vec because the loop body calls
        // `context.provider.on_address_found` / `on_address_absent`,
        // which need to re-borrow `provider` mutably.
        let pending: Vec<(P::Tag, P::Address)> = context.provider.pending_addresses().collect();

        for (tag, address) in pending {
            let key_bytes = address.to_bytes();
            if let Some(element) = trunk_result.elements.get(&key_bytes) {
                let funds = AddressFunds::try_from(element)?;
                context.result.found.insert((tag, address), funds);
                context
                    .provider
                    .on_address_found(tag, &address, funds)
                    .await;
            } else if let Some((leaf_key, info)) = trunk_result.trace_key_to_leaf(&key_bytes) {
                tracker.add_key(key_bytes, leaf_key, info);
            } else {
                // Key is proven absent
                context.result.absent.insert((tag, address));
                context.provider.on_address_absent(tag, &address).await;
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

    async fn process_branch_result(
        branch_result: &GroveBranchQueryResult,
        queried_leaf_key: &[u8],
        context: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) -> Result<(), Error> {
        let target_keys = tracker.keys_for_leaf(queried_leaf_key);

        for target_key in target_keys {
            // target_key is the raw GroveDB key bytes. Look up the
            // provider's (tag, address) pair the engine stashed during
            // `process_trunk_result` / `after_branch_iteration`. If we
            // ever see bytes the engine didn't originally register,
            // skip rather than fabricate.
            let Some(&(tag, address)) = context.key_to_tag.get(target_key.as_slice()) else {
                tracker.key_found(&target_key);
                continue;
            };

            if let Some(element) = branch_result.elements.get(&target_key) {
                let funds = AddressFunds::try_from(element)?;
                context.result.found.insert((tag, address), funds);
                context
                    .provider
                    .on_address_found(tag, &address, funds)
                    .await;
                tracker.key_found(&target_key);
            } else if let Some((new_leaf_key, info)) = branch_result.trace_key_to_leaf(&target_key)
            {
                tracker.update_leaf(&target_key, new_leaf_key, info);
            } else {
                // Key is proven absent
                context.result.absent.insert((tag, address));
                context.provider.on_address_absent(tag, &address).await;
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

    async fn after_branch_iteration(
        trunk_result: &GroveTrunkQueryResult,
        context: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) {
        // Check if provider has extended pending addresses (gap limit behavior).
        // Materialize the iterator so the `&context.provider` borrow it
        // holds is released before we mutate `context.key_to_tag`.
        let pending: Vec<(P::Tag, P::Address)> = context.provider.pending_addresses().collect();
        for (tag, address) in pending {
            let key_bytes = address.to_bytes();
            let traced = trunk_result.trace_key_to_leaf(&key_bytes);
            if let std::collections::hash_map::Entry::Vacant(entry) =
                context.key_to_tag.entry(key_bytes.clone())
            {
                // New key needs to be traced — it will be picked up in
                // next iteration.
                if let Some((leaf_key, info)) = traced {
                    tracker.add_key(key_bytes, leaf_key, info);
                }
                entry.insert((tag, address));
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
) -> Result<AddressSyncResult<P::Tag, P::Address>, Error> {
    let config = config.unwrap_or_default();

    // Build the key -> (tag, address) map. Key is the raw GroveDB
    // bytes so branch processing can look up directly from `target_key`
    // without decoding back through `from_bytes`.
    let mut key_to_tag: HashMap<Vec<u8>, (P::Tag, P::Address)> = HashMap::new();
    for (tag, address) in provider.pending_addresses() {
        key_to_tag.insert(address.to_bytes(), (tag, address));
    }

    // Initialize result
    let mut result: AddressSyncResult<P::Tag, P::Address> = AddressSyncResult::new();

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
        for (tag, address, funds) in provider.current_balances() {
            result.found.insert((tag, address), funds);
        }
        start_height
    } else {
        // Full tree scan via the shared algorithm
        let mut context = AddressSyncContext {
            provider,
            key_to_tag: &mut key_to_tag,
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
        &key_to_tag,
        catch_up_from,
        last_known_recent_block,
        provider,
        &mut result,
        config.request_settings,
    )
    .await?;

    // Sync completed successfully — give the provider a chance to
    // commit any per-pass scratch state it accumulated in the
    // `on_address_found` / `on_address_absent` callbacks.
    provider.sync_finished().await;

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
    key_to_tag: &HashMap<Vec<u8>, (P::Tag, P::Address)>,
    start_height: u64,
    last_known_recent_block: u64,
    provider: &mut P,
    result: &mut AddressSyncResult<P::Tag, P::Address>,
    settings: RequestSettings,
) -> Result<(), Error> {
    // Use the borrowed `key_to_tag` directly through the pass — only
    // unknown-address replay (rare, end-of-pass) materializes any extra
    // allocation. Buffered misses are bounded by the count of foreign /
    // post-snapshot addresses in the response.
    let mut pending_unknown: Vec<PendingUnknownChange> = Vec::new();

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

    // Store the raw GroveDB proof bytes for debugging/inspection.
    result.recent_proof = recent_proof.grovedb_proof.clone();

    result.new_sync_timestamp = recent_metadata.time_ms / 1000;
    result.metrics.recent_queries += 1;

    // Log what the recent query returned
    let recent_entry_count = recent_changes.as_ref().map(|c| c.0.len()).unwrap_or(0);
    info!(
        "Address sync: recent query returned {} entries (use_exclusive={}, query_height={}, metadata_height={})",
        recent_entry_count, use_exclusive_start, recent_query_height, recent_metadata.height
    );

    if recent_metadata.height > observed_tip_height {
        observed_tip_height = recent_metadata.height;
    }

    // Phase 2 — Determine whether compacted phase is needed
    //
    // Based on three values:
    // 1. checkpoint_height: the tree scan snapshot height
    // 2. last_recent_block: highest boundary in the recent proof (the actual
    //    last block in the recent tree)
    // 3. Whether boundaries exist at all
    //
    // If last_recent_block >= checkpoint_height → recent covers the full range,
    //   no compacted data can exist in the gap. Skip compacted.
    // If last_recent_block < checkpoint_height → there's a gap that may contain
    //   compacted data. Query compacted.
    // If no boundaries → recent tree is empty, nothing to compact. Skip.
    let need_compacted = match get_last_recent_block_from_proof(&recent_proof) {
        Ok(Some(last_recent_block)) => {
            if last_recent_block >= result.checkpoint_height {
                debug!(
                    "Address sync: last recent block {} >= checkpoint {} — skipping compacted",
                    last_recent_block, result.checkpoint_height
                );
                false
            } else {
                debug!(
                    "Address sync: last recent block {} < checkpoint {} — need compacted",
                    last_recent_block, result.checkpoint_height
                );
                true
            }
        }
        Ok(None) => {
            // No boundaries in recent tree → empty, nothing to compact
            debug!("Address sync: recent tree empty (no boundaries) — skipping compacted");
            false
        }
        Err(e) => {
            debug!(
                "Address sync: boundary extraction failed ({}), running compacted to be safe",
                e
            );
            true
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
            result.metrics.compacted_entries_returned += entry_count;

            for entry in &entries {
                apply_block_changes(
                    key_to_tag,
                    entry
                        .changes
                        .iter()
                        .map(|(a, op)| (a, AddressBalanceChange::Compacted(op))),
                    current_height,
                    provider,
                    result,
                    &mut pending_unknown,
                )
                .await;

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
        result.metrics.recent_entries_returned += entries.len();

        for entry in &entries {
            // Track the highest block height in recent entries
            if entry.block_height > highest_recent_block {
                highest_recent_block = entry.block_height;
            }

            apply_block_changes(
                key_to_tag,
                entry
                    .changes
                    .iter()
                    .map(|(a, op)| (a, AddressBalanceChange::Recent(op))),
                current_height,
                provider,
                result,
                &mut pending_unknown,
            )
            .await;

            if entry.block_height.saturating_add(1) > current_height {
                current_height = entry.block_height.saturating_add(1);
            }
        }
    }

    // Single end-of-pass recovery: foreign-wallet addresses fall out at
    // the extras-intersection check, so no per-block refresh and no log
    // flood on multi-wallet chains.
    refresh_and_replay_unknown(key_to_tag, pending_unknown, provider, result).await;

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

// ── Address-balance change application ────────────────────────────────

/// A single address balance change, abstracting the recent (`CreditOperation`)
/// and compacted (`BlockAwareCreditOperation`) shapes so one pure function can
/// apply both phases identically.
#[derive(Clone, Copy)]
pub(crate) enum AddressBalanceChange<'a> {
    /// A recent (per-block) credit operation.
    Recent(&'a CreditOperation),
    /// A compacted (block-range) credit operation.
    Compacted(&'a BlockAwareCreditOperation),
}

impl AddressBalanceChange<'_> {
    /// Resolve the post-change balance given the address's current balance and
    /// the catch-up cursor height. Mirrors the two original inline loops
    /// exactly (compacted height-filtered sum vs. recent flat add).
    fn new_balance(&self, current_balance: Credits, current_height: u64) -> Credits {
        match self {
            AddressBalanceChange::Recent(op) => match op {
                CreditOperation::SetCredits(credits) => *credits,
                CreditOperation::AddToCredits(credits) => current_balance.saturating_add(*credits),
            },
            AddressBalanceChange::Compacted(op) => match op {
                BlockAwareCreditOperation::SetCredits(credits) => *credits,
                BlockAwareCreditOperation::AddToCreditsOperations(operations) => {
                    let total_to_add: u64 = operations
                        .iter()
                        .filter(|(height, _)| **height >= current_height)
                        .map(|(_, credits)| *credits)
                        .fold(0u64, |acc, c| acc.saturating_add(c));
                    current_balance.saturating_add(total_to_add)
                }
            },
        }
    }

    /// Owned snapshot of the change for end-of-pass replay. Cheap for
    /// `Recent` (the inner op is `Copy`); clones the operations vector for
    /// `Compacted`. Only called for unknown addresses.
    fn into_owned(self) -> OwnedAddressBalanceChange {
        match self {
            AddressBalanceChange::Recent(op) => OwnedAddressBalanceChange::Recent(*op),
            AddressBalanceChange::Compacted(op) => OwnedAddressBalanceChange::Compacted(op.clone()),
        }
    }
}

/// Owned counterpart of [`AddressBalanceChange`] so unknown-address changes
/// can outlive the per-block iterator and be replayed at end-of-pass.
#[derive(Clone)]
pub(crate) enum OwnedAddressBalanceChange {
    Recent(CreditOperation),
    Compacted(BlockAwareCreditOperation),
}

impl OwnedAddressBalanceChange {
    fn as_borrowed(&self) -> AddressBalanceChange<'_> {
        match self {
            OwnedAddressBalanceChange::Recent(op) => AddressBalanceChange::Recent(op),
            OwnedAddressBalanceChange::Compacted(op) => AddressBalanceChange::Compacted(op),
        }
    }
}

/// A single change for an address that wasn't in the entry-time snapshot.
/// Buffered across the catch-up pass and replayed once at the end after a
/// single `pending_addresses()` refresh.
pub(crate) struct PendingUnknownChange {
    /// Raw GroveDB key bytes — joined against the refreshed lookup.
    key: Vec<u8>,
    /// Owned change so the underlying response entries can be dropped.
    change: OwnedAddressBalanceChange,
    /// Catch-up cursor at the time of the original block — feeds the
    /// compacted height filter on replay. Ignored by `Recent`.
    current_height: u64,
}

/// Apply one block's changes against the borrowed entry-time lookup, drive
/// `on_address_found` for every known address whose balance moved, and
/// append unknown-address changes to `pending_unknown` for a single
/// end-of-pass refresh + replay. The refresh is deliberately deferred so
/// foreign-wallet addresses on a shared chain do not trigger a per-block
/// provider poll.
async fn apply_block_changes<'a, P, I>(
    address_lookup: &HashMap<Vec<u8>, (P::Tag, P::Address)>,
    changes: I,
    current_height: u64,
    provider: &mut P,
    result: &mut AddressSyncResult<P::Tag, P::Address>,
    pending_unknown: &mut Vec<PendingUnknownChange>,
) where
    P: AddressProvider,
    I: IntoIterator<Item = (&'a PlatformAddress, AddressBalanceChange<'a>)>,
{
    let mut local_applied: Vec<(P::Tag, P::Address, AddressFunds)> = Vec::new();

    for (platform_addr, change) in changes {
        let addr_bytes = platform_addr.to_bytes();
        if let Some(&(tag, address)) = address_lookup.get(&addr_bytes) {
            let result_key = (tag, address);
            let current_balance = result
                .found
                .get(&result_key)
                .map(|f| f.balance)
                .unwrap_or(0);

            let new_balance = change.new_balance(current_balance, current_height);

            if new_balance != current_balance {
                // TODO: incremental RPCs carry only balance deltas, never
                // nonces — addresses first seen here get nonce=0. Clients
                // recover via `AddressInvalidNonceError.expected_nonce`;
                // a proper fix would fetch authoritative `AddressFunds`
                // or model `nonce` as `Option<u32>`.
                let nonce = result.found.get(&result_key).map(|f| f.nonce).unwrap_or(0);
                let funds = AddressFunds {
                    nonce,
                    balance: new_balance,
                };
                result.absent.remove(&result_key);
                result.found.insert(result_key, funds);
                local_applied.push((tag, address, funds));
            }
        } else {
            pending_unknown.push(PendingUnknownChange {
                key: addr_bytes,
                change: change.into_owned(),
                current_height,
            });
            // NOTE: this buffer is intentionally unbounded — premature optimization here
            // would couple the catch-up loop to ad-hoc memory heuristics. We log a
            // one-shot warning above a generous threshold so a future operator can
            // observe whether this path actually exceeds 1000 buffered foreign-wallet
            // changes in real workloads; if it does, the right fix is to follow the
            // reviewer's mitigation (a) — store only Vec<u8> keys and re-derive replay
            // changes after the refresh resolves them. See PR #3650 @thepastaclaw review.
            if pending_unknown.len() == PENDING_UNKNOWN_WARN_THRESHOLD {
                warn!(
                    "Address sync: pending_unknown buffer reached {} entries — \
                     foreign-wallet balance changes are accumulating on a shared chain",
                    PENDING_UNKNOWN_WARN_THRESHOLD
                );
            }
        }
    }

    for (tag, address, funds) in &local_applied {
        provider.on_address_found(*tag, address, *funds).await;
    }
}

/// End-of-pass recovery for addresses missing from the entry-time
/// snapshot. Re-polls `pending_addresses()` exactly once, builds a small
/// `extras` map of newly-derived addresses, and replays only the buffered
/// changes that match an `extras` entry. Foreign (other-wallet) addresses
/// fall out at the intersection check — no provider refresh storm, no
/// log flood.
async fn refresh_and_replay_unknown<P: AddressProvider>(
    key_to_tag: &HashMap<Vec<u8>, (P::Tag, P::Address)>,
    pending_unknown: Vec<PendingUnknownChange>,
    provider: &mut P,
    result: &mut AddressSyncResult<P::Tag, P::Address>,
) {
    if pending_unknown.is_empty() {
        return;
    }

    // Build the set of unknown keys for a fast intersection probe.
    let unknown_keys: std::collections::HashSet<&[u8]> =
        pending_unknown.iter().map(|p| p.key.as_slice()).collect();

    // Only addresses the provider can now produce AND that match a
    // buffered miss are interesting — everything else is some other
    // wallet's address and stays out of the lookup entirely.
    let mut extras: HashMap<Vec<u8>, (P::Tag, P::Address)> = HashMap::new();
    for (tag, address) in provider.pending_addresses() {
        let bytes = address.to_bytes();
        if unknown_keys.contains(bytes.as_slice()) && !key_to_tag.contains_key(&bytes) {
            extras.insert(bytes, (tag, address));
        }
    }

    if extras.is_empty() {
        // Common case on a populated multi-wallet chain: every buffered
        // unknown belongs to another wallet.
        debug!(
            "Address sync: {} platform-reported balance change(s) reference \
             address(es) not tracked by this wallet; ignoring",
            pending_unknown.len()
        );
        return;
    }

    // Replay only the entries whose key actually resolves in `extras`.
    // Order is preserved (compacted first, then recent — same as the
    // forward pass), so `AddToCredits` deltas accumulate correctly. The
    // catch-up cursor per change is preserved so the compacted height
    // filter still sees the same `current_height` it would have seen on
    // the forward pass.
    let mut replay_applied: Vec<(P::Tag, P::Address, AddressFunds)> = Vec::new();
    let mut still_unknown: usize = 0;
    for pending in &pending_unknown {
        let Some(&(tag, address)) = extras.get(pending.key.as_slice()) else {
            still_unknown += 1;
            continue;
        };
        let result_key = (tag, address);
        let current_balance = result
            .found
            .get(&result_key)
            .map(|f| f.balance)
            .unwrap_or(0);
        let new_balance = pending
            .change
            .as_borrowed()
            .new_balance(current_balance, pending.current_height);

        if new_balance != current_balance {
            // TODO: same synthesized nonce=0 gap as the forward pass.
            let nonce = result.found.get(&result_key).map(|f| f.nonce).unwrap_or(0);
            let funds = AddressFunds {
                nonce,
                balance: new_balance,
            };
            result.absent.remove(&result_key);
            result.found.insert(result_key, funds);
            replay_applied.push((tag, address, funds));
        }
    }

    for (tag, address, funds) in &replay_applied {
        provider.on_address_found(*tag, address, *funds).await;
    }

    if still_unknown > 0 {
        debug!(
            "Address sync: {} platform-reported balance change(s) reference \
             address(es) not tracked by this wallet (refresh recovered {} \
             other(s)); ignoring the untracked entries",
            still_unknown,
            replay_applied.len()
        );
    }
}

/// Extract the highest block height from the recent tree boundaries in the proof.
///
/// Returns:
/// - `Ok(Some(height))` — the highest block height found as a boundary
/// - `Ok(None)` — no boundaries found (recent tree is empty)
/// - `Err(...)` — proof decoding failed
fn get_last_recent_block_from_proof(proof: &Proof) -> Result<Option<u64>, Error> {
    let path: [&[u8]; 2] = [
        &[RootTree::SavedBlockTransactions as u8],
        &[ADDRESS_BALANCES_KEY_U8],
    ];

    let config = dpp::bincode::config::standard()
        .with_big_endian()
        .with_limit::<{ 256 * 1024 * 1024 }>();

    let (grovedb_proof, _): (drive::grovedb::operations::proof::GroveDBProof, usize) =
        dpp::bincode::decode_from_slice(&proof.grovedb_proof, config).map_err(|e| {
            Error::Protocol(dpp::ProtocolError::DecodingError(format!(
                "Failed to decode GroveDB proof: {}",
                e
            )))
        })?;

    let all_boundaries = grovedb_proof
        .boundaries(&[path[0], path[1]])
        .map_err(|e| Error::Drive(drive::error::Error::GroveDB(Box::new(e))))?;

    if all_boundaries.is_empty() {
        return Ok(None);
    }

    // Parse boundary keys as block heights (big-endian u64)
    let max_height = all_boundaries
        .iter()
        .filter_map(|key| {
            if key.len() == 8 {
                Some(u64::from_be_bytes(key.as_slice().try_into().unwrap()))
            } else {
                None
            }
        })
        .max();

    Ok(max_height)
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
    ) -> Result<AddressSyncResult<P::Tag, P::Address>, Error> {
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
        assert_eq!(
            config.full_rescan_after_time_s,
            6 * 24 * 3600 + 23 * 3600 + 45 * 60
        );
        assert_eq!(config.min_privacy_count, 32);
        assert_eq!(config.max_iterations, 50);
    }

    #[test]
    fn test_default_result_has_zero_new_sync_height() {
        let result: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::new();
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
    fn test_get_last_recent_block_empty_proof() {
        // Empty proof should return an error (conservative fallback)
        let proof = dapi_grpc::platform::v0::Proof {
            grovedb_proof: vec![],
            quorum_hash: vec![],
            signature: vec![],
            round: 0,
            block_id_hash: vec![],
            quorum_type: 0,
        };
        let result = get_last_recent_block_from_proof(&proof);
        // Empty proof should error — triggering conservative compacted query
        assert!(result.is_err());
    }

    #[test]
    fn test_get_last_recent_block_invalid_proof() {
        // Garbage bytes should return an error
        let proof = dapi_grpc::platform::v0::Proof {
            grovedb_proof: vec![0xFF, 0xFE, 0xFD, 0xFC],
            quorum_hash: vec![],
            signature: vec![],
            round: 0,
            block_id_hash: vec![],
            quorum_type: 0,
        };
        let result = get_last_recent_block_from_proof(&proof);
        assert!(result.is_err());
    }

    #[test]
    fn test_result_new_sync_height_max() {
        // new_sync_height should be max of current and observed tip
        let mut result: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::new();
        result.new_sync_height = 100;
        let observed_tip = 200u64;
        result.new_sync_height = result.new_sync_height.max(observed_tip);
        assert_eq!(result.new_sync_height, 200);
    }

    #[test]
    fn test_result_checkpoint_separate_from_sync_height() {
        let mut result: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::new();
        result.checkpoint_height = 50;
        result.new_sync_height = 100;
        assert_ne!(result.checkpoint_height, result.new_sync_height);
        assert_eq!(result.checkpoint_height, 50);
        assert_eq!(result.new_sync_height, 100);
    }

    #[test]
    fn test_incremental_mode_checkpoint_zero_skips_compacted() {
        // In incremental-only mode, checkpoint_height defaults to 0.
        // Any last_recent_block >= 0 should skip compacted (correct behavior:
        // known balances are seeded from current_balances, so no compacted gap).
        let result: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::new();
        assert_eq!(result.checkpoint_height, 0);

        // Simulating the compaction detection logic from incremental_catch_up
        let last_recent_block: u64 = 500;
        let need_compacted = last_recent_block < result.checkpoint_height;
        assert!(
            !need_compacted,
            "Incremental-only mode should skip compacted when checkpoint is 0"
        );
    }

    #[test]
    fn test_full_scan_mode_checkpoint_triggers_compacted() {
        // After a full tree scan, checkpoint_height is set to the scan height.
        // If last_recent_block < checkpoint, compacted phase should run.
        let mut result: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::new();
        result.checkpoint_height = 1000;

        let last_recent_block: u64 = 500;
        let need_compacted = last_recent_block < result.checkpoint_height;
        assert!(
            need_compacted,
            "Full scan mode should run compacted when recent is behind checkpoint"
        );
    }

    #[test]
    fn test_full_scan_mode_recent_covers_checkpoint() {
        // After a full tree scan, if recent tree covers the checkpoint, skip compacted.
        let mut result: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::new();
        result.checkpoint_height = 1000;

        let last_recent_block: u64 = 1050;
        let need_compacted = last_recent_block < result.checkpoint_height;
        assert!(
            !need_compacted,
            "Should skip compacted when recent covers checkpoint"
        );
    }

    #[test]
    fn test_address_funds_from_item_with_sum_item() {
        // Valid: nonce=5 (4 bytes big-endian) with balance=1000
        let elem = Element::ItemWithSumItem(vec![0, 0, 0, 5], 1000, None);
        let funds = AddressFunds::try_from(&elem).expect("should parse valid element");
        assert_eq!(funds.nonce, 5);
        assert_eq!(funds.balance, 1000);

        // Invalid: nonce bytes too short (only 2 bytes instead of 4)
        let short_nonce = Element::ItemWithSumItem(vec![0, 5], 500, None);
        let err = AddressFunds::try_from(&short_nonce).unwrap_err();
        assert!(
            matches!(err, Error::InvalidProvedResponse(ref msg) if msg.contains("4 bytes")),
            "expected nonce length error, got: {err:?}"
        );

        // Invalid: nonce bytes too long (5 bytes instead of 4)
        let long_nonce = Element::ItemWithSumItem(vec![0, 0, 0, 0, 1], 100, None);
        let err = AddressFunds::try_from(&long_nonce).unwrap_err();
        assert!(
            matches!(err, Error::InvalidProvedResponse(ref msg) if msg.contains("4 bytes")),
            "expected nonce length error, got: {err:?}"
        );

        // Invalid: empty nonce bytes
        let empty_nonce = Element::ItemWithSumItem(vec![], 0, None);
        let err = AddressFunds::try_from(&empty_nonce).unwrap_err();
        assert!(
            matches!(err, Error::InvalidProvedResponse(ref msg) if msg.contains("4 bytes")),
            "expected nonce length error, got: {err:?}"
        );

        // Wrong variant: Item should fail
        let item = Element::Item(vec![1, 2, 3], None);
        let err = AddressFunds::try_from(&item).unwrap_err();
        assert!(
            matches!(err, Error::InvalidProvedResponse(ref msg) if msg.contains("unexpected element type")),
            "expected element type error for Item, got: {err:?}"
        );

        // Wrong variant: Tree should fail
        let tree = Element::empty_tree();
        let err = AddressFunds::try_from(&tree).unwrap_err();
        assert!(
            matches!(err, Error::InvalidProvedResponse(ref msg) if msg.contains("unexpected element type")),
            "expected element type error for Tree, got: {err:?}"
        );
    }

    #[test]
    fn test_address_funds_zero_values() {
        // Zero nonce and zero balance via ItemWithSumItem
        let elem = Element::ItemWithSumItem(vec![0, 0, 0, 0], 0, None);
        let funds = AddressFunds::try_from(&elem).expect("should parse zero-value element");
        assert_eq!(funds.nonce, 0);
        assert_eq!(funds.balance, 0);
    }

    #[test]
    fn test_sync_result_new_defaults() {
        let result: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::new();
        assert!(result.found.is_empty());
        assert!(result.absent.is_empty());
        assert_eq!(result.checkpoint_height, 0);
        assert_eq!(result.new_sync_height, 0);
        assert_eq!(result.new_sync_timestamp, 0);
        assert_eq!(result.last_known_recent_block, 0);
        assert!(result.recent_proof.is_empty());
        assert_eq!(result.total_balance(), 0);
        assert_eq!(result.non_zero_count(), 0);
    }

    #[test]
    fn test_sync_result_default_matches_new() {
        let from_new: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::new();
        let from_default: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::default();
        assert_eq!(from_new.found.len(), from_default.found.len());
        assert_eq!(from_new.absent.len(), from_default.absent.len());
        assert_eq!(from_new.checkpoint_height, from_default.checkpoint_height);
        assert_eq!(from_new.new_sync_height, from_default.new_sync_height);
        assert_eq!(from_new.new_sync_timestamp, from_default.new_sync_timestamp);
        assert_eq!(
            from_new.last_known_recent_block,
            from_default.last_known_recent_block
        );
    }

    #[test]
    fn test_metrics_default_all_zero() {
        let m = AddressSyncMetrics::default();
        assert_eq!(m.trunk_queries, 0);
        assert_eq!(m.branch_queries, 0);
        assert_eq!(m.total_elements_seen, 0);
        assert_eq!(m.total_proof_bytes, 0);
        assert_eq!(m.iterations, 0);
        assert_eq!(m.compacted_queries, 0);
        assert_eq!(m.recent_queries, 0);
        assert_eq!(m.recent_entries_returned, 0);
        assert_eq!(m.compacted_entries_returned, 0);
        assert_eq!(m.total_queries(), 0);
        assert_eq!(m.average_proof_bytes(), 0.0);
    }

    #[test]
    fn test_metrics_total_queries_sum() {
        let m = AddressSyncMetrics {
            trunk_queries: 2,
            branch_queries: 5,
            compacted_queries: 3,
            recent_queries: 4,
            total_elements_seen: 100,
            total_proof_bytes: 5000,
            iterations: 10,
            recent_entries_returned: 20,
            compacted_entries_returned: 30,
        };
        assert_eq!(m.total_queries(), 2 + 5 + 3 + 4);
    }

    #[test]
    fn test_metrics_average_proof_bytes() {
        let m = AddressSyncMetrics {
            trunk_queries: 1,
            branch_queries: 1,
            compacted_queries: 1,
            recent_queries: 1,
            total_proof_bytes: 4000,
            ..Default::default()
        };
        // total_queries = 4, so average = 4000/4 = 1000.0
        assert_eq!(m.total_queries(), 4);
        assert!((m.average_proof_bytes() - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_metrics_average_proof_bytes_zero_queries() {
        let m = AddressSyncMetrics::default();
        assert_eq!(m.average_proof_bytes(), 0.0);
    }

    #[test]
    fn test_config_custom_values() {
        let config = AddressSyncConfig {
            min_privacy_count: 64,
            max_concurrent_requests: 20,
            max_iterations: 100,
            full_rescan_after_time_s: 3 * 24 * 3600, // 3 days
            request_settings: RequestSettings::default(),
        };
        assert_eq!(config.min_privacy_count, 64);
        assert_eq!(config.max_concurrent_requests, 20);
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.full_rescan_after_time_s, 259200);
    }

    #[test]
    fn test_config_full_rescan_zero_always_rescans() {
        let config = AddressSyncConfig {
            full_rescan_after_time_s: 0,
            ..Default::default()
        };
        // With full_rescan_after_time_s = 0, any timestamp should trigger full rescan
        // because elapsed >= 0 is always true
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let last_sync_timestamp = Some(now); // just synced
        let needs_full_scan = match last_sync_timestamp {
            Some(ts) if config.full_rescan_after_time_s > 0 => {
                let elapsed = now.saturating_sub(ts);
                elapsed >= config.full_rescan_after_time_s
            }
            Some(_) => {
                // full_rescan_after_time_s == 0 means always rescan
                config.full_rescan_after_time_s == 0
            }
            None => true,
        };
        assert!(needs_full_scan);
    }

    #[test]
    fn test_sync_result_total_balance_and_non_zero_count() {
        let mut result: AddressSyncResult<AddressIndex, dpp::address_funds::PlatformAddress> =
            AddressSyncResult::new();

        // Insert three addresses with varying balances
        result.found.insert(
            (0, dpp::address_funds::PlatformAddress::P2pkh([1; 20])),
            AddressFunds {
                nonce: 0,
                balance: 500,
            },
        );
        result.found.insert(
            (1, dpp::address_funds::PlatformAddress::P2pkh([2; 20])),
            AddressFunds {
                nonce: 1,
                balance: 0,
            },
        );
        result.found.insert(
            (2, dpp::address_funds::PlatformAddress::P2pkh([3; 20])),
            AddressFunds {
                nonce: 2,
                balance: 1500,
            },
        );

        assert_eq!(result.total_balance(), 2000);
        assert_eq!(result.non_zero_count(), 2);
    }

    #[test]
    fn test_address_funds_max_nonce_and_balance() {
        // Maximum valid nonce (u32::MAX) and large balance
        let max_nonce_bytes = u32::MAX.to_be_bytes().to_vec();
        let elem = Element::ItemWithSumItem(max_nonce_bytes, i64::MAX, None);
        let funds = AddressFunds::try_from(&elem).expect("should parse max values");
        assert_eq!(funds.nonce, u32::MAX);
        assert_eq!(funds.balance, i64::MAX as u64);
    }

    #[test]
    fn test_address_funds_negative_balance_errors() {
        // Negative sum_item value should fail conversion to u64
        let elem = Element::ItemWithSumItem(vec![0, 0, 0, 1], -100, None);
        let err = AddressFunds::try_from(&elem).unwrap_err();
        assert!(
            matches!(err, Error::InvalidProvedResponse(ref msg) if msg.contains("balance")),
            "expected balance conversion error, got: {err:?}"
        );
    }

    // ── End-of-pass refresh + replay regression guards ─────────────────

    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::BlockAwareCreditOperation;

    fn p2pkh(byte: u8) -> PlatformAddress {
        PlatformAddress::P2pkh([byte; 20])
    }

    /// A provider that derives a fresh address mid-pass — so the
    /// entry-time lookup misses it — gets the balance applied AND
    /// `on_address_found` fired after the end-of-pass refresh.
    #[tokio::test]
    async fn apply_block_changes_recovers_post_snapshot_address() {
        use async_trait::async_trait;

        struct GrowingProvider {
            late: PlatformAddress,
            found: Vec<(u32, PlatformAddress, AddressFunds)>,
        }

        #[async_trait]
        impl AddressProvider for GrowingProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                std::iter::once((7u32, self.late))
            }

            async fn on_address_found(
                &mut self,
                tag: Self::Tag,
                address: &Self::Address,
                funds: AddressFunds,
            ) {
                self.found.push((tag, *address, funds));
            }

            async fn on_address_absent(&mut self, _tag: Self::Tag, _address: &Self::Address) {}

            fn current_balances(
                &self,
            ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
                std::iter::empty()
            }
        }

        let late = p2pkh(0xCD);

        let lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();

        let mut provider = GrowingProvider {
            late,
            found: Vec::new(),
        };
        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        let mut pending_unknown: Vec<PendingUnknownChange> = Vec::new();

        let op = BlockAwareCreditOperation::SetCredits(42_000);
        let changes = [(&late, AddressBalanceChange::Compacted(&op))];

        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            0,
            &mut provider,
            &mut result,
            &mut pending_unknown,
        )
        .await;

        // Per-block apply must NOT touch the provider for unknowns —
        // the refresh is deferred to end-of-pass.
        assert!(
            provider.found.is_empty(),
            "no on_address_found before end-of-pass refresh"
        );
        assert_eq!(pending_unknown.len(), 1, "miss is buffered for replay");

        refresh_and_replay_unknown(&lookup, pending_unknown, &mut provider, &mut result).await;

        assert_eq!(
            result.found.get(&(7u32, late)).map(|f| f.balance),
            Some(42_000),
            "post-snapshot address balance must be applied after refresh"
        );
        assert!(
            provider
                .found
                .iter()
                .any(|(t, a, f)| *t == 7 && *a == late && f.balance == 42_000),
            "on_address_found must fire for the recovered post-snapshot address"
        );
    }

    /// A known address proven absent by the tree scan but re-discovered
    /// by an incremental change is moved into `found` and pruned from
    /// `absent`, keeping the two sets disjoint.
    #[tokio::test]
    async fn apply_block_changes_keeps_found_and_absent_disjoint_on_catch_up() {
        use async_trait::async_trait;

        struct NoopProvider;

        #[async_trait]
        impl AddressProvider for NoopProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                std::iter::empty()
            }

            async fn on_address_found(
                &mut self,
                _tag: Self::Tag,
                _address: &Self::Address,
                _funds: AddressFunds,
            ) {
            }

            async fn on_address_absent(&mut self, _tag: Self::Tag, _address: &Self::Address) {}

            fn current_balances(
                &self,
            ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
                std::iter::empty()
            }
        }

        let tag: u32 = 5;
        let addr = p2pkh(0x99);

        let mut lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();
        lookup.insert(addr.to_bytes(), (tag, addr));

        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        result.absent.insert((tag, addr));

        let op = BlockAwareCreditOperation::SetCredits(7_777);
        let changes = [(&addr, AddressBalanceChange::Compacted(&op))];

        let mut pending_unknown: Vec<PendingUnknownChange> = Vec::new();
        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            0,
            &mut NoopProvider,
            &mut result,
            &mut pending_unknown,
        )
        .await;

        assert_eq!(
            result.found.get(&(tag, addr)).map(|f| f.balance),
            Some(7_777),
        );
        assert!(
            !result.absent.contains(&(tag, addr)),
            "apply_block_changes must keep found/absent disjoint"
        );
        assert!(
            pending_unknown.is_empty(),
            "no unknowns expected for a known address"
        );
    }

    /// The end-of-pass refresh must not double-count a known address's
    /// `AddToCredits` delta when it replays the unknown subset in the
    /// same block (the replay must exclude already-applied addresses).
    #[tokio::test]
    async fn refresh_does_not_double_count_known_address_delta() {
        use async_trait::async_trait;

        let known = p2pkh(0x11);
        let late = p2pkh(0x22);

        struct GrowingProvider {
            late: PlatformAddress,
        }

        #[async_trait]
        impl AddressProvider for GrowingProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                std::iter::once((9u32, self.late))
            }

            async fn on_address_found(
                &mut self,
                _tag: Self::Tag,
                _address: &Self::Address,
                _funds: AddressFunds,
            ) {
            }

            async fn on_address_absent(&mut self, _tag: Self::Tag, _address: &Self::Address) {}

            fn current_balances(
                &self,
            ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
                std::iter::empty()
            }
        }

        // `known` is in the snapshot with a starting balance; `late` is
        // not (post-snapshot) and forces the refresh + replay path.
        let mut lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();
        lookup.insert(known.to_bytes(), (3u32, known));

        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        result.found.insert(
            (3u32, known),
            AddressFunds {
                nonce: 0,
                balance: 1_000,
            },
        );

        let mut provider = GrowingProvider { late };

        let known_op = BlockAwareCreditOperation::AddToCreditsOperations(
            std::iter::once((0u64, 500u64)).collect(),
        );
        let late_op = BlockAwareCreditOperation::SetCredits(7_000);
        let changes = [
            (&known, AddressBalanceChange::Compacted(&known_op)),
            (&late, AddressBalanceChange::Compacted(&late_op)),
        ];

        let mut pending_unknown: Vec<PendingUnknownChange> = Vec::new();
        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            0,
            &mut provider,
            &mut result,
            &mut pending_unknown,
        )
        .await;
        // Known address was applied immediately (no longer waits on the
        // end-of-pass refresh). `late` is buffered for replay.
        assert_eq!(pending_unknown.len(), 1);

        refresh_and_replay_unknown(&lookup, pending_unknown, &mut provider, &mut result).await;

        // Known delta applied exactly once: 1000 + 500 (NOT 1000 + 500 +
        // 500). The replay must skip the already-applied known address —
        // here that is guaranteed structurally because the replay only
        // walks the buffered misses, not the full change set.
        assert_eq!(
            result.found.get(&(3u32, known)).map(|f| f.balance),
            Some(1_500),
            "known AddToCredits delta must apply exactly once across refresh"
        );
        assert_eq!(
            result.found.get(&(9u32, late)).map(|f| f.balance),
            Some(7_000),
            "post-snapshot address still recovered after refresh"
        );
    }

    /// A foreign address (not in the lookup, never produced by the
    /// provider) is silently ignored — no `on_address_found`, no
    /// `result.found` insert, no `result.absent` mutation, and exactly
    /// one provider refresh for the whole pass.
    #[tokio::test]
    async fn apply_block_changes_ignores_foreign_address_without_refresh_storm() {
        use async_trait::async_trait;

        struct CountingNoopProvider {
            pending_polls: std::sync::atomic::AtomicUsize,
            found_calls: usize,
        }

        #[async_trait]
        impl AddressProvider for CountingNoopProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                self.pending_polls
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                std::iter::empty()
            }

            async fn on_address_found(
                &mut self,
                _tag: Self::Tag,
                _address: &Self::Address,
                _funds: AddressFunds,
            ) {
                self.found_calls += 1;
            }

            async fn on_address_absent(&mut self, _tag: Self::Tag, _address: &Self::Address) {}

            fn current_balances(
                &self,
            ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
                std::iter::empty()
            }
        }

        let mine = p2pkh(0x01);
        let foreign_1 = p2pkh(0xF1);
        let foreign_2 = p2pkh(0xF2);
        let foreign_3 = p2pkh(0xF3);

        let mut lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();
        lookup.insert(mine.to_bytes(), (1u32, mine));

        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        let mut provider = CountingNoopProvider {
            pending_polls: std::sync::atomic::AtomicUsize::new(0),
            found_calls: 0,
        };
        let mut pending_unknown: Vec<PendingUnknownChange> = Vec::new();

        // Three separate "blocks" (representing the per-entry calls
        // inside `incremental_catch_up`), every change but the first
        // belongs to another wallet.
        for (addr, credits) in [
            (&mine, 1_000),
            (&foreign_1, 5_000),
            (&foreign_2, 5_000),
            (&foreign_3, 5_000),
        ] {
            let op = BlockAwareCreditOperation::SetCredits(credits);
            let changes = [(addr, AddressBalanceChange::Compacted(&op))];
            apply_block_changes(
                &lookup,
                changes.iter().map(|(a, c)| (*a, *c)),
                0,
                &mut provider,
                &mut result,
                &mut pending_unknown,
            )
            .await;
        }

        // Per-block apply must NEVER refresh the provider — the refresh
        // runs once, at end of pass.
        assert_eq!(
            provider
                .pending_polls
                .load(std::sync::atomic::Ordering::Relaxed),
            0,
            "no per-block pending_addresses() polls — refresh is end-of-pass only"
        );

        // The end-of-pass refresh runs exactly once.
        refresh_and_replay_unknown(&lookup, pending_unknown, &mut provider, &mut result).await;
        assert_eq!(
            provider
                .pending_polls
                .load(std::sync::atomic::Ordering::Relaxed),
            1,
            "end-of-pass refresh must poll the provider exactly once"
        );

        // Foreign addresses must not surface as `found` or fire callbacks.
        assert_eq!(
            result.found.len(),
            1,
            "only the known address is in `found` (foreign addresses ignored)"
        );
        assert_eq!(
            result.found.get(&(1u32, mine)).map(|f| f.balance),
            Some(1_000),
            "known address applied"
        );
        assert!(
            !result
                .found
                .keys()
                .any(|(_, a)| *a == foreign_1 || *a == foreign_2 || *a == foreign_3),
            "no foreign address may be inserted into `result.found`"
        );
        assert!(
            result.absent.is_empty(),
            "foreign addresses must not be marked `absent` either"
        );
        assert_eq!(
            provider.found_calls, 1,
            "on_address_found fires only for the known address"
        );

        // `found` and `absent` stay disjoint.
        for key in result.found.keys() {
            assert!(
                !result.absent.contains(key),
                "found ∩ absent must be empty: {key:?} in both"
            );
        }
    }
}
