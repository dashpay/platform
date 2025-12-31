//! Address balance synchronization using trunk/branch chunk queries.
//!
//! This module provides privacy-preserving address balance synchronization for wallets.
//! It uses an iterative query process that fetches chunks of the address tree rather than
//! individual addresses, providing privacy by making it unclear which specific addresses
//! are being queried.
//!
//! # Query Strategy
//!
//! 1. **Trunk Query**: Gets the top N levels of the address tree, returning
//!    elements (addresses with balances) and leaf keys with their expected hashes.
//!
//! 2. **Branch Queries**: For keys not found in trunk, trace through the tree
//!    structure to find which leaf subtree contains each target key, then query
//!    only those specific branches.
//!
//! # Privacy Considerations
//!
//! The module ensures that result sets are sufficiently large to provide privacy.
//! When a subtree is too small (fewer elements than `min_privacy_count`), the query
//! is redirected to an ancestor subtree with sufficient elements.
//!
//! # Example
//!
//! ```rust,ignore
//! use dash_sdk::{Sdk, platform::address_sync::{AddressProvider, sync_address_balances}};
//!
//! // Implement AddressProvider for your wallet type
//! struct MyWallet { /* ... */ }
//! impl AddressProvider for MyWallet { /* ... */ }
//!
//! let mut wallet = MyWallet::new();
//! let result = sdk.sync_address_balances(&mut wallet, None).await?;
//!
//! println!("Found {} addresses with balances", result.found.len());
//! println!("Proved {} addresses absent", result.absent.len());
//! ```

mod provider;
mod tracker;
mod types;

pub use provider::AddressProvider;
pub use types::{
    AddressIndex, AddressKey, AddressSyncConfig, AddressSyncMetrics, AddressSyncResult,
    LeafBoundaryKey,
};

use crate::error::Error;
use crate::Sdk;
use dapi_grpc::platform::v0::{
    get_addresses_branch_state_request, get_addresses_branch_state_response,
    get_addresses_trunk_state_request, GetAddressesBranchStateRequest,
    GetAddressesTrunkStateRequest,
};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::{
    calculate_max_tree_depth_from_count, Element, GroveBranchQueryResult, GroveTrunkQueryResult,
    LeafInfo,
};
use futures::stream::{FuturesUnordered, StreamExt};
use rs_dapi_client::{DapiRequest, IntoInner, RequestSettings};
use std::collections::{BTreeSet, HashMap};
use tracing::{debug, trace, warn};
use tracker::KeyLeafTracker;

