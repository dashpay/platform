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
                // Pin the scan absolute at the snapshot height: the trunk
                // proof attests this balance as of `checkpoint_height`, so
                // any delta recorded at or below it is already included.
                let mut funds = AddressFunds::try_from(element)?;
                funds.as_of_height = context.result.checkpoint_height;
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
                // Branch queries are checkpointed to the trunk query's
                // height, so branch absolutes carry the same pin.
                let mut funds = AddressFunds::try_from(element)?;
                funds.as_of_height = context.result.checkpoint_height;
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
    // Defensive fold of the `current_balances` invariant (see
    // `AddressProvider::current_balances`): seed any current-balance-only
    // address so its delta stays on the direct apply path; pending wins.
    for (tag, address, _funds) in provider.current_balances() {
        key_to_tag
            .entry(address.to_bytes())
            .or_insert((tag, address));
    }

    // Initialize result
    let mut result: AddressSyncResult<P::Tag, P::Address> = AddressSyncResult::new();

    // Nothing to scan when no addresses are pending. Return a visibly-empty
    // result: the watermark fields stay 0 and `sync_finished` is not called,
    // so a caller that persists `new_sync_height` after each sync can tell
    // this pass did no work and will not regress its watermark to 0. We
    // deliberately do NOT seed `found` from `current_balances` here — doing so
    // would only echo the caller's own base balances back while disguising a
    // zero-watermark no-op as a populated, successful sync.
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
        seed_base_balances(&mut result, &key_to_tag, &*provider);
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

    // Seed base balances the scan missed (invariant-violating bridge case).
    // Runs after the authoritative scan inserts, so the scan wins on overlap;
    // fills gaps so a later `AddToCredits` delta is `existing + X`, not `0 + X`.
    seed_base_balances(&mut result, &key_to_tag, &*provider);

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

