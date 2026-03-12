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
//! use dash_sdk::platform::nullifier_sync::NullifierSyncCheckpoint;
//!
//! let nullifiers: Vec<[u8; 32]> = vec![/* ... */];
//!
//! // First sync — full tree scan + catch-up
//! let result = sdk.sync_nullifiers(&nullifiers, None, None).await?;
//! let checkpoint = NullifierSyncCheckpoint {
//!     height: result.new_sync_height,
//!     timestamp: result.new_sync_timestamp,
//! };
//!
//! // Subsequent sync — incremental only (unless too old per full_rescan_after_time_s)
//! let result = sdk.sync_nullifiers(&nullifiers, None, Some(checkpoint)).await?;
//! ```

mod provider;
mod types;

pub use provider::NullifierProvider;
pub use types::{
    NullifierKey, NullifierPoolConfig, NullifierSyncCheckpoint, NullifierSyncConfig,
    NullifierSyncMetrics, NullifierSyncResult,
};

use crate::error::Error;
use crate::platform::trunk_branch_sync::{
    self, BranchQueryParams, KeyLeafTracker, TrunkBranchSyncOps, TrunkQueryResponse,
};
use crate::platform::Fetch;
use crate::sync::retry;
use crate::Sdk;
use dapi_grpc::platform::v0::{
    get_nullifiers_branch_state_request, get_nullifiers_branch_state_response,
    get_recent_compacted_nullifier_changes_request, get_recent_nullifier_changes_request,
    GetNullifiersBranchStateRequest, GetRecentCompactedNullifierChangesRequest,
    GetRecentNullifierChangesRequest,
};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::{GroveBranchQueryResult, GroveTrunkQueryResult};
use drive_proof_verifier::types::{
    NullifiersTrunkQuery, NullifiersTrunkState, RecentCompactedNullifierChanges,
    RecentNullifierChanges,
};
use rs_dapi_client::{
    DapiRequest, ExecutionError, ExecutionResponse, InnerInto, IntoInner, RequestSettings,
};
use std::collections::HashSet;
use tracing::{debug, trace};

/// Server limit for compacted nullifier changes per request.
const COMPACTED_BATCH_LIMIT: usize = 25;
/// Server limit for recent nullifier changes per request.
const RECENT_BATCH_LIMIT: usize = 100;

// ── Context type for the shared algorithm ────────────────────────────

/// Mutable context carried through the trunk/branch tree scan for nullifiers.
struct NullifierSyncContext<'a> {
    config: &'a NullifierSyncConfig,
    nullifiers: &'a [NullifierKey],
    result: &'a mut NullifierSyncResult,
}

// SAFETY: All fields are Send when their referents are Send.
// NullifierSyncConfig, NullifierKey ([u8; 32]), and NullifierSyncResult are all Send.
unsafe impl Send for NullifierSyncContext<'_> {}

// ── TrunkBranchSyncOps implementation ────────────────────────────────

/// Marker type for nullifier sync operations.
struct NullifierOps;

impl TrunkBranchSyncOps for NullifierOps {
    type Context<'a> = NullifierSyncContext<'a>;

    type BranchQueryConfig = NullifierPoolConfig;