/// Synchronize address balances using trunk/branch chunk queries.
///
/// This function discovers address balances for addresses supplied by the provider,
/// using privacy-preserving chunk queries. It supports HD wallet gap limit behavior
/// where finding a used address extends the search range.
///
/// # Arguments
/// - `sdk`: The SDK instance for making network requests.
/// - `provider`: An implementation of [`AddressProvider`] that supplies addresses.
/// - `config`: Optional configuration; uses defaults if `None`.
///
/// # Returns
/// - `Ok(AddressSyncResult)`: Contains found addresses with balances and absent addresses.
/// - `Err(Error)`: If the sync fails after exhausting retries.
///
/// # Example
///
/// ```rust,ignore
/// use dash_sdk::platform::address_sync::{sync_address_balances, AddressSyncConfig};
///
/// let config = AddressSyncConfig {
///     min_privacy_count: 64,  // Require larger result sets for more privacy
///     ..Default::default()
/// };
/// let result = sync_address_balances(&sdk, &mut wallet, Some(config)).await?;
/// ```
pub async fn sync_address_balances<P: AddressProvider>(
    sdk: &Sdk,
    provider: &mut P,
    config: Option<AddressSyncConfig>,
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

    // Step 1: Execute trunk query
    let (trunk_result, checkpoint_height) = execute_trunk_query(sdk, &mut result.metrics).await?;

    trace!(
        "Trunk query returned {} elements, {} leaf_keys",
        trunk_result.elements.len(),
        trunk_result.leaf_keys.len()
    );

    // Step 2: Process trunk result
    let mut tracker = KeyLeafTracker::new();
    process_trunk_result(&trunk_result, provider, &mut result, &mut tracker);

    // Step 3: Iterative branch queries
    let mut iterations = 0;
    while !tracker.is_empty() && iterations < config.max_iterations {
        iterations += 1;
        result.metrics.iterations = iterations;

        // Get leaves that need querying (apply privacy adjustment)
        let leaves_to_query =
            get_privacy_adjusted_leaves(&tracker, &trunk_result, config.min_privacy_count);

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
            platform_version,
        )
        .await?;

        // Process branch results
        for (leaf_key, branch_result) in branch_results {
            process_branch_result(
                &branch_result,
                &leaf_key,
                provider,
                &key_to_index,
                &mut result,
                &mut tracker,
            );
        }

        // Check if provider has extended pending addresses (gap limit behavior)
        for (index, key) in provider.pending_addresses() {
            if !key_to_index.contains_key(&key) {
                key_to_index.insert(key.clone(), index);
                // New key needs to be traced - it will be picked up in next iteration
                // For now, trace it using the trunk result if possible
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

    // Set highest found index from provider
    result.highest_found_index = provider.highest_found_index();

    Ok(result)
}

/// Execute the trunk query and return the verified result.
async fn execute_trunk_query(
    sdk: &Sdk,
    metrics: &mut AddressSyncMetrics,
) -> Result<(GroveTrunkQueryResult, u64), Error> {
    let request = GetAddressesTrunkStateRequest {
        version: Some(get_addresses_trunk_state_request::Version::V0(
            get_addresses_trunk_state_request::GetAddressesTrunkStateRequestV0 {},
        )),
    };

    let response: dapi_grpc::platform::v0::GetAddressesTrunkStateResponse = request
        .execute(sdk, RequestSettings::default())
        .await?
        .into_inner();

    metrics.trunk_queries += 1;

    // Parse and verify the proof using the standard SDK pattern
    let (trunk_state, metadata) = sdk
        .parse_proof_with_metadata::<GetAddressesTrunkStateRequest, GroveTrunkQueryResult>(
            request, response,
        )
        .await?;

    let trunk_state = trunk_state.ok_or_else(|| {
        Error::Protocol(dpp::ProtocolError::CorruptedCodeExecution(
            "Trunk query returned no state".to_string(),
        ))
    })?;

    metrics.total_elements_seen += trunk_state.elements.len();

    Ok((trunk_state, metadata.height))
}

/// Process the trunk query result.
fn process_trunk_result<P: AddressProvider>(
    trunk_result: &GroveTrunkQueryResult,
    provider: &mut P,
    result: &mut AddressSyncResult,
    tracker: &mut KeyLeafTracker,
) {
    // Get pending addresses
    let pending: Vec<(AddressIndex, AddressKey)> = provider.pending_addresses();

    for (index, key) in pending {
        // Check if found in elements
        if let Some(element) = trunk_result.elements.get(&key) {
            let balance = extract_balance_from_element(element);
            result.found.insert((index, key.clone()), balance);
            provider.on_address_found(index, &key, balance);
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
}

/// Get privacy-adjusted leaves to query.
///
/// For leaves with count below min_privacy_count, find an ancestor with sufficient count.
fn get_privacy_adjusted_leaves(
    tracker: &KeyLeafTracker,
    trunk_result: &GroveTrunkQueryResult,
    min_privacy_count: u64,
) -> Vec<(LeafBoundaryKey, LeafInfo, u8)> {
    let active_leaves = tracker.active_leaves();
    let mut result = Vec::new();
    let mut seen_ancestors: BTreeSet<LeafBoundaryKey> = BTreeSet::new();

    for (leaf_key, info) in active_leaves {
        let count = info.count.unwrap_or(0);
        let tree_depth = calculate_max_tree_depth_from_count(count);

        if count >= min_privacy_count {
            // Leaf has sufficient privacy, use it directly
            if seen_ancestors.insert(leaf_key.clone()) {
                result.push((leaf_key, info, tree_depth));
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
                    // Reduce depth based on how many levels up we went
                    let depth = tree_depth.saturating_sub(levels_up);
                    result.push((ancestor_key, ancestor_info, depth.max(1)));
                }
            } else {
                // No suitable ancestor found, use the leaf anyway
                if seen_ancestors.insert(leaf_key.clone()) {
                    result.push((leaf_key, info, tree_depth));
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

/// Execute a single branch query.
async fn execute_single_branch_query(
    sdk: &Sdk,
    key: LeafBoundaryKey,
    depth: u32,
    expected_hash: [u8; 32],
    checkpoint_height: u64,
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

    let response: dapi_grpc::platform::v0::GetAddressesBranchStateResponse = request
        .execute(sdk, RequestSettings::default())
        .await?
        .into_inner();

    // Extract merk proof
    let proof_bytes = match response.version {
        Some(get_addresses_branch_state_response::Version::V0(v0)) => v0.merk_proof,
        None => {
            return Err(Error::Protocol(dpp::ProtocolError::CorruptedCodeExecution(
                "Missing version in branch response".to_string(),
            )));
        }
    };

    // Verify the proof
    let branch_result = Drive::verify_address_funds_branch_query(
        &proof_bytes,
        key,
        depth as u8,
        expected_hash,
        platform_version,
    )?;

    Ok(branch_result)
}

/// Process a branch query result.
fn process_branch_result<P: AddressProvider>(
    branch_result: &GroveBranchQueryResult,
    queried_leaf_key: &[u8],
    provider: &mut P,
    key_to_index: &HashMap<AddressKey, AddressIndex>,
    result: &mut AddressSyncResult,
    tracker: &mut KeyLeafTracker,
) {
    // Get all target keys that were in this leaf's subtree
    let target_keys = tracker.keys_for_leaf(queried_leaf_key);

    for target_key in target_keys {
        let index = key_to_index.get(&target_key).copied().unwrap_or(0);

        // Check if found in elements
        if let Some(element) = branch_result.elements.get(&target_key) {
            let balance = extract_balance_from_element(element);
            result.found.insert((index, target_key.clone()), balance);
            provider.on_address_found(index, &target_key, balance);
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
}

/// Extract balance from a GroveDB Element.
///
/// The address funds tree stores balances as sum items.
fn extract_balance_from_element(element: &Element) -> u64 {
    match element {
        Element::SumItem(value, _) => *value as u64,
        _ => 0,
    }
}

// SDK integration impl
impl Sdk {
    /// Synchronize address balances using privacy-preserving trunk/branch chunk queries.
    ///
    /// This method discovers address balances for addresses supplied by the provider,
    /// using an iterative query process that fetches chunks of the address tree rather
    /// than individual addresses. This provides privacy by making it unclear which
    /// specific addresses are being queried.
    ///
    /// # Arguments
    /// - `provider`: An implementation of [`AddressProvider`] that supplies addresses
    ///   and handles callbacks when addresses are found or proven absent.
    /// - `config`: Optional configuration; uses defaults if `None`.
    ///
    /// # Returns
    /// - `Ok(AddressSyncResult)`: Contains found addresses with balances and absent addresses.
    /// - `Err(Error)`: If the sync fails after exhausting retries.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use dash_sdk::{Sdk, platform::address_sync::{AddressProvider, AddressSyncConfig}};
    ///
    /// // Implement AddressProvider for your wallet type
    /// struct MyWallet { /* ... */ }
    /// impl AddressProvider for MyWallet { /* ... */ }
    ///
    /// let sdk = Sdk::new(/* ... */);
    /// let mut wallet = MyWallet::new();
    ///
    /// let config = AddressSyncConfig {
    ///     min_privacy_count: 64,
    ///     ..Default::default()
    /// };
    ///
    /// let result = sdk.sync_address_balances(&mut wallet, Some(config)).await?;
    /// println!("Found {} addresses with {} total balance",
    ///     result.found.len(),
    ///     result.total_balance());
    /// ```
    pub async fn sync_address_balances<P: AddressProvider>(
        &self,
        provider: &mut P,
        config: Option<AddressSyncConfig>,
    ) -> Result<AddressSyncResult, Error> {
        sync_address_balances(self, provider, config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_balance() {
        let sum_item = Element::SumItem(1000, None);
        assert_eq!(extract_balance_from_element(&sum_item), 1000);

        let item = Element::Item(vec![1, 2, 3], None);
        assert_eq!(extract_balance_from_element(&item), 0);
    }
}