/// Seed `result.found` with the provider's known base balances for any
/// address the authoritative scan did not already record, so a later
/// `AddToCredits` delta accumulates on `existing + X` rather than `0 + X`.
/// This is the defensive bridge for a provider that violates the
/// [`current_balances`](AddressProvider::current_balances) invariant (e.g. the
/// FFI batch provider built from two independent caller-supplied arrays).
///
/// Two guards keep this defensive seed from corrupting the scan's result:
///
/// - **Tag reconciliation.** The `(tag, address)` key is resolved through
///   `key_to_tag` (which folds `pending_addresses` first, so *pending wins*)
///   rather than trusting the `current_balances` tag. A tag mismatch between
///   the two views would otherwise split one address across two result keys —
///   the seeded base under one tag, a later delta under the pending tag — and
///   double-count it.
///
/// - **Absent-scan wins.** An address the tree scan proved absent is skipped:
///   the scan is authoritative, so a stale cached balance must never resurrect
///   a proven-absent address back into `found` (which would violate
///   `found`/`absent` disjointness and inflate `total_balance`).
fn seed_base_balances<P: AddressProvider>(
    result: &mut AddressSyncResult<P::Tag, P::Address>,
    key_to_tag: &HashMap<Vec<u8>, (P::Tag, P::Address)>,
    provider: &P,
) {
    for (tag, address, funds) in provider.current_balances() {
        // Resolve to the key the delta path will use (pending wins).
        let result_key = key_to_tag
            .get(&address.to_bytes())
            .copied()
            .unwrap_or((tag, address));
        // Never resurrect a scan-proven-absent address from a cached balance.
        if result.absent.contains(&result_key) {
            continue;
        }
        result.found.entry(result_key).or_insert(funds);
    }
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
    let mut pending_unknown: Vec<PendingMiss> = Vec::new();

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
                // TODO(address-sync): can't tell a transient failure (keep the
                // cursor) from "server lacks incremental RPCs" (advance) without
                // typed errors, so we advance the watermark in both — a transient
                // failure then skips blocks until the next full rescan.
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

            let mut entries = match changes {
                Some(c) => c.into_inner(),
                None => break,
            };
            // Apply in ascending range order so per-address pins advance
            // monotonically — an out-of-order apply could gate off a
            // genuinely newer delta.
            entries.sort_by_key(|e| e.end_block_height);

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
                        .map(|(a, op)| (a, BalanceOp::Compacted(op))),
                    // The pin gate needs the height this change is recorded
                    // AS OF — the range end, not the pagination cursor.
                    entry.end_block_height,
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
        let mut entries = changes.into_inner();
        result.metrics.recent_entries_returned += entries.len();

        // Apply in ascending block order so per-address pins advance
        // monotonically — an out-of-order apply could gate off a genuinely
        // newer delta.
        entries.sort_by_key(|e| e.block_height);

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
                    .map(|(a, op)| (a, BalanceOp::Recent(op))),
                // The pin gate needs the block this change is recorded AT,
                // not the pagination cursor.
                entry.block_height,
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

    // TODO(address-sync): the watermark advances past every applied block, so a
    // delta dropped by the replay livelock guard is never re-queried (full
    // rescan only). Holding it back risks hot-path re-query storms; deferred —
    // the guard is set high enough that legitimate chains never hit it.
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

/// A single borrowed address balance change, abstracting the recent
/// (`CreditOperation`) and compacted (`BlockAwareCreditOperation`) shapes so one
/// pure function can apply both phases identically.
#[derive(Clone, Copy)]
pub(crate) enum BalanceOp<'a> {
    /// A recent (per-block) credit operation.
    Recent(&'a CreditOperation),
    /// A compacted (block-range) credit operation.
    Compacted(&'a BlockAwareCreditOperation),
}

/// Owned arity of [`BalanceOp`], used only to buffer a miss past the borrow of
/// its response entry so it can be replayed at end-of-pass.
#[derive(Clone)]
pub(crate) enum OwnedBalanceOp {
    Recent(CreditOperation),
    Compacted(BlockAwareCreditOperation),
}

/// A buffered miss: raw GroveDB key, owned change, and the height the change
/// is recorded as of (recent: the entry's block height; compacted: the range
/// end). Feeds the height-pin gate on replay exactly as on the forward pass.
type PendingMiss = (Vec<u8>, OwnedBalanceOp, u64);

/// Resolve the post-change funds from the current funds and the height the
/// change is recorded as of (`op_height` — recent: the entry's block height;
/// compacted: the range end).
///
/// This is where the height pin (`AddressFunds::as_of_height`) gates delta
/// replay: the pinned balance already includes every block up to and
/// including the pin, so a change at or below it is already inside the
/// absolute — re-applying it is the ADDR-09 double-count (a fresh trunk/ST
/// absolute plus the same block's `AddToCredits` replayed on top). Returns
/// `current` unchanged when the change is fully gated off; applying advances
/// the pin so later passes gate correctly too.
fn apply_op(op: BalanceOp<'_>, current: AddressFunds, op_height: u64) -> AddressFunds {
    match op {
        BalanceOp::Recent(op) => {
            if op_height <= current.as_of_height {
                return current;
            }
            let balance = match op {
                CreditOperation::SetCredits(credits) => *credits,
                CreditOperation::AddToCredits(credits) => current.balance.saturating_add(*credits),
            };
            AddressFunds {
                nonce: current.nonce,
                balance,
                as_of_height: op_height,
            }
        }
        BalanceOp::Compacted(op) => match op {
            BlockAwareCreditOperation::SetCredits(credits) => {
                // Absolute as of the end of this compacted range —
                // authoritative only if it postdates the pin.
                if op_height <= current.as_of_height {
                    return current;
                }
                AddressFunds {
                    nonce: current.nonce,
                    balance: *credits,
                    as_of_height: op_height,
                }
            }
            BlockAwareCreditOperation::AddToCreditsOperations(operations) => {
                // Each operation carries its own block height, so a pin that
                // falls inside the compacted range drops exactly the ops the
                // pinned absolute already includes and applies the rest.
                let total_to_add: u64 = operations
                    .iter()
                    .filter(|(height, _)| **height > current.as_of_height)
                    .map(|(_, credits)| *credits)
                    .fold(0u64, |acc, c| acc.saturating_add(c));
                AddressFunds {
                    nonce: current.nonce,
                    balance: current.balance.saturating_add(total_to_add),
                    // The entry aggregates every change for this address
                    // through its range end, so the balance is now current
                    // through there.
                    as_of_height: current.as_of_height.max(op_height),
                }
            }
        },
    }
}

/// Borrow an owned op back into [`BalanceOp`] for replay.
fn borrow_op(op: &OwnedBalanceOp) -> BalanceOp<'_> {
    match op {
        OwnedBalanceOp::Recent(op) => BalanceOp::Recent(op),
        OwnedBalanceOp::Compacted(op) => BalanceOp::Compacted(op),
    }
}

/// Apply a single balance change to `result` for an already-resolved
/// `(tag, address)`. Returns `Some(funds)` when the balance moved — the
/// caller records the update and fires `on_address_found` — or `None` on a
/// no-op.
///
/// This is the single home for the two invariants the forward apply pass
/// ([`apply_block_changes`]) and the end-of-pass replay
/// ([`refresh_and_replay_unknown`]) must keep byte-for-byte identical: the
/// `found`/`absent` disjointness rule, and the synthesized-nonce rule. A
/// future change to either lands here once instead of on two divergent
/// copies.
fn apply_change<P: AddressProvider>(
    result: &mut AddressSyncResult<P::Tag, P::Address>,
    tag: P::Tag,
    address: P::Address,
    op: BalanceOp<'_>,
    op_height: u64,
) -> Option<AddressFunds> {
    let result_key = (tag, address);
    // INTENTIONAL — accepted risk: incremental RPCs carry no nonce, so a
    // catch-up-discovered address synthesizes nonce=0. It is published and
    // persisted but NON-AUTHORITATIVE — every spend re-fetches the on-chain
    // nonce. Callers MUST NOT treat this as the authoritative nonce.
    // (Option<u32> rework deliberately skipped.)
    //
    // `as_of_height: 0` = "unknown provenance" for an address with no
    // recorded funds: every change applies, and the first one pins it.
    let current = result
        .found
        .get(&result_key)
        .copied()
        .unwrap_or(AddressFunds {
            nonce: 0,
            balance: 0,
            as_of_height: 0,
        });

    let funds = apply_op(op, current, op_height);
    // Commit on ANY funds change — a balance-neutral change that advances
    // the pin still persists, hardening future replay gating.
    if funds == current {
        return None;
    }

    // Keep `found` and `absent` disjoint: a post-checkpoint funding can land
    // on an address the branch scan proved absent, so clear any stale
    // `absent` entry before recording it as found.
    result.absent.remove(&result_key);
    result.found.insert(result_key, funds);
    Some(funds)
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
    op_height: u64,
    provider: &mut P,
    result: &mut AddressSyncResult<P::Tag, P::Address>,
    pending_unknown: &mut Vec<PendingMiss>,
) where
    P: AddressProvider,
    I: IntoIterator<Item = (&'a PlatformAddress, BalanceOp<'a>)>,
{
    let mut local_applied: Vec<(P::Tag, P::Address, AddressFunds)> = Vec::new();

    for (platform_addr, change) in changes {
        let addr_bytes = platform_addr.to_bytes();
        if let Some(&(tag, address)) = address_lookup.get(&addr_bytes) {
            if let Some(funds) = apply_change::<P>(result, tag, address, change, op_height) {
                local_applied.push((tag, address, funds));
            }
        } else {
            let owned = match change {
                BalanceOp::Recent(op) => OwnedBalanceOp::Recent(*op),
                BalanceOp::Compacted(op) => OwnedBalanceOp::Compacted(op.clone()),
            };
            pending_unknown.push((addr_bytes, owned, op_height));
            // NOTE: this buffer is intentionally unbounded — premature
            // optimization here would couple the catch-up loop to ad-hoc
            // memory heuristics. We log a one-shot warning above a generous
            // threshold so a future operator can observe whether this path
            // actually exceeds the threshold of buffered foreign-wallet
            // changes in real workloads. If it ever does, the owned change
            // must still be buffered here rather than just its `Vec<u8>` key:
            // the RPC response entries are dropped at the end of each
            // pagination iteration, so once the end-of-pass refresh resolves a
            // key there is nothing left in memory to re-derive its dropped
            // change from. A bounded-memory fix would therefore have to
            // re-query the changes for the resolved keys, not merely stash
            // their keys.
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

/// Livelock guard for the refresh+replay loop — NOT a functional limit
/// on gap-extension depth. The loop iterates so a `pending_addresses()`
/// set that grows via `on_address_found`-triggered gap extension is
/// picked up in the same pass; each iteration must resolve at least one
/// new address or the loop exits early, so a well-behaved provider never
/// approaches this bound. The cap exists solely to bound a buggy or
/// adversarial provider that keeps emitting ever-new pending addresses,
/// turning the loop into a livelock. Set generously so legitimate deep
/// chains complete in one pass.
const REPLAY_REFRESH_MAX_ITERATIONS: usize = 32;

/// End-of-pass recovery for addresses missing from the entry-time
/// snapshot. Re-polls `pending_addresses()`, builds a small `extras` map
/// of newly-derived addresses, and replays only the buffered changes
/// that match an `extras` entry. Foreign (other-wallet) addresses fall
/// out at the intersection check — no provider refresh storm, no log
/// flood.
///
/// The refresh+replay is wrapped in a loop so that `on_address_found`
/// callbacks fired during replay can trigger gap extension on the
/// provider and surface follow-on addresses (e.g. address `A+1` that the
/// provider only exposes after seeing `A` was used). Iteration stops as
/// soon as no new addresses are resolved; the
/// [`REPLAY_REFRESH_MAX_ITERATIONS`] livelock guard only bounds a
/// misbehaving provider and is not expected to be reached in practice.
async fn refresh_and_replay_unknown<P: AddressProvider>(
    key_to_tag: &HashMap<Vec<u8>, (P::Tag, P::Address)>,
    pending_unknown: Vec<PendingMiss>,
    provider: &mut P,
    result: &mut AddressSyncResult<P::Tag, P::Address>,
) {
    if pending_unknown.is_empty() {
        return;
    }

    // Build the set of unknown keys for a fast intersection probe.
    let unknown_keys: std::collections::HashSet<&[u8]> = pending_unknown
        .iter()
        .map(|(key, _, _)| key.as_slice())
        .collect();

    // Keys resolved across all iterations so we don't double-apply a
    // delta if a follow-on iteration's `extras` still contains an
    // already-replayed key. Owned bytes because the borrow checker won't
    // let us keep `&[u8]` references into `pending_unknown` while we
    // also borrow it for the inner loop.
    let mut resolved_keys: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();
    let mut total_replay_applied: usize = 0;

    for iteration in 0..REPLAY_REFRESH_MAX_ITERATIONS {
        // Only addresses the provider can now produce AND that match a
        // still-unresolved buffered miss are interesting — everything
        // else is some other wallet's address and stays out of the
        // lookup entirely.
        let mut extras: HashMap<Vec<u8>, (P::Tag, P::Address)> = HashMap::new();
        for (tag, address) in provider.pending_addresses() {
            let bytes = address.to_bytes();
            if unknown_keys.contains(bytes.as_slice())
                && !resolved_keys.contains(&bytes)
                && !key_to_tag.contains_key(&bytes)
            {
                extras.insert(bytes, (tag, address));
            }
        }

        if extras.is_empty() {
            if iteration == 0 {
                // Common case on a populated multi-wallet chain: the provider
                // offers none of the buffered unknowns even after a refresh —
                // typically because they belong to another wallet.
                debug!(
                    "Address sync: {} platform-reported balance change(s) reference \
                     address(es) the provider did not offer during this pass; ignoring",
                    pending_unknown.len()
                );
                return;
            }
            // No new addresses surfaced this iteration — we're done.
            break;
        }

        // Replay only the entries whose key actually resolves in
        // `extras` and hasn't been resolved in a prior iteration. Order
        // is preserved (compacted first, then recent — same as the
        // forward pass), so `AddToCredits` deltas accumulate correctly.
        // The catch-up cursor per change is preserved so the compacted
        // height filter still sees the same `current_height` it would
        // have seen on the forward pass.
        let mut iteration_applied: Vec<(P::Tag, P::Address, AddressFunds)> = Vec::new();
        for (key, change, height) in &pending_unknown {
            if resolved_keys.contains(key) {
                continue;
            }
            let Some(&(tag, address)) = extras.get(key.as_slice()) else {
                continue;
            };
            if let Some(funds) = apply_change::<P>(result, tag, address, borrow_op(change), *height)
            {
                iteration_applied.push((tag, address, funds));
            }
        }

        // Mark every key whose entry resolved in `extras` as resolved
        // this pass — even if no balance moved — so the next iteration
        // doesn't reconsider it.
        for (key, _, _) in &pending_unknown {
            if extras.contains_key(key.as_slice()) {
                resolved_keys.insert(key.clone());
            }
        }

        let iteration_resolved = iteration_applied.len();
        total_replay_applied += iteration_resolved;

        // Fire callbacks for this iteration BEFORE the next refresh so
        // that `on_address_found`-driven gap extension can expose the
        // next batch of addresses to `pending_addresses()`.
        for (tag, address, funds) in &iteration_applied {
            provider.on_address_found(*tag, address, *funds).await;
        }

        if iteration_resolved == 0 {
            // `extras` was non-empty but every entry's delta was a
            // no-op; nothing for gap extension to chew on.
            break;
        }
    }

    // Classify the still-unresolved tail. A key the provider can still
    // produce is wallet-owned loss the livelock guard stranded — only a
    // full rescan recovers it, since the next RangeAfter sync skips the
    // block. A key the provider never offered during this pass is treated as
    // foreign (other-wallet) noise and ignored. That is a heuristic, not a
    // proof of foreignness: an address this wallet derived just after the
    // pass ended also lands here. Ignoring it now is still safe because the
    // periodic full rescan reads absolute balances and recovers it — so this
    // path must never be reported as definitive data loss.
    let provider_keys: std::collections::HashSet<Vec<u8>> = provider
        .pending_addresses()
        .map(|(_, address)| address.to_bytes())
        .collect();
    let mut wallet_owned_lost = 0usize;
    let mut foreign = 0usize;
    for (key, _, _) in &pending_unknown {
        if resolved_keys.contains(key) {
            continue;
        }
        if provider_keys.contains(key) {
            wallet_owned_lost += 1;
        } else {
            foreign += 1;
        }
    }
    // Only a wallet-owned loss is worth a warning — foreign noise is
    // expected on a shared chain and must not be reported as lost data.
    // A wallet-owned key only survives the loop when the guard truncates
    // it, so this fires solely on a real cap hit.
    if wallet_owned_lost > 0 {
        warn!(
            "Address sync: {} wallet-owned buffered balance change(s) remained \
             unresolved when the livelock guard truncated refresh+replay \
             (recovered {} first); these are dropped until a full rescan \
             ({} foreign address(es) ignored)",
            wallet_owned_lost, total_replay_applied, foreign
        );
    } else if foreign > 0 {
        debug!(
            "Address sync: {} platform-reported balance change(s) reference \
             address(es) the provider did not offer during this pass (refresh \
             recovered {} other(s)); ignoring the un-offered entries",
            foreign, total_replay_applied
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
    ///
    /// The element itself carries no block height, so the produced funds
    /// have `as_of_height == 0`; the caller must pin them at the proof
    /// height of the query that returned the element.
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
            return Ok(AddressFunds {
                nonce,
                balance,
                as_of_height: 0,
            });
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
                as_of_height: 0,
            },
        );
        result.found.insert(
            (1, dpp::address_funds::PlatformAddress::P2pkh([2; 20])),
            AddressFunds {
                nonce: 1,
                balance: 0,
                as_of_height: 0,
            },
        );
        result.found.insert(
            (2, dpp::address_funds::PlatformAddress::P2pkh([3; 20])),
            AddressFunds {
                nonce: 2,
                balance: 1500,
                as_of_height: 0,
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
        let mut pending_unknown: Vec<PendingMiss> = Vec::new();

        let op = BlockAwareCreditOperation::SetCredits(42_000);
        let changes = [(&late, BalanceOp::Compacted(&op))];

        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            // A real block height — heights are never 0 in production, and
            // the pin gate drops changes at or below the current pin.
            100,
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
        let changes = [(&addr, BalanceOp::Compacted(&op))];

        let mut pending_unknown: Vec<PendingMiss> = Vec::new();
        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            // A real block height — heights are never 0 in production, and
            // the pin gate drops changes at or below the current pin.
            100,
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
                as_of_height: 0,
            },
        );

        let mut provider = GrowingProvider { late };

        let known_op = BlockAwareCreditOperation::AddToCreditsOperations(
            std::iter::once((50u64, 500u64)).collect(),
        );
        let late_op = BlockAwareCreditOperation::SetCredits(7_000);
        let changes = [
            (&known, BalanceOp::Compacted(&known_op)),
            (&late, BalanceOp::Compacted(&late_op)),
        ];

        let mut pending_unknown: Vec<PendingMiss> = Vec::new();
        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            // A real block height — heights are never 0 in production, and
            // the pin gate drops changes at or below the current pin.
            100,
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
        let mut pending_unknown: Vec<PendingMiss> = Vec::new();

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
            let changes = [(addr, BalanceOp::Compacted(&op))];
            apply_block_changes(
                &lookup,
                changes.iter().map(|(a, c)| (*a, *c)),
                // A real block height — heights are never 0 in production, and
                // the pin gate drops changes at or below the current pin.
                100,
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

    /// Two post-snapshot addresses A and A+1 where the provider only
    /// exposes A initially and extends its gap to include A+1 from
    /// inside `on_address_found(A, ...)`. The bounded-iteration replay
    /// must pick up A+1 in a follow-on iteration instead of leaving
    /// its buffered change silently dropped until the next sync.
    #[tokio::test]
    async fn refresh_loops_until_gap_extension_recovers_follow_on_address() {
        use async_trait::async_trait;

        struct GapExtendingProvider {
            a: PlatformAddress,
            b: PlatformAddress,
            // false until `on_address_found(a, ...)` mutates it — then
            // `pending_addresses()` returns both A and B.
            extended: bool,
            found: Vec<(u32, PlatformAddress, AddressFunds)>,
        }

        #[async_trait]
        impl AddressProvider for GapExtendingProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                // First call returns just A; once `on_address_found(A, …)`
                // has flipped `extended`, subsequent calls also yield B.
                // The recovery of B is what proves the loop ran more
                // than once.
                let initial = std::iter::once((10u32, self.a));
                let extended = self
                    .extended
                    .then(|| std::iter::once((11u32, self.b)))
                    .into_iter()
                    .flatten();
                initial.chain(extended)
            }

            async fn on_address_found(
                &mut self,
                tag: Self::Tag,
                address: &Self::Address,
                funds: AddressFunds,
            ) {
                self.found.push((tag, *address, funds));
                // The hook that simulates HD-wallet gap extension: as
                // soon as A is observed, expose A+1 as the next pending
                // address.
                if *address == self.a {
                    self.extended = true;
                }
            }

            async fn on_address_absent(&mut self, _tag: Self::Tag, _address: &Self::Address) {}

            fn current_balances(
                &self,
            ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
                std::iter::empty()
            }
        }

        let a = p2pkh(0xAA);
        let b = p2pkh(0xBB);

        // Both A and B are post-snapshot — entry-time lookup is empty.
        let lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();

        let mut provider = GapExtendingProvider {
            a,
            b,
            extended: false,
            found: Vec::new(),
        };
        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();

        // Buffer changes for both A and B as if `apply_block_changes`
        // had already seen them and stashed them for end-of-pass replay.
        let op_a = BlockAwareCreditOperation::SetCredits(1_111);
        let op_b = BlockAwareCreditOperation::SetCredits(2_222);
        let pending_unknown: Vec<PendingMiss> = vec![
            (a.to_bytes(), OwnedBalanceOp::Compacted(op_a), 100),
            (b.to_bytes(), OwnedBalanceOp::Compacted(op_b), 100),
        ];

        refresh_and_replay_unknown(&lookup, pending_unknown, &mut provider, &mut result).await;

        // A surfaced on iteration 0, then `on_address_found(A,...)`
        // flipped `extended = true`, so iteration 1 sees B and applies
        // its balance too.
        assert_eq!(
            result.found.get(&(10u32, a)).map(|f| f.balance),
            Some(1_111),
            "A must be recovered by the first iteration"
        );
        assert_eq!(
            result.found.get(&(11u32, b)).map(|f| f.balance),
            Some(2_222),
            "B must be recovered by the bounded-iteration follow-up"
        );
        assert!(
            provider
                .found
                .iter()
                .any(|(t, addr, f)| *t == 10 && *addr == a && f.balance == 1_111),
            "on_address_found must fire for A"
        );
        assert!(
            provider
                .found
                .iter()
                .any(|(t, addr, f)| *t == 11 && *addr == b && f.balance == 2_222),
            "on_address_found must fire for B in the follow-on iteration"
        );
    }

    /// A gap-extension chain DEEPER than the old 3-iteration cap:
    /// A → A+1 → A+2 → A+3 → A+4, where each address is only exposed by
    /// `pending_addresses()` after its predecessor's `on_address_found`
    /// fires. With the cap raised to 32 the whole chain resolves in a
    /// single end-of-pass pass — the old cap of 3 would have stranded
    /// A+3 and A+4. Guards against the cap regressing into a functional
    /// depth limit.
    #[tokio::test]
    async fn refresh_resolves_gap_extension_chain_deeper_than_old_cap() {
        use async_trait::async_trait;

        const CHAIN: [(u32, u8, u64); 5] = [
            (20, 0xA0, 1_000),
            (21, 0xA1, 2_000),
            (22, 0xA2, 3_000),
            (23, 0xA3, 4_000),
            (24, 0xA4, 5_000),
        ];

        struct ChainProvider {
            addrs: Vec<PlatformAddress>,
            // How many chain links the provider currently exposes. Starts
            // at 1 (just the head); each `on_address_found` for the
            // deepest exposed link extends the gap by one.
            exposed: usize,
            found: Vec<(u32, PlatformAddress, AddressFunds)>,
        }

        #[async_trait]
        impl AddressProvider for ChainProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                (0..self.exposed).map(|i| (CHAIN[i].0, self.addrs[i]))
            }

            async fn on_address_found(
                &mut self,
                tag: Self::Tag,
                address: &Self::Address,
                funds: AddressFunds,
            ) {
                self.found.push((tag, *address, funds));
                // Extend the gap one link deeper when the current deepest
                // exposed address is the one that was just found.
                if self.exposed < CHAIN.len() && *address == self.addrs[self.exposed - 1] {
                    self.exposed += 1;
                }
            }

            async fn on_address_absent(&mut self, _tag: Self::Tag, _address: &Self::Address) {}

            fn current_balances(
                &self,
            ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
                std::iter::empty()
            }
        }

        let addrs: Vec<PlatformAddress> = CHAIN.iter().map(|(_, b, _)| p2pkh(*b)).collect();

        // Every link is post-snapshot — the entry-time lookup is empty,
        // so all five changes are buffered and must be recovered by the
        // looping replay.
        let lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();

        let pending_unknown: Vec<PendingMiss> = CHAIN
            .iter()
            .enumerate()
            .map(|(i, (_, _, credits))| {
                (
                    addrs[i].to_bytes(),
                    OwnedBalanceOp::Compacted(BlockAwareCreditOperation::SetCredits(*credits)),
                    100,
                )
            })
            .collect();

        let mut provider = ChainProvider {
            addrs: addrs.clone(),
            exposed: 1,
            found: Vec::new(),
        };
        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();

        refresh_and_replay_unknown(&lookup, pending_unknown, &mut provider, &mut result).await;

        // The chain depth (5) exceeds the old cap (3); all five must land
        // in this single pass thanks to the raised livelock guard.
        for (i, (tag, _, credits)) in CHAIN.iter().enumerate() {
            assert_eq!(
                result.found.get(&(*tag, addrs[i])).map(|f| f.balance),
                Some(*credits),
                "chain link {i} (depth {}) must resolve in one pass under the raised cap",
                i + 1
            );
            assert!(
                provider
                    .found
                    .iter()
                    .any(|(t, a, f)| t == tag && *a == addrs[i] && f.balance == *credits),
                "on_address_found must fire for chain link {i}"
            );
        }
    }

    /// Counts `WARN`-level `tracing` events on the current thread so a
    /// test can assert whether a wallet-owned-loss warning fired.
    #[derive(Clone, Default)]
    struct WarnCounter(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl<S> tracing_subscriber::Layer<S> for WarnCounter
    where
        S: tracing::Subscriber,
    {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    /// At the livelock-guard cap, the still-unresolved tail must be split:
    /// a wallet-owned stray (still in `pending_addresses()`) raises a WARN;
    /// foreign-only leftovers (never offered by the provider) must NOT —
    /// they are re-ignored by a full rescan, not lost data.
    #[tokio::test]
    async fn cap_hit_warns_on_wallet_owned_loss_but_not_foreign_noise() {
        use async_trait::async_trait;
        use std::sync::atomic::Ordering;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        // Endless gap-extension drip: each `on_address_found` exposes one
        // brand-new wallet address, so the loop never runs out of work and
        // hits `REPLAY_REFRESH_MAX_ITERATIONS`, stranding the next exposed
        // (still-pending) address as wallet-owned loss. A `foreign` address
        // is buffered but never offered by the provider.
        struct EndlessChainProvider {
            // Deterministic per-index wallet addresses; index 0 is the head.
            exposed: usize,
        }

        fn chain_addr(i: usize) -> PlatformAddress {
            // Distinct from any foreign byte; high bit set to avoid clashes.
            p2pkh(0x80u8.wrapping_add((i % 0x40) as u8))
        }

        #[async_trait]
        impl AddressProvider for EndlessChainProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                (0..self.exposed).map(|i| (i as u32, chain_addr(i)))
            }

            async fn on_address_found(
                &mut self,
                tag: Self::Tag,
                _address: &Self::Address,
                _funds: AddressFunds,
            ) {
                // Expose the next link only once the current deepest one is
                // found, so each iteration surfaces exactly one new address.
                if tag as usize + 1 == self.exposed {
                    self.exposed += 1;
                }
            }

            async fn on_address_absent(&mut self, _tag: Self::Tag, _address: &Self::Address) {}

            fn current_balances(
                &self,
            ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
                std::iter::empty()
            }
        }

        let foreign = p2pkh(0x01);

        // One buffered miss per chain link deeper than the cap, plus the
        // foreign one. The drip resolves the cap's worth and strands the
        // rest — at least one strand is still in `pending_addresses()`.
        let mut pending_unknown: Vec<PendingMiss> = (0..=REPLAY_REFRESH_MAX_ITERATIONS)
            .map(|i| {
                (
                    chain_addr(i).to_bytes(),
                    OwnedBalanceOp::Compacted(BlockAwareCreditOperation::SetCredits(
                        1_000 + i as u64,
                    )),
                    100,
                )
            })
            .collect();
        pending_unknown.push((
            foreign.to_bytes(),
            OwnedBalanceOp::Compacted(BlockAwareCreditOperation::SetCredits(9_999)),
            100,
        ));

        let lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();

        // Wallet-owned cap loss present → exactly one WARN expected.
        let warns = WarnCounter::default();
        let collected = warns.clone();
        {
            let _guard = tracing_subscriber::registry().with(warns).set_default();
            let mut provider = EndlessChainProvider { exposed: 1 };
            let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
            refresh_and_replay_unknown(&lookup, pending_unknown, &mut provider, &mut result).await;
        }
        assert_eq!(
            collected.0.load(Ordering::Relaxed),
            1,
            "a stranded wallet-owned address at the cap must raise exactly one WARN"
        );

        // Foreign-leftover branch → no WARN, but the classifier MUST run.
        // The provider offers one resolvable wallet-owned address, so the
        // replay loop runs past the `extras.is_empty()` early-return; a
        // foreign miss it never offers then survives as `foreign`, exercising
        // the foreign-counting branch with `wallet_owned_lost == 0`.
        let owned = p2pkh(0x70);

        struct OneOwnedProvider {
            owned: PlatformAddress,
        }

        #[async_trait]
        impl AddressProvider for OneOwnedProvider {
            type Tag = u32;
            type Address = PlatformAddress;
            fn gap_limit(&self) -> AddressIndex {
                0
            }
            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                std::iter::once((7u32, self.owned))
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

        let foreign_with_owned: Vec<PendingMiss> = vec![
            (
                owned.to_bytes(),
                OwnedBalanceOp::Compacted(BlockAwareCreditOperation::SetCredits(5_000)),
                100,
            ),
            (
                p2pkh(0x12).to_bytes(),
                OwnedBalanceOp::Compacted(BlockAwareCreditOperation::SetCredits(6_000)),
                100,
            ),
        ];

        let warns = WarnCounter::default();
        let collected = warns.clone();
        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        {
            let _guard = tracing_subscriber::registry().with(warns).set_default();
            let mut provider = OneOwnedProvider { owned };
            refresh_and_replay_unknown(&lookup, foreign_with_owned, &mut provider, &mut result)
                .await;
        }
        assert_eq!(
            collected.0.load(Ordering::Relaxed),
            0,
            "a surviving foreign leftover must not raise a wallet-owned-loss WARN"
        );
        // Proves the loop ran past the `extras.is_empty()` early-return and
        // the classifier actually executed its foreign branch — the owned
        // address could only resolve via the replay the early-return skips.
        assert_eq!(
            result.found.get(&(7u32, owned)).map(|f| f.balance),
            Some(5_000),
            "the wallet-owned address must resolve, proving the classifier ran"
        );
    }

    /// The common ~15s incremental resync hot path drives `Recent` ops
    /// through `apply_block_changes`. This pins the `Recent` arm — both
    /// the direct-apply path for a known address and the buffer+replay
    /// path for a post-snapshot one — which had zero coverage.
    #[tokio::test]
    async fn apply_block_changes_recent_op_direct_apply_and_buffer_replay() {
        use async_trait::async_trait;

        let known = p2pkh(0x31);
        let late = p2pkh(0x32);

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
                std::iter::once((2u32, self.late))
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

        // `known` is in the entry-time snapshot with a seeded balance;
        // `late` is post-snapshot and exercises the buffer+replay arm.
        let mut lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();
        lookup.insert(known.to_bytes(), (1u32, known));

        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        result.found.insert(
            (1u32, known),
            AddressFunds {
                nonce: 0,
                balance: 1_000,
                as_of_height: 0,
            },
        );

        let mut provider = GrowingProvider {
            late,
            found: Vec::new(),
        };
        let mut pending_unknown: Vec<PendingMiss> = Vec::new();

        // Recent ops: AddToCredits accumulates on the known base, SetCredits
        // establishes the late address's balance.
        let known_op = CreditOperation::AddToCredits(250);
        let late_op = CreditOperation::SetCredits(8_000);
        let changes = [
            (&known, BalanceOp::Recent(&known_op)),
            (&late, BalanceOp::Recent(&late_op)),
        ];

        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            // A real block height — heights are never 0 in production, and
            // the pin gate drops changes at or below the current pin.
            100,
            &mut provider,
            &mut result,
            &mut pending_unknown,
        )
        .await;

        // Known Recent AddToCredits applied immediately on the direct path.
        assert_eq!(
            result.found.get(&(1u32, known)).map(|f| f.balance),
            Some(1_250),
            "known Recent AddToCredits applies on the direct path"
        );
        // Late address buffered as a Recent miss for end-of-pass replay.
        assert_eq!(
            pending_unknown.len(),
            1,
            "post-snapshot Recent change is buffered for replay"
        );
        assert!(
            matches!(pending_unknown[0].1, OwnedBalanceOp::Recent(_)),
            "buffered miss must preserve the Recent op shape"
        );

        refresh_and_replay_unknown(&lookup, pending_unknown, &mut provider, &mut result).await;

        // Late Recent SetCredits recovered after the refresh.
        assert_eq!(
            result.found.get(&(2u32, late)).map(|f| f.balance),
            Some(8_000),
            "post-snapshot Recent SetCredits recovered after refresh"
        );
        assert!(
            provider
                .found
                .iter()
                .any(|(t, a, f)| *t == 2 && *a == late && f.balance == 8_000),
            "on_address_found fires for the recovered Recent address"
        );
    }

    /// The compacted `AddToCreditsOperations` height filter
    /// (`apply_op`, `>= current_height`) must drop deltas at heights
    /// below the catch-up cursor (already counted by the tree scan) while
    /// summing deltas at or above it. A single op carrying entries on both
    /// sides of a discriminating cursor pins the anti-double-count edge.
    #[tokio::test]
    async fn apply_block_changes_height_filter_drops_below_cursor() {
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

        let addr = p2pkh(0x41);
        let mut lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();
        lookup.insert(addr.to_bytes(), (1u32, addr));

        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        // The base balance is pinned at height 99: the scan absolute that
        // produced it already includes every block through 99.
        result.found.insert(
            (1u32, addr),
            AddressFunds {
                nonce: 0,
                balance: 10_000,
                as_of_height: 99,
            },
        );

        // Pin at height 99: ops at 98 and 99 are at/below the pin (already
        // inside the pinned absolute, must be dropped as a double-count
        // guard); ops at 100 and 101 postdate it (must apply). Dropped
        // sum = 700, applied sum = 30.
        let op = BlockAwareCreditOperation::AddToCreditsOperations(
            [
                (98u64, 300u64),
                (99u64, 400u64),
                (100u64, 10u64),
                (101u64, 20u64),
            ]
            .into_iter()
            .collect(),
        );
        let changes = [(&addr, BalanceOp::Compacted(&op))];

        let mut pending_unknown: Vec<PendingMiss> = Vec::new();
        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            // The compacted range's end height — becomes the new pin.
            101,
            &mut NoopProvider,
            &mut result,
            &mut pending_unknown,
        )
        .await;

        // 10_000 base + only the post-pin deltas (10 + 20) = 10_030. The
        // at/below-pin 300 + 400 must NOT be counted, and the pin advances
        // to the range end so later replays gate correctly too.
        assert_eq!(
            result.found.get(&(1u32, addr)).copied(),
            Some(AddressFunds {
                nonce: 0,
                balance: 10_030,
                as_of_height: 101,
            }),
            "only deltas at heights above the pin may apply (anti-double-count)"
        );
        assert!(
            pending_unknown.is_empty(),
            "no unknowns for a known address"
        );
    }

    /// The exact ADDR-09 double-count shape, on the pure seam: a fresh
    /// proof-attested absolute (e.g. an asset-lock top-up reconcile) is
    /// pinned at its proof height, and the recent replay then delivers the
    /// same block's `AddToCredits` delta. The pin gate must drop it — the
    /// delta is already inside the absolute — instead of producing
    /// `X + X = 2X`. A genuinely newer delta still applies and advances
    /// the pin.
    #[tokio::test]
    async fn apply_block_changes_drops_recent_delta_already_inside_pinned_absolute() {
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

        let addr = p2pkh(0x42);
        let mut lookup: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();
        lookup.insert(addr.to_bytes(), (1u32, addr));

        // The reconcile seam committed the proof-attested absolute X,
        // pinned at the funding proof's block height.
        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        result.found.insert(
            (1u32, addr),
            AddressFunds {
                nonce: 0,
                balance: 9_985_071_720,
                as_of_height: 379_731,
            },
        );

        // The recent tree replays the funding credit recorded at the SAME
        // block — the ADDR-09 double-count if applied.
        let same_block = CreditOperation::AddToCredits(9_985_071_720);
        let changes = [(&addr, BalanceOp::Recent(&same_block))];
        let mut pending_unknown: Vec<PendingMiss> = Vec::new();
        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            379_731,
            &mut NoopProvider,
            &mut result,
            &mut pending_unknown,
        )
        .await;
        assert_eq!(
            result.found.get(&(1u32, addr)).copied(),
            Some(AddressFunds {
                nonce: 0,
                balance: 9_985_071_720,
                as_of_height: 379_731,
            }),
            "a delta at the pin height is already inside the absolute (ADDR-09)"
        );

        // A genuinely newer delta still applies and advances the pin.
        let newer = CreditOperation::AddToCredits(1_000);
        let changes = [(&addr, BalanceOp::Recent(&newer))];
        apply_block_changes(
            &lookup,
            changes.iter().map(|(a, c)| (*a, *c)),
            379_740,
            &mut NoopProvider,
            &mut result,
            &mut pending_unknown,
        )
        .await;
        assert_eq!(
            result.found.get(&(1u32, addr)).copied(),
            Some(AddressFunds {
                nonce: 0,
                balance: 9_985_072_720,
                as_of_height: 379_740,
            }),
            "a post-pin delta applies once and advances the pin"
        );
    }

    /// A provider that lists an address in `current_balances` but NOT in
    /// `pending_addresses` (the FFI two-array shape that violates the
    /// trait invariant). On the full-scan path the tree scan never sees it,
    /// so without the two defensive folds its delta would be buffered+dropped
    /// (key fold) and, even if applied, computed on a 0 base (balance fold).
    /// This pins both folds: the key fold keeps it off the drop path, and the
    /// base-balance fold makes its `AddToCredits(X)` resolve to `existing + X`,
    /// not `0 + X`.
    #[tokio::test]
    async fn current_balances_fold_seeds_base_balance_on_full_scan() {
        use async_trait::async_trait;

        let seeded = p2pkh(0x51);

        struct SplitArrayProvider {
            seeded: PlatformAddress,
        }

        #[async_trait]
        impl AddressProvider for SplitArrayProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            // Intentionally does NOT list `seeded` — violates the invariant.
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
                std::iter::once((
                    7u32,
                    self.seeded,
                    AddressFunds {
                        nonce: 0,
                        balance: 5_000,
                        as_of_height: 0,
                    },
                ))
            }
        }

        let mut provider = SplitArrayProvider { seeded };

        // Mirror the full-scan seams from `sync_address_balances`: the
        // key_to_tag fold (built before block processing) and the
        // result.found base-balance gap-seed (after the empty scan). The tree
        // scan is empty here, standing in for "never saw a current-balances
        // address", so result.found starts empty — NOT pre-seeded by hand.
        let mut key_to_tag: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();
        for (tag, address) in provider.pending_addresses() {
            key_to_tag.insert(address.to_bytes(), (tag, address));
        }
        for (tag, address, _funds) in provider.current_balances() {
            key_to_tag
                .entry(address.to_bytes())
                .or_insert((tag, address));
        }

        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        seed_base_balances(&mut result, &key_to_tag, &provider);

        assert!(
            key_to_tag.contains_key(&seeded.to_bytes()),
            "key fold must seed the current-balance-only address into the lookup"
        );
        assert_eq!(
            result.found.get(&(7u32, seeded)).map(|f| f.balance),
            Some(5_000),
            "base-balance fold must seed the on-record balance before deltas apply"
        );

        let mut pending_unknown: Vec<PendingMiss> = Vec::new();

        let op = BlockAwareCreditOperation::AddToCreditsOperations(
            std::iter::once((50u64, 1_500u64)).collect(),
        );
        let changes = [(&seeded, BalanceOp::Compacted(&op))];

        apply_block_changes(
            &key_to_tag,
            changes.iter().map(|(a, c)| (*a, *c)),
            // A real block height — heights are never 0 in production, and
            // the pin gate drops changes at or below the current pin.
            100,
            &mut provider,
            &mut result,
            &mut pending_unknown,
        )
        .await;

        assert!(
            pending_unknown.is_empty(),
            "folded address applies on the direct path — never buffered as unknown"
        );
        assert_eq!(
            result.found.get(&(7u32, seeded)).map(|f| f.balance),
            Some(6_500),
            "AddToCredits must compute existing + X (5000 + 1500), not 0 + X"
        );
    }

    /// Finding-1 regression: the post-scan base-balance seed must not
    /// resurrect an address the tree scan already proved absent. The scan is
    /// authoritative — a stale `current_balances` cache entry must stay out of
    /// `found`, preserving `found`/`absent` disjointness and the total.
    #[test]
    fn seed_base_balances_skips_scan_proven_absent() {
        use async_trait::async_trait;

        let addr = p2pkh(0x60);

        struct PrunedProvider {
            addr: PlatformAddress,
        }

        #[async_trait]
        impl AddressProvider for PrunedProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                std::iter::once((3u32, self.addr))
            }

            async fn on_address_found(
                &mut self,
                _tag: Self::Tag,
                _address: &Self::Address,
                _funds: AddressFunds,
            ) {
            }

            async fn on_address_absent(&mut self, _tag: Self::Tag, _address: &Self::Address) {}

            // Stale cache from a previous sync still lists a balance for an
            // address whose on-chain entry was pruned this pass.
            fn current_balances(
                &self,
            ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
                std::iter::once((
                    3u32,
                    self.addr,
                    AddressFunds {
                        nonce: 0,
                        balance: 5_000,
                        as_of_height: 0,
                    },
                ))
            }
        }

        let provider = PrunedProvider { addr };

        let mut key_to_tag: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();
        for (tag, address) in provider.pending_addresses() {
            key_to_tag.insert(address.to_bytes(), (tag, address));
        }

        // The tree scan proved the address absent this pass.
        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        result.absent.insert((3u32, addr));

        seed_base_balances(&mut result, &key_to_tag, &provider);

        assert!(
            !result.found.contains_key(&(3u32, addr)),
            "must not resurrect a scan-proven-absent address into found"
        );
        assert!(
            result.absent.contains(&(3u32, addr)),
            "the address stays absent — the scan is authoritative"
        );
        assert_eq!(
            result.total_balance(),
            0,
            "no stale cached balance leaks into the total"
        );
    }

    /// Finding-2 regression: when `pending_addresses` and `current_balances`
    /// disagree on the tag for one address, the base-balance seed must resolve
    /// the tag through `key_to_tag` (pending wins) so the seed and a later
    /// delta land on the SAME result key — not two keys that double-count.
    #[tokio::test]
    async fn seed_base_balances_resolves_tag_through_lookup() {
        use async_trait::async_trait;

        let addr = p2pkh(0x61);

        struct MismatchedTagProvider {
            addr: PlatformAddress,
        }

        #[async_trait]
        impl AddressProvider for MismatchedTagProvider {
            type Tag = u32;
            type Address = PlatformAddress;

            fn gap_limit(&self) -> AddressIndex {
                0
            }

            // Pending view tags the address 10.
            fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
                std::iter::once((10u32, self.addr))
            }

            async fn on_address_found(
                &mut self,
                _tag: Self::Tag,
                _address: &Self::Address,
                _funds: AddressFunds,
            ) {
            }

            async fn on_address_absent(&mut self, _tag: Self::Tag, _address: &Self::Address) {}

            // current_balances disagrees, tagging the SAME address 11
            // (violates the same-(tag, address)-pairing invariant).
            fn current_balances(
                &self,
            ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
                std::iter::once((
                    11u32,
                    self.addr,
                    AddressFunds {
                        nonce: 0,
                        balance: 5_000,
                        as_of_height: 0,
                    },
                ))
            }
        }

        let mut provider = MismatchedTagProvider { addr };

        // Build the lookup exactly as `sync_address_balances` does: pending
        // first, then the current_balances fold (pending wins on conflict).
        let mut key_to_tag: HashMap<Vec<u8>, (u32, PlatformAddress)> = HashMap::new();
        for (tag, address) in provider.pending_addresses() {
            key_to_tag.insert(address.to_bytes(), (tag, address));
        }
        for (tag, address, _funds) in provider.current_balances() {
            key_to_tag
                .entry(address.to_bytes())
                .or_insert((tag, address));
        }

        let mut result: AddressSyncResult<u32, PlatformAddress> = AddressSyncResult::new();
        seed_base_balances(&mut result, &key_to_tag, &provider);

        // Seeded under the pending tag (10), never the current_balances tag (11).
        assert_eq!(
            result.found.get(&(10u32, addr)).map(|f| f.balance),
            Some(5_000),
            "base seed lands under the resolved (pending) tag"
        );
        assert!(
            !result.found.contains_key(&(11u32, addr)),
            "must not seed under the mismatched current_balances tag"
        );
        assert_eq!(
            result.found.len(),
            1,
            "one address resolves to exactly one result key"
        );

        // A later AddToCredits delta resolves via key_to_tag to (10, addr) and
        // accumulates on the seeded base: 5000 + 1500 = 6500 under ONE key.
        let op = BlockAwareCreditOperation::AddToCreditsOperations(
            std::iter::once((50u64, 1_500u64)).collect(),
        );
        let changes = [(&addr, BalanceOp::Compacted(&op))];
        let mut pending_unknown: Vec<PendingMiss> = Vec::new();
        apply_block_changes(
            &key_to_tag,
            changes.iter().map(|(a, c)| (*a, *c)),
            // A real block height — heights are never 0 in production, and
            // the pin gate drops changes at or below the current pin.
            100,
            &mut provider,
            &mut result,
            &mut pending_unknown,
        )
        .await;

        assert_eq!(
            result.found.get(&(10u32, addr)).map(|f| f.balance),
            Some(6_500),
            "delta accumulates once on the seeded base under the resolved key"
        );
        assert_eq!(
            result.found.len(),
            1,
            "still exactly one result key — no split, no double-count"
        );
    }
}
