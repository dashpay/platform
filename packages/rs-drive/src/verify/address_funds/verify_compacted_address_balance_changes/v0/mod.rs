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
    pub(super) fn verify_compacted_address_balance_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedCompactedAddressBalanceChanges), Error> {
        let path = vec![
            vec![RootTree::SavedBlockTransactions as u8],
            vec![COMPACTED_ADDRESS_BALANCES_KEY_U8],
        ];

        // Create a range query starting from the specified height
        // Keys are 16 bytes: (start_block, end_block), both big-endian
        // We query from (start_block_height, 0) onwards
        let mut start_key = Vec::with_capacity(16);
        start_key.extend_from_slice(&start_block_height.to_be_bytes());
        start_key.extend_from_slice(&0u64.to_be_bytes());

        let mut query = Query::new();
        query.insert_range_from(start_key..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

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
