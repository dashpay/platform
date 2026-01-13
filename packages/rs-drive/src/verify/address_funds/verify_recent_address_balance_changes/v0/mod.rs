use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;

/// The subtree key for address balances storage as u8
const ADDRESS_BALANCES_KEY_U8: u8 = b'm';
use dpp::balances::credits::CreditOperation;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

use super::VerifiedAddressBalanceChangesPerBlock;

impl Drive {
    /// Verifies recent address balance changes proof.
    ///
    /// Uses the same query as the prove function: a simple range query
    /// starting from start_block_height.
    pub(super) fn verify_recent_address_balance_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedAddressBalanceChangesPerBlock), Error> {
        let path = vec![
            vec![RootTree::SavedBlockTransactions as u8],
            vec![ADDRESS_BALANCES_KEY_U8],
        ];

        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        // Create the same range query as the prove function
        let mut query = Query::new();
        query.insert_range_from(start_block_height.to_be_bytes().to_vec()..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let mut address_balance_changes = Vec::new();

        for (_path, key, maybe_element) in proved_key_values {
            let Some(element) = maybe_element else {
                continue;
            };

            // Parse block height from key (8 bytes, big-endian)
            let height_bytes: [u8; 8] = key.try_into().map_err(|_| {
                Error::Proof(ProofError::CorruptedProof(
                    "invalid block height key length".to_string(),
                ))
            })?;
            let block_height = u64::from_be_bytes(height_bytes);

            // Get the serialized data from the ItemWithSumItem element
            let Element::ItemWithSumItem(serialized_data, _, _) = element else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "expected item with sum item element for address balances".to_string(),
                )));
            };

            // Deserialize the address balance map
            let (address_balances, _): (BTreeMap<PlatformAddress, CreditOperation>, usize) =
                bincode::decode_from_slice(&serialized_data, config).map_err(|e| {
                    Error::Proof(ProofError::CorruptedProof(format!(
                        "cannot decode address balances: {}",
                        e
                    )))
                })?;

            address_balance_changes.push((block_height, address_balances));
        }

        Ok((root_hash, address_balance_changes))
    }
}
