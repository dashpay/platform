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

    /// Verifies recent address balance changes proof using exclusive start (RangeAfter).
    ///
    /// Uses the same query as `prove_recent_address_balance_changes_after`:
    /// a `RangeAfter` query starting after `after_block_height`.
    pub(super) fn verify_recent_address_balance_changes_after_v0(
        proof: &[u8],
        after_block_height: u64,
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

        // Create the same exclusive range query as the prove_after function
        let mut query = Query::new();
        query.insert_range_after(after_block_height.to_be_bytes().to_vec()..);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::CreditOperation;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_and_verify_recent_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address_1 = PlatformAddress::P2pkh([10; 20]);
        let address_2 = PlatformAddress::P2sh([11; 20]);

        // Store balance changes for block 100
        let mut block_100_changes = BTreeMap::new();
        block_100_changes.insert(address_1, CreditOperation::SetCredits(500_000));
        block_100_changes.insert(address_2, CreditOperation::AddToCredits(200_000));

        drive
            .store_address_balances_for_block(
                &block_100_changes,
                100,
                100_000,
                None,
                platform_version,
            )
            .expect("should store block 100 balances");

        // Store balance changes for block 101
        let mut block_101_changes = BTreeMap::new();
        block_101_changes.insert(address_1, CreditOperation::AddToCredits(100_000));

        drive
            .store_address_balances_for_block(
                &block_101_changes,
                101,
                101_000,
                None,
                platform_version,
            )
            .expect("should store block 101 balances");

        // Prove from block 100
        let proof = drive
            .prove_recent_address_balance_changes(100, None, None, platform_version)
            .expect("should prove recent address balance changes");

        // Verify the proof
        let (root_hash, changes) = Drive::verify_recent_address_balance_changes(
            proof.as_slice(),
            100,
            None,
            false,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(changes.len(), 2, "should have 2 blocks of changes");

        // Check block 100
        let (height, balances) = &changes[0];
        assert_eq!(*height, 100);
        assert_eq!(balances.len(), 2);
        assert_eq!(
            balances.get(&address_1),
            Some(&CreditOperation::SetCredits(500_000))
        );
        assert_eq!(
            balances.get(&address_2),
            Some(&CreditOperation::AddToCredits(200_000))
        );

        // Check block 101
        let (height, balances) = &changes[1];
        assert_eq!(*height, 101);
        assert_eq!(balances.len(), 1);
        assert_eq!(
            balances.get(&address_1),
            Some(&CreditOperation::AddToCredits(100_000))
        );
    }

    #[test]
    fn should_prove_and_verify_empty_recent_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Prove from block 100 with no data
        let proof = drive
            .prove_recent_address_balance_changes(100, None, None, platform_version)
            .expect("should prove empty recent address balance changes");

        // Verify the proof
        let (root_hash, changes) = Drive::verify_recent_address_balance_changes(
            proof.as_slice(),
            100,
            None,
            false,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(changes.is_empty(), "should have no changes");
    }

    #[test]
    fn should_prove_and_verify_recent_address_balance_changes_with_limit() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address = PlatformAddress::P2pkh([10; 20]);

        // Store 3 blocks of changes
        for block_height in 100u64..103 {
            let mut changes = BTreeMap::new();
            changes.insert(address, CreditOperation::AddToCredits(block_height * 1000));
            drive
                .store_address_balances_for_block(
                    &changes,
                    block_height,
                    block_height * 1000,
                    None,
                    platform_version,
                )
                .expect("should store balances");
        }

        // Prove from block 100 with limit 2
        let proof = drive
            .prove_recent_address_balance_changes(100, Some(2), None, platform_version)
            .expect("should prove with limit");

        // Verify the proof
        let (root_hash, changes) = Drive::verify_recent_address_balance_changes(
            proof.as_slice(),
            100,
            Some(2),
            false,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(
            changes.len(),
            2,
            "should have 2 blocks of changes (limited)"
        );
        assert_eq!(changes[0].0, 100);
        assert_eq!(changes[1].0, 101);
    }
}