    async fn execute_trunk_query(
        sdk: &Sdk,
        settings: RequestSettings,
        context: &mut Self::Context<'_>,
    ) -> Result<TrunkQueryResponse, Error> {
        let trunk_query = NullifiersTrunkQuery {
            pool_type: context.config.pool_type,
            pool_identifier: context.config.pool_identifier,
        };

        let (trunk_state, metadata) =
            NullifiersTrunkState::fetch_with_metadata(sdk, trunk_query, Some(settings)).await?;

        context.result.metrics.trunk_queries += 1;

        let trunk_state = trunk_state.ok_or_else(|| {
            Error::InvalidProvedResponse("Nullifier trunk query returned no state".to_string())
        })?;

        context.result.metrics.total_elements_seen += trunk_state.elements.len();

        trace!(
            "Nullifier trunk query returned {} elements, {} leaf_keys",
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
        for key in context.nullifiers {
            let key_vec = key.to_vec();

            if trunk_result.elements.contains_key(&key_vec) {
                // Nullifier found in tree — the note is spent
                context.result.found.insert(*key);
            } else if let Some((leaf_key, info)) = trunk_result.trace_key_to_leaf(&key_vec) {
                // Not in trunk elements, but traces to a leaf subtree
                tracker.add_key(key_vec, leaf_key, info);
            } else {
                // Proven absent — the note is unspent
                context.result.absent.insert(*key);
            }
        }
        Ok(())
    }

    fn branch_query_config(context: &Self::Context<'_>) -> Self::BranchQueryConfig {
        NullifierPoolConfig {
            pool_type: context.config.pool_type,
            pool_identifier: context.config.pool_identifier,
        }
    }

    async fn execute_single_branch_query(
        sdk: &Sdk,
        config: &Self::BranchQueryConfig,
        params: BranchQueryParams,
        settings: RequestSettings,
        platform_version: &PlatformVersion,
    ) -> Result<GroveBranchQueryResult, Error> {
        execute_nullifier_branch_query(sdk, config, params, settings, platform_version).await
    }

    fn process_branch_result(
        branch_result: &GroveBranchQueryResult,
        queried_leaf_key: &[u8],
        context: &mut Self::Context<'_>,
        tracker: &mut KeyLeafTracker,
    ) -> Result<(), Error> {
        let target_keys = tracker.keys_for_leaf(queried_leaf_key);

        for target_key in target_keys {
            if branch_result.elements.contains_key(&target_key) {
                // Nullifier found — note is spent
                if let Ok(nf) = <[u8; 32]>::try_from(target_key.as_slice()) {
                    context.result.found.insert(nf);
                }
                tracker.key_found(&target_key);
            } else if let Some((new_leaf_key, info)) = branch_result.trace_key_to_leaf(&target_key)
            {
                // Traces to a deeper leaf — need another iteration
                tracker.update_leaf(&target_key, new_leaf_key, info);
            } else {
                // Proven absent — note is unspent
                if let Ok(nf) = <[u8; 32]>::try_from(target_key.as_slice()) {
                    context.result.absent.insert(nf);
                }
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
                .shielded
                .nullifiers_query_min_depth,
            platform_version
                .drive
                .methods
                .shielded
                .nullifiers_query_max_depth,
        )
    }

    // No post-iteration hook needed for nullifiers (no gap limit).

    fn on_branch_query(context: &mut Self::Context<'_>) {
        context.result.metrics.branch_queries += 1;
    }

    fn on_branch_failure(context: &mut Self::Context<'_>) {
        context.result.metrics.branch_query_failures += 1;
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

/// Synchronize nullifier statuses using trunk/branch chunk queries with
/// incremental block-based catch-up.
///
/// See [module docs](self) for full description of sync modes.
///
/// # Arguments
/// - `sdk`: The SDK instance for making network requests.
/// - `provider`: An implementation of [`NullifierProvider`] that supplies nullifier keys.
/// - `config`: Optional configuration; uses defaults if `None`.
/// - `last_sync`: Optional checkpoint from a previous sync. Pass `None` to
///   always perform a full tree scan. When provided, the function compares
///   `now - checkpoint.timestamp` against
///   [`NullifierSyncConfig::full_rescan_after_time_s`] to decide whether a
///   full tree rescan is needed or incremental-only catch-up suffices.
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
    last_sync: Option<NullifierSyncCheckpoint>,
) -> Result<NullifierSyncResult, Error> {
    let config = config.unwrap_or_default();

    let nullifiers = provider.nullifiers_to_check();

    let mut result = NullifierSyncResult::new();

    if nullifiers.is_empty() {
        return Ok(result);
    }

    // Decide whether to do a full tree scan or incremental-only.
    //
    // Incremental-only is chosen when ALL of these are true:
    //   1. last_sync checkpoint is provided
    //   2. full_rescan_after_time_s > 0
    //   3. elapsed time since last sync < full_rescan_after_time_s
    let needs_full_scan = match last_sync {
        Some(checkpoint) if config.full_rescan_after_time_s > 0 => {
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let elapsed = now_secs.saturating_sub(checkpoint.timestamp);
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
        let start_height = last_sync.map(|c| c.height).unwrap_or(0);
        debug!(
            "Nullifier sync: incremental-only from height {}",
            start_height
        );
        start_height
    } else {
        // Full tree scan via the shared algorithm
        let mut context = NullifierSyncContext {
            config: &config,
            nullifiers: &nullifiers,
            result: &mut result,
        };
        let (scan_height, block_time_ms) = trunk_branch_sync::run_full_tree_scan::<NullifierOps>(
            sdk,
            config.min_privacy_count,
            config.max_iterations,
            config.max_concurrent_requests,
            config.request_settings,
            &mut context,
        )
        .await?;
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

// ── Incremental catch-up (nullifier-specific) ────────────────────────

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
                    result.absent.remove(nf_bytes);
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
                    result.absent.remove(nf_bytes);
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

// ── Branch query helper (nullifier-specific) ─────────────────────────

/// Execute a single nullifier branch query with retry logic.
async fn execute_nullifier_branch_query(
    sdk: &Sdk,
    pool_config: &NullifierPoolConfig,
    params: BranchQueryParams,
    settings: RequestSettings,
    platform_version: &PlatformVersion,
) -> Result<GroveBranchQueryResult, Error> {
    let pool_type = pool_config.pool_type;
    let pool_id_owned = pool_config.pool_identifier.map(|p| p.to_vec());
    let BranchQueryParams {
        key,
        depth,
        expected_hash,
        checkpoint_height,
    } = params;

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

// ── SDK integration ──────────────────────────────────────────────────

impl Sdk {
    /// Synchronize nullifier statuses with incremental catch-up support.
    ///
    /// This is the main entry point for nullifier synchronization. It handles
    /// both full tree scans and incremental block-based catch-up, depending on
    /// the parameters.
    ///
    /// On subsequent calls, construct a [`NullifierSyncCheckpoint`] from the
    /// previous [`NullifierSyncResult::new_sync_height`] and
    /// [`NullifierSyncResult::new_sync_timestamp`] so the function can decide
    /// whether a full tree rescan is needed or incremental-only catch-up
    /// suffices.
    ///
    /// # Arguments
    /// - `provider`: An implementation of [`NullifierProvider`] that supplies nullifier keys.
    /// - `config`: Optional configuration; uses defaults if `None`.
    /// - `last_sync`: Optional checkpoint from the previous sync.
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
    /// use dash_sdk::platform::nullifier_sync::NullifierSyncCheckpoint;
    ///
    /// let sdk = Sdk::new(/* ... */);
    /// let nullifiers: Vec<[u8; 32]> = vec![/* known nullifiers */];
    ///
    /// // First call — full scan
    /// let result = sdk.sync_nullifiers(&nullifiers, None, None).await?;
    /// let checkpoint = NullifierSyncCheckpoint {
    ///     height: result.new_sync_height,
    ///     timestamp: result.new_sync_timestamp,
    /// };
    ///
    /// // Next call — incremental only if within threshold
    /// let result = sdk.sync_nullifiers(&nullifiers, None, Some(checkpoint)).await?;
    /// ```
    pub async fn sync_nullifiers<P: NullifierProvider>(
        &self,
        provider: &P,
        config: Option<NullifierSyncConfig>,
        last_sync: Option<NullifierSyncCheckpoint>,
    ) -> Result<NullifierSyncResult, Error> {
        sync_nullifiers(self, provider, config, last_sync).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

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
