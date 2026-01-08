use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;

/// The subtree key for compacted address balances storage as u8
const COMPACTED_ADDRESS_BALANCES_KEY_U8: u8 = b'c';
use dpp::balances::credits::CreditOperation;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

use super::VerifiedCompactedAddressBalanceChanges;

impl Drive {
    /// Verifies compacted address balance changes proof.
    ///
    /// Verification is done in two steps:
    /// 1. First verify as SUBSET to examine what's in the proof and determine
    ///    what actual_start_block was used when generating the proof.
    /// 2. Then verify the main ascending query (not as subset) using the exact
    ///    same query that was used for proving.
    pub(super) fn verify_compacted_address_balance_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedCompactedAddressBalanceChanges), Error> {
        let path = vec![
            vec![RootTree::SavedBlockTransactions as u8],
            vec![COMPACTED_ADDRESS_BALANCES_KEY_U8],
        ];

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        // Step 1: Use subset query with insert_all() to examine everything in the proof
        // This allows us to find entries that might start before start_block_height
        // but contain it (e.g., range 100-200 when querying from block 150)
        let mut subset_query = Query::new();
        subset_query.insert_all();

        let subset_path_query =
            PathQuery::new(path.clone(), SizedQuery::new(subset_query, None, None));

        // Use verify_subset_query to look at what's in the proof
        let (root_hash, subset_results) = GroveDb::verify_subset_query(
            proof,
            &subset_path_query,
            &platform_version.drive.grove_version,
        )?;

        // Collect results into a BTreeMap to ensure proper ordering by key
        let results_map: BTreeMap<Vec<u8>, Element> = subset_results
            .into_iter()
            .filter_map(|(_, key, maybe_element)| maybe_element.map(|element| (key, element)))
            .collect();

        // Get the first entry and check if it's a containing range
        // Only use the key from the proof if it contains start_block_height
        // (start <= start_block_height <= end)
        // Otherwise fall back to (start_block_height, start_block_height)
        let start_key = results_map.first_key_value().and_then(|(key, _)| {
            if key.len() != 16 {
                return None;
            }
            let start_block = u64::from_be_bytes(key[0..8].try_into().unwrap());
            let end_block = u64::from_be_bytes(key[8..16].try_into().unwrap());

            // Only return the key if it's a containing range
            if start_block <= start_block_height && start_block_height <= end_block {
                Some(key.clone())
            } else {
                None
            }
        });

        // Step 2: Verify the proof using the start_key discovered from the proof
        // The smallest key in the proof is what the prove function used as its starting point
        // If no entries exist, fall back to (start_block_height, start_block_height)
        let start_key = start_key.unwrap_or_else(|| {
            let mut key = Vec::with_capacity(16);
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key
        });

        let mut query = Query::new();
        query.insert_range_from(start_key..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));
        let (verified_root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        // Both verifications must have the same root hash
        if root_hash != verified_root_hash {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "root hash mismatch between subset and main query verification".to_string(),
            )));
        }

        let mut compacted_changes = Vec::new();

        for (_path, key, maybe_element) in proved_key_values {
            let Some(element) = maybe_element else {
                continue;
            };

            // Parse start_block and end_block from key (16 bytes total, both big-endian)
            if key.len() != 16 {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "invalid compacted block key length, expected 16 bytes".to_string(),
                )));
            }

            let start_block = u64::from_be_bytes(key[0..8].try_into().unwrap());
            let end_block = u64::from_be_bytes(key[8..16].try_into().unwrap());

            // Get the serialized data from the Item element
            let Element::Item(serialized_data, _) = element else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "expected item element for compacted address balances".to_string(),
                )));
            };

            // Deserialize the address balance map
            let (address_balances, _): (BTreeMap<PlatformAddress, CreditOperation>, usize) =
                bincode::decode_from_slice(&serialized_data, config).map_err(|e| {
                    Error::Proof(ProofError::CorruptedProof(format!(
                        "cannot decode compacted address balances: {}",
                        e
                    )))
                })?;

            compacted_changes.push((start_block, end_block, address_balances));
        }

        Ok((root_hash, compacted_changes))
    }
}
